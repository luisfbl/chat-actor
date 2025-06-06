use std::env;
use std::sync::Arc;
use actix::{Actor};
use actix_web::{web, App, HttpServer, HttpResponse};
use serde_json::json;
use log::{info, debug, warn, error};
use sysinfo::{System};
use tokio::sync::Mutex;
use crate::actors::relay::RelayActor;
use crate::actors::ws::WsConn;
use crate::load_balancer::{LoadBalancer, PodMetrics};
use crate::dynamic_relay_balancer::{DynamicRelayBalancer, RelayMetrics};

pub mod actors;
pub mod load_balancer;
pub mod dynamic_relay_balancer;
pub mod redis_cluster;

pub struct AppState {
    relay_balancer: DynamicRelayBalancer,
    load_balancer: LoadBalancer,
    pod_id: String,
    system: Arc<Mutex<System>>,
}

impl AppState {
    async fn new() -> Self {
        info!("Iniciando configuração do AppState");
        
        let relay_count: u32 = env::var("RELAY_COUNT")
            .unwrap_or_else(|_| "3".to_string())
            .parse()
            .unwrap_or(3);
        debug!("RELAY_COUNT configurado para: {}", relay_count);

        let relay_start_id: u32 = env::var("RELAY_START_ID")
            .unwrap_or_else(|_| "1".to_string())
            .parse()
            .unwrap_or(1);
        debug!("RELAY_START_ID configurado para: {}", relay_start_id);

        let max_connections_per_relay: usize = env::var("MAX_CONNECTIONS_PER_RELAY")
            .unwrap_or_else(|_| "800".to_string())
            .parse()
            .unwrap_or(800);
        debug!("MAX_CONNECTIONS_PER_RELAY configurado para: {}", max_connections_per_relay);

        let pod_id = env::var("POD_NAME")
            .unwrap_or_else(|_| format!("pod-{}", std::process::id()));
        info!("Pod ID: {}", pod_id);

        info!("Criando DynamicRelayBalancer e LoadBalancer");
        let relay_balancer = DynamicRelayBalancer::new(max_connections_per_relay);
        let load_balancer = LoadBalancer::new();
        
        info!("Iniciando {} relays", relay_count);
        for i in 0..relay_count {
            let relay_id = relay_start_id + i;
            debug!("Tentando iniciar relay {}", relay_id);

            match RelayActor::new(relay_id).await {
                Ok(relay_actor) => {
                    let relay_addr = relay_actor.start();
                    relay_balancer.add_relay(relay_id, relay_addr).await;
                    info!("Pod {}: Relay {} iniciado com sucesso e conectado ao Redis Cluster", pod_id, relay_id);
                }
                Err(e) => {
                    error!("Falha ao iniciar relay {}: {}", relay_id, e);
                }
            }
        }

        info!("Inicializando sistema de monitoramento sysinfo");
        let system = Arc::new(Mutex::new(System::new_all()));
        
        info!("AppState configurado com sucesso");
        AppState {
            relay_balancer,
            load_balancer,
            pod_id,
            system,
        }
    }

    async fn get_cpu_usage(system: &Arc<Mutex<System>>) -> f64 {
        let mut sys = system.lock().await;
        sys.refresh_all();
        
        let cpu_usage = sys.global_cpu_usage();
        debug!("CPU usage real: {:.2}%", cpu_usage);
        cpu_usage as f64
    }

    async fn get_memory_usage(system: &Arc<Mutex<System>>) -> f64 {
        let mut sys = system.lock().await;
        sys.refresh_memory();
        
        let total_memory = sys.total_memory();
        let used_memory = sys.used_memory();
        let memory_usage = (used_memory as f64 / total_memory as f64) * 100.0;
        
        debug!("Memory usage real: {:.2}% ({} MB usados de {} MB totais)", 
               memory_usage, used_memory / 1024 / 1024, total_memory / 1024 / 1024);
        memory_usage
    }

    async fn start_metrics_updater(&self) {
        info!("Iniciando sistema de atualização de métricas");
        let load_balancer = self.load_balancer.clone();
        let relay_balancer = self.relay_balancer.clone();
        let pod_id = self.pod_id.clone();
        let system = self.system.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(10));
            info!("Metrics updater configurado para executar a cada 10 segundos");

