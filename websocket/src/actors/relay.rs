use crate::actors::ws::WsConn;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use actix::{Actor, Context, Handler, AsyncContext, ActorFutureExt, WrapFuture};
use tokio::sync::mpsc;
use crate::actors::{
    JoinEvent, RegisterConnection, UnRegisterConnection, UserMessage,
    RedisMessage, RedisMessageType, GetMetrics
};
use crate::redis_cluster::RedisClusterManager;

pub struct RelayActor {
    relay_id: u32,
    connections: HashMap<String, actix::Addr<WsConn>>,
    redis_manager: RedisClusterManager,
    redis_receiver: Option<mpsc::UnboundedReceiver<RedisMessage>>,
    last_heartbeat: Instant,
    metrics: RelayMetrics,
}

#[derive(Debug, Clone)]
pub struct RelayMetrics {
    pub active_connections: usize,
    pub message_count: u64,
    pub last_message_time: Instant,
    pub avg_response_time: f64,
}

impl RelayActor {
    pub async fn new(relay_id: u32) -> Result<Self, redis::RedisError> {
        let redis_manager = RedisClusterManager::new()?;

        Ok(Self {
            relay_id,
            connections: HashMap::new(),
            redis_manager,
            redis_receiver: None,
            last_heartbeat: Instant::now(),
            metrics: RelayMetrics {
                active_connections: 0,
                message_count: 0,
                last_message_time: Instant::now(),
                avg_response_time: 0.0,
            },
        })
    }

    fn start_heartbeat(&self, ctx: &mut Context<Self>) {
        ctx.run_interval(Duration::from_secs(15), |act, _ctx| {
            let redis_manager = act.redis_manager.clone();
            let relay_id = act.relay_id;
            let active_connections = act.connections.len();

            let channel = format!("relay_heartbeat_{}", relay_id);
            let fallback_channel = "relay_heartbeat_global";

            let fut = async move {
                if let Err(e) = redis_manager.publish_with_fallback(
                    &channel,
                    fallback_channel,
                    relay_id,
                    RedisMessageType::RelayHeartbeat {
                        relay_id,
                        active_connections,
                    },
                ).await {
                    eprintln!("Erro ao enviar heartbeat via Redis Cluster: {}", e);
                }
            };

            _ctx.spawn(fut.into_actor(act));
            act.last_heartbeat = Instant::now();
        });
    }

    fn start_redis_listener(&mut self, ctx: &mut Context<Self>) {
        let redis_manager = self.redis_manager.clone();
        let relay_id = self.relay_id;

        let fut = async move {
            // Subscribe a múltiplos canais para redundância
            let channels = vec![
                format!("relay_messages_{}", relay_id),
                "relay_messages_global".to_string(),
                format!("relay_events_{}", relay_id),
            ];

            // Usar o primeiro canal disponível
            for channel in channels {
                match redis_manager.subscribe_to_channel(&channel).await {
                    Ok(receiver) => return Ok(receiver),
                    Err(e) => {
                        eprintln!("Falha ao conectar canal {}: {}", channel, e);
                        continue;
                    }
                }
            }

            Err(redis::RedisError::from((
                redis::ErrorKind::IoError,
                "Falha ao conectar a qualquer canal Redis"
            )))
        };

        let fut = fut.into_actor(self).map(|result, act, ctx| {
            match result {
                Ok(receiver) => {
                    act.redis_receiver = Some(receiver);
                    act.poll_redis_messages(ctx);
                    println!("Relay {}: Conectado ao Redis Cluster", act.relay_id);
                }
                Err(e) => {
                    eprintln!("Relay {}: Erro ao conectar ao Redis Cluster: {}", act.relay_id, e);
                    // Tentar reconectar após 5 segundos
                    ctx.run_later(Duration::from_secs(5), |act, ctx| {
                        act.start_redis_listener(ctx);
                    });
                }
            }
        });

        ctx.spawn(fut);
    }

