use std::time::{Duration, Instant};
use actix::{Actor, ActorContext, AsyncContext, Handler, StreamHandler};
use actix_web_actors::ws::{Message, ProtocolError, WebsocketContext};
use bytestring::ByteString;
use crate::actors::{JoinEvent, RegisterConnection, UnRegisterConnection, UserMessage};
use crate::actors::relay::RelayActor;

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(6);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(12);

pub struct WsConn {
    username: String,
    relay_actor: actix::Addr<RelayActor>,
    heartbeat: Instant
}

impl WsConn {
    pub fn new(username: String, relay_actor: actix::Addr<RelayActor>) -> Self {
        WsConn {
            username,
            relay_actor,
            heartbeat: Instant::now()
        }
    }
    
    fn heartbeat(&mut self, ctx: &mut <WsConn as Actor>::Context) {
        ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
            if act.heartbeat.duration_since(Instant::now()) > CLIENT_TIMEOUT {
                ctx.stop();
                return;
            }
            
            ctx.ping(&[])
        });
    }
}

impl Actor for WsConn {
    type Context = WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.heartbeat(ctx);

        self.relay_actor.do_send(RegisterConnection {
            username: self.username.clone(),
            addr: ctx.address(),
        });
    }

    fn stopped(&mut self, ctx: &mut Self::Context) {
        self.relay_actor.do_send(UnRegisterConnection {
            username: self.username.clone(),
        });
    }
}

impl Handler<UserMessage> for WsConn {
    type Result = ();

    fn handle(&mut self, msg: UserMessage, ctx: &mut Self::Context) -> Self::Result {
        let content = serde_json::to_string(&msg).unwrap();
        ctx.write_raw(Message::Text(ByteString::from(content)));
    }
}

impl Handler<JoinEvent> for WsConn {
    type Result = ();

    fn handle(&mut self, msg: JoinEvent, ctx: &mut Self::Context) -> Self::Result {
        let content = serde_json::to_string(&msg).unwrap();
        ctx.write_raw(Message::Text(ByteString::from(content)));
    }
}

impl Handler<UnRegisterConnection> for WsConn {
    type Result = ();

    fn handle(&mut self, msg: UnRegisterConnection, ctx: &mut Self::Context) -> Self::Result {
        let content = serde_json::to_string(&msg).unwrap();
        ctx.write_raw(Message::Text(ByteString::from(content)));
    }
}

impl StreamHandler<Result<Message, ProtocolError>> for WsConn {
    fn handle(&mut self, msg: Result<Message, ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(Message::Ping(msg)) => {
                self.heartbeat = Instant::now();
                ctx.pong(&msg);
            },
            Ok(Message::Pong(_)) => {
                self.heartbeat = Instant::now();
            },
            Ok(Message::Text(text)) => {
                if let Ok(message) = serde_json::from_str::<UserMessage>(&text) {
                    self.relay_actor.do_send(message);
                }
            },
            Ok(Message::Close(_)) => {
                ctx.stop();
            }
            _ => {}
        }
    }
}