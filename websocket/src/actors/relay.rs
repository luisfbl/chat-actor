use crate::actors::ws::WsConn;
use std::collections::HashMap;
use crate::actors::{JoinEvent, RegisterConnection, UnRegisterConnection, UserMessage};

pub struct RelayActor {
    relay_id: u32,
    connections: HashMap<String, actix::Addr<WsConn>>,
}

impl RelayActor {
    pub fn new(relay_id: u32) -> Self {
        Self {
            relay_id,
            connections: HashMap::new()
        }
    }
}

impl actix::Actor for RelayActor {
    type Context = actix::Context<Self>;
}

impl actix::Handler<RegisterConnection> for RelayActor {
    type Result = ();

    fn handle(&mut self, msg: RegisterConnection, ctx: &mut Self::Context) -> Self::Result {
        let is_new_user = self.connections.contains_key(&msg.username);
        
        if !is_new_user { return; }
        
        for (_, connection) in self.connections.iter() {
            connection.do_send(JoinEvent {
                username: msg.username.clone(),
            })
        }
        
        self.connections
            .insert(msg.username, msg.addr);
    }
}

impl actix::Handler<UnRegisterConnection> for RelayActor {
    type Result = ();

    fn handle(&mut self, msg: UnRegisterConnection, ctx: &mut Self::Context) -> Self::Result {
        self.connections
            .remove(&msg.username);
        
        for (_, connection) in self.connections.iter() {
            connection.do_send(msg.clone())
        }
    }
}

impl actix::Handler<UserMessage> for RelayActor {
    type Result = ();

    fn handle(&mut self, msg: UserMessage, ctx: &mut Self::Context) -> Self::Result {
        for (username, connection) in self.connections.iter() {
            if username == &msg.username {
                continue;
            }
            
            connection.do_send(msg.clone())
        }
    }
}