    fn poll_redis_messages(&mut self, ctx: &mut Context<Self>) {
        if let Some(ref mut receiver) = self.redis_receiver {
            ctx.run_interval(Duration::from_millis(5), |act, _ctx| {
                let mut messages = Vec::new();
                let start_time = Instant::now();

                if let Some(ref mut receiver) = act.redis_receiver {
                    // Processar até 10 mensagens por vez para evitar bloqueio
                    for _ in 0..10 {
                        match receiver.try_recv() {
                            Ok(message) => messages.push(message),
                            Err(_) => break,
                        }
                    }
                }

                for message in messages.clone() {
                    act.handle_redis_message(message);
                }

                // Atualizar métricas de resposta
                if !messages.is_empty() {
                    let processing_time = start_time.elapsed().as_millis() as f64;
                    act.update_response_time(processing_time);
                }
            });
        }
    }

    fn handle_redis_message(&mut self, message: RedisMessage) {
        self.metrics.message_count += 1;
        self.metrics.last_message_time = Instant::now();

        match message.message_type {
            RedisMessageType::UserMessage(user_msg) => {
                println!("Relay {}: Mensagem Redis de {}: {}",
                         self.relay_id, user_msg.username, user_msg.content);

                // Distribuir para conexões locais exceto o remetente
                for (username, connection) in self.connections.iter() {
                    if username != &user_msg.username {
                        connection.do_send(user_msg.clone());
                    }
                }
            }
            RedisMessageType::JoinEvent(join_event) => {
                println!("Relay {}: Usuário {} entrou (via Redis)",
                         self.relay_id, join_event.username);

                for (_, connection) in self.connections.iter() {
                    connection.do_send(join_event.clone());
                }
            }
            RedisMessageType::UnRegisterConnection(unreg_msg) => {
                println!("Relay {}: Usuário {} saiu (via Redis)",
                         self.relay_id, unreg_msg.username);

                for (_, connection) in self.connections.iter() {
                    connection.do_send(unreg_msg.clone());
                }
            }
            RedisMessageType::RelayHeartbeat { relay_id, active_connections } => {
                if relay_id != self.relay_id {
                    println!("Relay {}: Heartbeat do relay {} do pod {} ({} conexões)",
                             self.relay_id, relay_id, message.from_pod_id, active_connections);
                }
            }
        }
    }

    fn update_response_time(&mut self, new_time: f64) {
        // Moving average simples
        self.metrics.avg_response_time = (self.metrics.avg_response_time * 0.9) + (new_time * 0.1);
    }

    pub fn get_metrics(&self) -> RelayMetrics {
        RelayMetrics {
            active_connections: self.connections.len(),
            message_count: self.metrics.message_count,
            last_message_time: self.metrics.last_message_time,
            avg_response_time: self.metrics.avg_response_time,
        }
    }

    // Health check periódico do Redis
    fn start_health_check(&self, ctx: &mut Context<Self>) {
        ctx.run_interval(Duration::from_secs(30), |act, ctx| {
            let redis_manager = act.redis_manager.clone();
            let relay_id = act.relay_id;

            let fut = async move {
                redis_manager.health_check().await
            };

            let fut = fut.into_actor(act).map(move |is_healthy, act, ctx| {
                if !is_healthy {
                    eprintln!("Relay {}: Redis Cluster não responsivo, tentando reconectar...", relay_id);
                    act.start_redis_listener(ctx);
                }
            });

            ctx.spawn(fut);
        });
    }
}

impl Actor for RelayActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        println!("RelayActor {} iniciado com Redis Cluster", self.relay_id);

        self.start_redis_listener(ctx);
        self.start_heartbeat(ctx);
        self.start_health_check(ctx);
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        println!("RelayActor {} parado", self.relay_id);
    }
}

impl Handler<RegisterConnection> for RelayActor {
    type Result = ();

