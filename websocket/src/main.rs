use std::collections::HashMap;
use actix::Addr;
use crate::actors::relay::RelayActor;
use crate::actors::ws::WsConn;

pub mod actors;

pub struct AppState {
    relays: HashMap<u32, Addr<RelayActor>>,
}

#[actix_web::get("/")]
async fn websocket(
    req: actix_web::HttpRequest,
    stream: actix_web::web::Payload,
    username: actix_web::web::Path<String>,
    state: actix_web::web::Data<AppState>,
) -> Result<actix_web::HttpResponse, actix_web::Error> {
    let conn = WsConn::new(username.into_inner(), state.relays.get(&0u32).unwrap().clone());
    actix_web_actors::ws::start(conn, &req, stream)
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let relay = RelayActor::new(1);
    
    actix_web::HttpServer::new(|| actix_web::App::new().service(websocket))
        .bind(("127.0.0.1", 9002))?
        .run()
        .await
}