            loop {
                interval.tick().await;
                debug!("Executando ciclo de atualização de métricas");
                
                let relay_stats = relay_balancer.get_relay_stats().await;
                let total_connections: usize = relay_stats.values()
                    .map(|r| r.active_connections)
                    .sum();
                debug!("Total de conexões ativas: {}", total_connections);

                let cpu_usage = Self::get_cpu_usage(&system).await;
                let memory_usage = Self::get_memory_usage(&system).await;
                
                let pod_metrics = PodMetrics {
                    pod_id: pod_id.clone(),
                    active_connections: total_connections,
                    cpu_usage,
                    memory_usage,
                    relay_count: relay_stats.len(),
                    last_updated: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                };

                info!("Atualizando métricas do pod: {} conexões, CPU: {:.2}%, Mem: {:.2}%", 
                     total_connections, cpu_usage, memory_usage);
                load_balancer.update_pod_metrics(pod_metrics).await;
                load_balancer.cleanup_inactive_pods().await;
                
                let rebalances = relay_balancer.rebalance_if_needed().await;
                if !rebalances.is_empty() {
                    warn!("Rebalanceamento necessário: {} usuários precisam ser redistribuídos", rebalances.len());
                }
            }
        });
    }
}

#[actix_web::get("/ws/{username}")]
async fn websocket(
    req: actix_web::HttpRequest,
    stream: web::Payload,
    username: web::Path<String>,
    state: web::Data<AppState>,
) -> Result<actix_web::HttpResponse, actix_web::Error> {
    let username = username.into_inner();
    info!("Nova conexão WebSocket solicitada para usuário: {}", username);
    
    let relay_id = state.relay_balancer.get_best_relay_for_user(&username).await
        .ok_or_else(|| {
            error!("Nenhum relay disponível para usuário: {}", username);
            actix_web::error::ErrorInternalServerError("No relay available")
        })?;
    debug!("Relay {} selecionado para usuário: {}", relay_id, username);

    let relay_addr = state.relay_balancer.get_relay_addr(relay_id).await
        .ok_or_else(|| {
            error!("Relay {} não encontrado para usuário: {}", relay_id, username);
            actix_web::error::ErrorInternalServerError("Relay not found")
        })?;

    info!("Estabelecendo conexão WebSocket: usuário {} -> relay {}", username, relay_id);
    let conn = WsConn::new(username, relay_addr);
    actix_web_actors::ws::start(conn, &req, stream)
}

#[actix_web::get("/health")]
async fn health(state: actix_web::web::Data<AppState>) -> actix_web::HttpResponse {
    debug!("Health check solicitado para pod: {}", state.pod_id);
    let relay_stats = state.relay_balancer.get_relay_stats().await;
    let pod_stats = state.load_balancer.get_pod_stats().await;

    let response = json!({
        "status": "healthy",
        "pod_id": state.pod_id,
        "relays": relay_stats,
        "cluster_pods": pod_stats.len()
    });
    
    debug!("Health check respondido: {} relays ativos, {} pods no cluster", 
           relay_stats.len(), pod_stats.len());
    HttpResponse::Ok().json(response)
}

#[actix_web::get("/metrics")]
async fn metrics(state: actix_web::web::Data<AppState>) -> actix_web::HttpResponse {
    debug!("Métricas solicitadas para pod: {}", state.pod_id);
    let relay_stats = state.relay_balancer.get_relay_stats().await;
    let pod_stats = state.load_balancer.get_pod_stats().await;

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let response = json!({
        "pod_metrics": pod_stats,
        "relay_metrics": relay_stats,
        "timestamp": timestamp
    });
    
    info!("Métricas enviadas: {} pods, {} relays, timestamp: {}", 
          pod_stats.len(), relay_stats.len(), timestamp);
    HttpResponse::Ok().json(response)
}

#[actix_web::get("/relays")]
async fn get_relays(state: actix_web::web::Data<AppState>) -> actix_web::HttpResponse {
    debug!("Informações de relays solicitadas para pod: {}", state.pod_id);
    let relay_stats = state.relay_balancer.get_relay_stats().await;
    let active_relay_ids: Vec<_> = relay_stats.keys().collect();

    let response = json!({
        "active_relays": active_relay_ids,
        "detailed_stats": relay_stats,
        "pod_id": state.pod_id
    });
    
    info!("Informações de relays enviadas: {} relays ativos no pod {}", 
          active_relay_ids.len(), state.pod_id);
    HttpResponse::Ok().json(response)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();
    info!("🚀 Iniciando WebSocket Server");
    
    info!("Criando estado da aplicação...");
    let app_state = web::Data::new(AppState::new().await);

    info!("Iniciando sistema de métricas...");
    app_state.start_metrics_updater().await;

    info!("Configurando servidor HTTP na porta 9002...");
    HttpServer::new(move || {
        info!("Configurando rotas da aplicação");
        App::new()
            .app_data(app_state.clone())
            .service(websocket)
            .service(health)
            .service(get_relays)
            .service(metrics)
    })
        .bind(("0.0.0.0", 9002))?
        .run()
        .await
}