use async_process::{Child, Command, Stdio};
use fs_extra::{copy_items, dir::CopyOptions};
use futures::{
    channel::mpsc, io::BufReader, stream::SplitSink, AsyncBufReadExt, AsyncRead, SinkExt, StreamExt,
};
use regex::Regex;
use reqwest::{Client, Method, RequestBuilder, Response, StatusCode};
use rocket::async_trait;
use serde::Serialize;
use serde_json::json;
use std::{env, fs, path::PathBuf};
use tempdir::TempDir;
use tokio::{
    net::TcpStream,
    time::{sleep, Duration},
};
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};

pub struct Dirs {
    home_dir: TempDir,
    ui_dir: PathBuf,
    addons_dir: PathBuf,
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
        println!("Requesting gateway exit");
        std::process::Command::new("curl")
            .arg(format!("{}:{}/exit", self.base_url, self.http_port))
            .output()
            .unwrap();
        println!("Waiting for gateway exit");
        futures::executor::block_on(self.child.status()).expect("Wait for child to exit");
        println!("Gateway exited");
    }
}

impl Gateway {
    pub async fn startup_with_mock_addon() -> (Self, MockAddonConnection) {
        let dirs = create_dirs();
        let mock_addon_source_dir = env::current_dir().unwrap().join("mock-addon");

        println!("Copying mock addon to {:?}", dirs.addons_dir);
        copy_items(
            &[mock_addon_source_dir],
            dirs.addons_dir.clone(),
            &CopyOptions::new(),
        )
        .unwrap();

        println!("Starting gateway");
        let child = start_gateway(&dirs).await;
        let mut gateway = Self::from(child, dirs).await;

        println!("Enabling mock addon");
        gateway.authorize().await;
        let _: (_, String) = gateway
            .put("/addons/mock-addon", json!({"enabled": true}))
            .await;

        (gateway, MockAddonConnection::new().await)
    }

    pub async fn startup() -> Self {
        let dirs = create_dirs();
        let child = start_gateway(&dirs).await;
        Self::from(child, dirs).await
    }

    pub async fn from(mut child: Child, dirs: Dirs) -> Self {
        let (tx, mut rx) = mpsc::unbounded();

        let tx1 = tx.clone();
        let stream = child.stdout.take().expect("Take stdout");
        forward_stream(stream, move |line| {
            if let Some(url) = try_extract_base_url(line) {
                tx1.unbounded_send(url).expect("Send base url");
            }
        });

        let tx2 = tx.clone();
        let stream = child.stderr.take().expect("Take stderr");
        forward_stream(stream, move |line| {
            if let Some(url) = try_extract_base_url(line) {
                tx2.unbounded_send(url).expect("Send base url");
            }
        });

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

    pub async fn delete<U: FromResponseBody>(&self, path: &str) -> (StatusCode, U) {
        RequestBuilder::build_from(self, Method::DELETE, path)
            .add_authorization(self)
            .send_req()
            .await
    }

    pub async fn create_secondary_user(&self) {
        self.post::<serde_json::Value>(
            "/users",
            json!({"email": "foo@bar", "password": "42", "name": "foo"}),
        )
        .await;
    }
}

fn create_dirs() -> Dirs {
    let home_dir = TempDir::new(".webthingsio").expect("Create home dir");

    let ui_dir = home_dir.path().join(".webthingsui");
    fs::create_dir(&ui_dir).expect("Create ui dir");
    fs::write(ui_dir.join("index.html"), "foo").expect("Create index.html");

    let addons_dir = home_dir.path().join("addons");
    fs::create_dir(&addons_dir).expect("Create addons dir");

    Dirs {
        home_dir,
        ui_dir,
        addons_dir,
    }
}

async fn start_gateway(dirs: &Dirs) -> Child {
    #[cfg(not(feature = "debug"))]
    {
        Command::new("cargo")
            .args(&["build", "--features", "debug"])
            .output()
            .await
            .expect("Build gateway for debug");
    }
    Command::new("./target/debug/crateway")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env("WEBTHINGS_HOME", dirs.home_dir().into_os_string())
        .env("WEBTHINGS_UI", dirs.ui_dir().into_os_string())
        .spawn()
        .expect("Start gateway process")
}

fn forward_stream<T: AsyncRead + Unpin + Send + 'static>(
    stream: T,
    f: impl Fn(String) + std::marker::Send + 'static,
) {
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

#[derive(Serialize, Debug)]
pub enum MockAddonMessage {
    CreateMockDevice(webthings_gateway_ipc_types::Device),
}

pub struct MockAddonConnection(
    SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, tungstenite::Message>,
);

impl MockAddonConnection {
    pub async fn new() -> Self {
        loop {
            sleep(Duration::from_millis(100)).await;
            println!("Trying to connect to mock addon");
            match connect_async("ws://localhost:9501").await {
                Ok((socket, _)) => {
                    let (sink, _) = socket.split();
                    return Self(sink);
                }
                Err(err) => eprintln!("Error connecting to mock addon: {:?}", err),
            }
        }
    }

    pub async fn send(&mut self, msg: MockAddonMessage) {
        self.0
            .send(tungstenite::Message::Text(
                serde_json::to_string(&msg).unwrap(),
            ))
            .await
            .unwrap();
    }

    pub async fn create_mock_device(&mut self, description: webthings_gateway_ipc_types::Device) {
        self.send(MockAddonMessage::CreateMockDevice(description))
            .await
    }
}
