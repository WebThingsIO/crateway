use async_process::{Child, ChildStdout, Command, Stdio};
use futures::{channel::mpsc, io::BufReader, AsyncBufReadExt, StreamExt};
use regex::Regex;
use reqwest::{Client, Method, RequestBuilder, Response, StatusCode};
use rocket::async_trait;
use serde_json::json;
use std::{fs, path::PathBuf};
use tempdir::TempDir;

pub struct Dirs {
    home_dir: TempDir,
    ui_dir: PathBuf,
}

impl Dirs {
    fn home_dir(&self) -> PathBuf {
        self.home_dir.path().to_owned()
    }
    fn ui_dir(&self) -> PathBuf {
        self.ui_dir.to_owned()
    }
}

pub struct Gateway {
    pub dirs: Dirs,
    pub base_url: String,
    pub http_port: u16,
    pub jwt: Option<String>,
    pub child: Child,
}

impl Drop for Gateway {
    fn drop(&mut self) {
        self.child.kill().expect("Kill gateway process")
    }
}

impl Gateway {
    pub async fn startup() -> Self {
        let dirs = create_dirs();
        let mut child = start_gateway(&dirs);

        let (tx, mut rx) = mpsc::unbounded();

        let stream = child.stdout.take().expect("Take stdout");
        forward_stream(stream, move |line| {
            if let Some(url) = try_extract_base_url(line) {
                tx.unbounded_send(url).expect("Send base url");
            }
        })
        .await;

        let base_url = rx.next().await.expect("Receive base url");

        Self {
            dirs,
            base_url,
            http_port: 8081,
            child,
            jwt: None,
        }
    }

    pub async fn authorize(&mut self) {
        let (status, body) = self
            .post::<serde_json::Value>(
                "/users",
                json!({"email": "test@test", "password": "test", "name": "Tester"}),
            )
            .await;
        if !status.is_success() {
            panic!("Failed to login");
        }
        self.jwt = Some(body.get("jwt").unwrap().as_str().unwrap().to_owned());
    }

    pub async fn get<U: FromResponseBody>(&self, path: &str) -> (StatusCode, U) {
        RequestBuilder::build_from(self, Method::GET, path)
            .add_authorization(self)
            .send_req()
            .await
    }

    pub async fn post<U: FromResponseBody>(
        &self,
        path: &str,
        body: serde_json::Value,
    ) -> (StatusCode, U) {
        RequestBuilder::build_from(self, Method::POST, path)
            .add_authorization(self)
            .body(serde_json::to_string(&body).expect("Serialize body"))
            .send_req()
            .await
    }

    pub async fn put<U: FromResponseBody>(
        &self,
        path: &str,
        body: serde_json::Value,
    ) -> (StatusCode, U) {
        RequestBuilder::build_from(self, Method::PUT, path)
            .add_authorization(self)
            .body(serde_json::to_string(&body).expect("Serialize body"))
            .send_req()
            .await
    }
}

fn create_dirs() -> Dirs {
    let home_dir = TempDir::new(".webthingsio").expect("Create home dir");
    let ui_dir = home_dir.path().join(".webthingsui");
    fs::create_dir(&ui_dir).expect("Create ui dir");
    fs::write(ui_dir.join("index.html"), "foo").expect("Create index.html");
    Dirs { home_dir, ui_dir }
}

fn start_gateway(dirs: &Dirs) -> Child {
    Command::new("./target/debug/crateway")
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .env("WEBTHINGS_HOME", dirs.home_dir().into_os_string())
        .env("WEBTHINGS_UI", dirs.ui_dir().into_os_string())
        .spawn()
        .expect("Start gateway process")
}

async fn forward_stream(stream: ChildStdout, f: impl Fn(String) + std::marker::Send + 'static) {
    tokio::spawn(async move {
        let mut lines = BufReader::new(stream).lines();
        while let Some(Ok(line)) = lines.next().await {
            println!("{}", line);
            f(line);
        }
    });
}

fn try_extract_base_url(line: String) -> Option<String> {
    if line.contains("Rocket has launched from") {
        let line = String::from_utf8(
            strip_ansi_escapes::strip(line.as_bytes()).expect("Remove ANSII control characters"),
        )
        .expect("Convert slice to string");

        let re = Regex::new(r"Rocket has launched from ((?:http|https)://[0-9]{1,3}\.[0-9]{1,3}\.[0-9]{1,3}\.[0-9]{1,3}):[0-9]{1,5}").unwrap();
        Some(re.captures(&line).expect("Find url")[1].to_owned())
    } else {
        None
    }
}

#[async_trait]
pub trait FromResponseBody {
    async fn from_response_body(response: Response) -> Self;
}

#[async_trait]
impl FromResponseBody for String {
    async fn from_response_body(response: Response) -> Self {
        response.text().await.unwrap_or_else(|_| "".to_owned())
    }
}

#[async_trait]
impl FromResponseBody for serde_json::Value {
    async fn from_response_body(response: Response) -> Self {
        response.json().await.unwrap_or_else(|_| json!({}))
    }
}

#[async_trait]
pub trait GatewayRequest {
    fn build_from(gateway: &Gateway, method: Method, route: &str) -> Self;
    async fn send_req<T: FromResponseBody>(self) -> (StatusCode, T);
    fn add_authorization(self, gateway: &Gateway) -> Self;
}

#[async_trait]
impl GatewayRequest for RequestBuilder {
    fn build_from(gateway: &Gateway, method: Method, path: &str) -> Self {
        let client = Client::new();
        client.request(
            method,
            format!("{}:{}{}", gateway.base_url, gateway.http_port, path),
        )
    }

    async fn send_req<T: FromResponseBody>(self) -> (StatusCode, T) {
        let response = RequestBuilder::send(self).await.expect("Receive response");

        let status = response.status();
        let body = T::from_response_body(response).await;
        (status, body)
    }

    fn add_authorization(self, gateway: &Gateway) -> RequestBuilder {
        if let Some(jwt) = &gateway.jwt {
            self.bearer_auth(jwt)
        } else {
            self
        }
    }
}