    fn handle(&mut self, msg: RegisterConnection, ctx: &mut Self::Context) -> Self::Result {
        let is_new_user = !self.connections.contains_key(&msg.username);

        if !is_new_user {
            println!("Relay {}: Usuário {} já conectado", self.relay_id, msg.username);
            return;
        }

        // Notificar conexões locais
        for (_, connection) in self.connections.iter() {
            connection.do_send(JoinEvent {
                username: msg.username.clone(),
            })
        }

        self.connections.insert(msg.username.clone(), msg.addr);
        self.metrics.active_connections = self.connections.len();

        let redis_manager = self.redis_manager.clone();
        let relay_id = self.relay_id;
        let username = msg.username.clone();

        let fut = async move {
            let _ = redis_manager.set_user_location(&username, relay_id).await;

            // Publicar com fallback
            let primary_channel = format!("relay_events_{}", relay_id);
            let fallback_channel = "relay_events_global";

            let _ = redis_manager.publish_with_fallback(
                &primary_channel,
                &fallback_channel,
                relay_id,
                RedisMessageType::JoinEvent(JoinEvent { username }),
            ).await;
        };

        ctx.spawn(fut.into_actor(self));

        println!("Relay {}: Usuário {} conectado, total: {}",
                 self.relay_id, msg.username, self.connections.len());
    }
}

impl Handler<UnRegisterConnection> for RelayActor {
    type Result = ();

    fn handle(&mut self, msg: UnRegisterConnection, ctx: &mut Self::Context) -> Self::Result {
        if self.connections.remove(&msg.username).is_some() {
            self.metrics.active_connections = self.connections.len();

            for (_, connection) in self.connections.iter() {
                connection.do_send(msg.clone())
            }

            let redis_manager = self.redis_manager.clone();
            let relay_id = self.relay_id;
            let username = msg.username.clone();

            let fut = async move {
                let _ = redis_manager.remove_user_location(&username).await;

                let primary_channel = format!("relay_events_{}", relay_id);
                let fallback_channel = "relay_events_global";

                let _ = redis_manager.publish_with_fallback(
                    &primary_channel,
                    &fallback_channel,
                    relay_id,
                    RedisMessageType::UnRegisterConnection(UnRegisterConnection { username }),
                ).await;
            };

            ctx.spawn(fut.into_actor(self));

            println!("Relay {}: Usuário {} desconectado, total: {}",
                     self.relay_id, msg.username, self.connections.len());
        }
    }
}

impl Handler<UserMessage> for RelayActor {
    type Result = ();

    fn handle(&mut self, msg: UserMessage, ctx: &mut Self::Context) -> Self::Result {
        let start_time = Instant::now();

        println!("Relay {}: Mensagem de {}: {}", self.relay_id, msg.username, msg.content);

        // Distribuir localmente
        for (username, connection) in self.connections.iter() {
            if username != &msg.username {
                connection.do_send(msg.clone())
            }
        }

        let redis_manager = self.redis_manager.clone();
        let relay_id = self.relay_id;
        let user_msg = msg.clone();

        let fut = async move {
            let primary_channel = format!("relay_messages_{}", relay_id);
            let fallback_channel = "relay_messages_global";

            let _ = redis_manager.publish_with_fallback(
                &primary_channel,
                &fallback_channel,
                relay_id,
                RedisMessageType::UserMessage(user_msg),
            ).await;
        };

        ctx.spawn(fut.into_actor(self));

        // Atualizar métricas de performance
        let processing_time = start_time.elapsed().as_millis() as f64;
        self.update_response_time(processing_time);
        self.metrics.message_count += 1;
    }
}

impl Handler<GetMetrics> for RelayActor {
    type Result = actix::MessageResult<GetMetrics>;

    fn handle(&mut self, _msg: GetMetrics, _ctx: &mut Self::Context) -> Self::Result {
        actix::MessageResult(self.metrics.clone())
    }
}