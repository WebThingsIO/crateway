/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use crate::config::CONFIG;
use anyhow::{anyhow, Error};
use bytes::BytesMut;
use futures::StreamExt;
use httparse::Request;
use log::debug;
use std::net::SocketAddr;
use tokio::io::AsyncWriteExt;
use tokio::net::{TcpListener, TcpStream};
use tokio_util::codec::Decoder;

#[derive(Debug)]
pub enum RoutingResult {
    Rest,
    Websocket(String),
}

pub struct HttpTunnelCodec;

const MAX_REQUEST_SIZE: usize = 4096;

impl Decoder for HttpTunnelCodec {
    type Item = RoutingResult;
    type Error = Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.len() > MAX_REQUEST_SIZE {
            return Err(anyhow!(
                "Buffer exceeded max size of {} bytes",
                MAX_REQUEST_SIZE
            ));
        }

        let mut headers = [httparse::EMPTY_HEADER; 64];
        let mut req = Request::new(&mut headers);

        let status = req.parse(src).context("Failed to parse incoming request")?;
        if !status.is_complete() {
            return Ok(None);
        }
        if let Some(header) = req.headers.iter().find(|header| header.name == "Upgrade") {
            let value = String::from_utf8(header.value.to_vec())
                .context("Failed to parse upgrade header")?;

            if value == "websocket" {
                let path = req.path.ok_or_else(|| anyhow!("Failed to parse path"))?;
                return Ok(Some(RoutingResult::Websocket(path.to_owned())));
            }
        }
        Ok(Some(RoutingResult::Rest))
    }
}

pub async fn start() -> Result<(), Error> {
    let listener = TcpListener::bind(format!("127.0.0.1:{}", CONFIG.ports.api)).await?;

    loop {
        let (stream, addr) = listener.accept().await?;
        debug!("Incoming tcp connection from {}", addr);

        if let Err(err) = forward_stream(stream, addr).await {
            error!("Failed to forward stream from {}: {}", addr, err);
        }
    }
}

async fn forward_stream(stream: TcpStream, addr: SocketAddr) -> Result<(), Error> {
    let codec = HttpTunnelCodec;
    let mut stream = codec.framed(stream);

    match stream.next().await {
        Some(Ok(result)) => {
            debug!("Type of stream is {:?}", result);

            let consumed_bytes = stream.read_buffer().to_vec();
            let mut stream = stream.into_inner();

            let port = match result {
                RoutingResult::Rest => CONFIG.ports.http,
                RoutingResult::Websocket(_) => CONFIG.ports.websocket,
            };

            let addr = format!("127.0.0.1:{}", port)
                .parse::<SocketAddr>()
                .expect("Failed to parse forward address");

            debug!("Forwarding connection to {}", addr);

            let mut remote_stream = TcpStream::connect(&addr)
                .await
                .map_err(|err| anyhow!("Could not connect to {}: {}", addr, err))?;

            remote_stream
                .write_all(&consumed_bytes[..])
                .await
                .map_err(|err| anyhow!("Failed to write initial data to {}: {}", addr, err))?;

            tokio::spawn(async move {
                match tokio::io::copy_bidirectional(&mut stream, &mut remote_stream).await {
                    Ok((tx, rx)) => {
                        debug!(
                            "Stream closed after {} outgoing bytes and {} incoming bytes",
                            tx + consumed_bytes.len() as u64,
                            rx
                        );
                    }
                    Err(err) => {
                        error!("Failed to forward stream: {}", err)
                    }
                }
            });

            Ok(())
        }
        Some(Err(err)) => Err(anyhow!("Failed to parse incoming request: {}", err)),
        None => Err(anyhow!("Stream of {} is empty", addr)),
    }
}
