use std::collections::HashMap;
use std::env;
use actix::{Actor, Addr};
use crate::actors::relay::RelayActor;
use crate::actors::ws::WsConn;

pub mod actors;

pub struct AppState {
    relays: HashMap<u32, Addr<RelayActor>>,
}

impl AppState {
    async fn new() -> Self {
        let mut relays = HashMap::new();

        let relay_count: u32 = env::var("RELAY_COUNT")
            .unwrap_or_else(|_| "3".to_string())
            .parse()
            .unwrap_or(3);

        let relay_start_id: u32 = env::var("RELAY_START_ID")
            .unwrap_or_else(|_| "1".to_string())
            .parse()
            .unwrap_or(1);

        let pod_name = env::var("POD_NAME").unwrap_or_else(|_| "default".to_string());
        
        for i in 0..relay_count {
            let relay_id = relay_start_id + i;

            match RelayActor::new(relay_id).await {
                Ok(relay_actor) => {
                    let relay_addr = relay_actor.start();
                    relays.insert(relay_id, relay_addr);
                    println!("Pod {}: Iniciado relay {} com Redis", pod_name, relay_id);
                }
                Err(e) => {
                    eprintln!("Erro ao iniciar relay {}: {}", relay_id, e);
                }
            }
        }

        AppState { relays }
    }

    fn get_relay_for_user(&self, username: &str) -> Option<&Addr<RelayActor>> {
        if self.relays.is_empty() {
            return None;
        }
        
        let hash = username.chars().map(|c| c as u32).sum::<u32>();
        let relay_ids: Vec<_> = self.relays.keys().cloned().collect();
        let index = (hash as usize) % relay_ids.len();
        let relay_id = relay_ids[index];

        self.relays.get(&relay_id)
    }
}

#[actix_web::get("/ws/{username}")]
async fn websocket(
    req: actix_web::HttpRequest,
    stream: actix_web::web::Payload,
    username: actix_web::web::Path<String>,
    state: actix_web::web::Data<AppState>,
) -> Result<actix_web::HttpResponse, actix_web::Error> {
    let username = username.into_inner();
    
    let relay_addr = state.get_relay_for_user(&username)
        .ok_or_else(|| actix_web::error::ErrorInternalServerError("No relay available"))?;

    let conn = WsConn::new(username, relay_addr.clone());
    actix_web_actors::ws::start(conn, &req, stream)
}

#[actix_web::get("/health")]
async fn health() -> actix_web::HttpResponse {
    actix_web::HttpResponse::Ok().json(serde_json::json!({
        "status": "healthy"
    }))
}

#[actix_web::get("/relays")]
async fn get_relays(state: actix_web::web::Data<AppState>) -> actix_web::HttpResponse {
    let relay_ids: Vec<u32> = state.relays.keys().cloned().collect();
    actix_web::HttpResponse::Ok().json(serde_json::json!({
        "active_relays": relay_ids,
        "count": relay_ids.len(),
        "pod_id": env::var("POD_NAME").unwrap_or_else(|_| "default".to_string())
    }))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let app_state = actix_web::web::Data::new(AppState::new().await);

    actix_web::HttpServer::new(move || {
        actix_web::App::new()
            .app_data(app_state.clone())
            .service(websocket)
            .service(health)
            .service(get_relays)
    })
        .bind(("0.0.0.0", 9002))?
        .run()
        .await
}