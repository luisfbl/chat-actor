use crate::actors::ws::WsConn;

pub mod ws;
pub mod relay;
pub mod redis_manager;

#[derive(actix::Message)]
#[rtype(result="()")]
pub struct RegisterConnection {
    pub username: String,
    pub addr: actix::Addr<WsConn>
}

#[derive(actix::Message, Clone, serde::Serialize, serde::Deserialize)]
#[rtype(result="()")]
pub struct JoinEvent {
    pub username: String
}

#[derive(actix::Message, Clone, serde::Serialize, serde::Deserialize)]
#[rtype(result="()")]
pub struct UnRegisterConnection {
    pub username: String,
}

#[derive(actix::Message, Clone, serde::Serialize, serde::Deserialize)]
#[rtype(result="()")]
pub struct UserMessage {
    pub username: String,
    pub content: String
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct RedisMessage {
    pub from_pod_id: String,
    pub from_relay_id: u32,
    pub message_type: RedisMessageType,
    pub timestamp: u64,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub enum RedisMessageType {
    UserMessage(UserMessage),
    JoinEvent(JoinEvent),
    UnRegisterConnection(UnRegisterConnection),
    RelayHeartbeat { relay_id: u32, active_connections: usize },
}