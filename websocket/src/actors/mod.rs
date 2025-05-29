use crate::actors::ws::WsConn;

pub mod ws;
pub mod relay;

#[derive(actix::Message)]
#[rtype(result="()")]
pub struct RegisterConnection {
    username: String,
    addr: actix::Addr<WsConn>
}

#[derive(actix::Message, Clone, serde::Serialize, serde::Deserialize)]
#[rtype(result="()")]
pub struct JoinEvent {
    username: String
}

#[derive(actix::Message, Clone, serde::Serialize, serde::Deserialize)]
#[rtype(result="()")]
pub struct UnRegisterConnection {
    username: String,
}

#[derive(actix::Message, Clone, serde::Serialize, serde::Deserialize)]
#[rtype(result="()")]
pub struct UserMessage {
    username: String,
    content: String
}