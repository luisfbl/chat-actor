use crate::actors::ws::WsConn;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use actix::{Actor, Context, Handler, AsyncContext, ActorFutureExt, WrapFuture};
use tokio::sync::mpsc;
use crate::actors::{
    JoinEvent, RegisterConnection, UnRegisterConnection, UserMessage,
    RedisMessage, RedisMessageType
};
use crate::actors::redis_manager::RedisManager;

pub struct RelayActor {
    relay_id: u32,
    connections: HashMap<String, actix::Addr<WsConn>>,
    redis_manager: RedisManager,
    redis_receiver: Option<mpsc::UnboundedReceiver<RedisMessage>>,
    last_heartbeat: Instant,
}

impl RelayActor {
    pub async fn new(relay_id: u32) -> Result<Self, redis::RedisError> {
        let redis_manager = RedisManager::new()?;

        Ok(Self {
            relay_id,
            connections: HashMap::new(),
            redis_manager,
            redis_receiver: None,
            last_heartbeat: Instant::now(),
        })
    }

    fn start_heartbeat(&self, ctx: &mut Context<Self>) {
        ctx.run_interval(Duration::from_secs(30), |act, _ctx| {
            let redis_manager = act.redis_manager.clone();
            let relay_id = act.relay_id;
            let active_connections = act.connections.len();

            let fut = async move {
                if let Err(e) = redis_manager.publish_message(
                    relay_id,
                    RedisMessageType::RelayHeartbeat {
                        relay_id,
                        active_connections,
                    },
                ).await {
                    eprintln!("Erro ao enviar heartbeat via Redis: {}", e);
                }
            };

            _ctx.spawn(fut.into_actor(act));
            act.last_heartbeat = Instant::now();
        });
    }

    fn start_redis_listener(&mut self, ctx: &mut Context<Self>) {
        let redis_manager = self.redis_manager.clone();

        let fut = async move {
            redis_manager.subscribe_to_messages().await
        };

        let fut = fut.into_actor(self).map(|result, act, ctx| {
            match result {
                Ok(receiver) => {
                    act.redis_receiver = Some(receiver);
                    act.poll_redis_messages(ctx);
                }
                Err(e) => {
                    eprintln!("Erro ao conectar ao Redis: {}", e);
                }
            }
        });

        ctx.spawn(fut);
    }

    fn poll_redis_messages(&mut self, ctx: &mut Context<Self>) {
        if let Some(ref mut receiver) = self.redis_receiver {
            ctx.run_interval(Duration::from_millis(10), |act, _ctx| {
                let mut messages = Vec::new();

                if let Some(ref mut receiver) = act.redis_receiver {
                    while let Ok(message) = receiver.try_recv() {
                        messages.push(message);
                    }
                }

                for message in messages {
                    act.handle_redis_message(message);
                }
            });
        }
    }

    fn handle_redis_message(&self, message: RedisMessage) {
        match message.message_type {
            RedisMessageType::UserMessage(user_msg) => {
                println!("Relay {}: Recebida mensagem Redis de {}: {}",
                         self.relay_id, user_msg.username, user_msg.content);
                
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
}

impl Actor for RelayActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        println!("RelayActor {} iniciado", self.relay_id);
        
        self.start_redis_listener(ctx);
        self.start_heartbeat(ctx);
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
        
        for (_, connection) in self.connections.iter() {
            connection.do_send(JoinEvent {
                username: msg.username.clone(),
            })
        }
        
        self.connections.insert(msg.username.clone(), msg.addr);
        
        let redis_manager = self.redis_manager.clone();
        let relay_id = self.relay_id;
        let username = msg.username.clone();

        let fut = async move {
            let _ = redis_manager.set_user_location(&username, relay_id).await;

            let _ = redis_manager.publish_message(
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
            for (_, connection) in self.connections.iter() {
                connection.do_send(msg.clone())
            }
            
            let redis_manager = self.redis_manager.clone();
            let relay_id = self.relay_id;
            let username = msg.username.clone();

            let fut = async move {
                let _ = redis_manager.remove_user_location(&username).await;

                let _ = redis_manager.publish_message(
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
        println!("Relay {}: Mensagem de {}: {}", self.relay_id, msg.username, msg.content);
        
        for (username, connection) in self.connections.iter() {
            if username != &msg.username {
                connection.do_send(msg.clone())
            }
        }
        
        let redis_manager = self.redis_manager.clone();
        let relay_id = self.relay_id;
        let user_msg = msg.clone();

        let fut = async move {
            let _ = redis_manager.publish_message(
                relay_id,
                RedisMessageType::UserMessage(user_msg),
            ).await;
        };

        ctx.spawn(fut.into_actor(self));
    }
}
