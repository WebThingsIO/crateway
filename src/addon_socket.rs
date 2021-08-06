/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use crate::addon_instance::AddonInstance;
use actix_web::{web, App, Error as ActixError, HttpRequest, HttpResponse, HttpServer};
use actix_web_actors::ws;
use anyhow::Error;
use log::{debug, info};

async fn route(req: HttpRequest, stream: web::Payload) -> Result<HttpResponse, ActixError> {
    debug!("Incoming websocket connection from {:?}", req.peer_addr());
    ws::start(AddonInstance::new(), &req, stream)
}

pub async fn start() -> Result<(), Error> {
    info!("Starting addon socket");

    HttpServer::new(|| App::new().route("/", web::get().to(route)))
        .bind("127.0.0.1:9500")?
        .run()
        .await?;

    Ok(())
}
