use redis::{Client, Commands, Connection};
use tokio::sync::mpsc;
use std::env;
use futures_util::StreamExt;
use crate::actors::{RedisMessage, RedisMessageType};

#[derive(Clone)]
pub struct RedisManager {
    client: Client,
    pod_id: String,
}

impl RedisManager {
    pub fn new() -> Result<Self, redis::RedisError> {
        let redis_url = env::var("REDIS_URL")
            .unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());

        let pod_id = env::var("POD_NAME")
            .unwrap_or_else(|_| format!("pod-{}", std::process::id()));

        let client = Client::open(redis_url)?;

        Ok(Self { client, pod_id })
    }

    pub fn get_connection(&self) -> Result<Connection, redis::RedisError> {
        self.client.get_connection()
    }

    pub fn get_pod_id(&self) -> &str {
        &self.pod_id
    }

    pub async fn publish_message(
        &self,
        from_relay_id: u32,
        message_type: RedisMessageType,
    ) -> Result<(), redis::RedisError> {
        let client = self.client.clone();
        let pod_id = self.pod_id.clone();

        let message = RedisMessage {
            from_pod_id: pod_id,
            from_relay_id,
            message_type,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        };

        let payload = serde_json::to_string(&message)
            .map_err(|e| redis::RedisError::from((redis::ErrorKind::TypeError, "Serialization", e.to_string())))?;

        tokio::task::spawn_blocking(move || {
            let mut conn = client.get_connection()?;
            let _: () = conn.publish("relay_messages", payload)?;
            Ok::<_, redis::RedisError>(())
        })
            .await
            .unwrap()?;
        
        Ok(())
    }

    pub async fn subscribe_to_messages(&self) -> Result<mpsc::UnboundedReceiver<RedisMessage>, redis::RedisError> {
        let mut pubsub = self.client.get_async_pubsub().await?;
        pubsub.subscribe("relay_messages").await?;

        let (tx, rx) = mpsc::unbounded_channel();
        let pod_id = self.pod_id.clone();
        
        tokio::spawn(async move {
            let mut on_message_stream = pubsub.on_message();
            while let Some(msg) = on_message_stream.next().await {
                if let Ok(payload) = msg.get_payload::<String>() {
                    if let Ok(redis_message) = serde_json::from_str::<RedisMessage>(&payload) {
                        if redis_message.from_pod_id != pod_id {
                            if tx.send(redis_message).is_err() {
                                break;
                            }
                        }
                    }
                }
            }
        });

        Ok(rx)
    }

    pub async fn set_user_location(&self, username: &str, relay_id: u32) -> Result<(), redis::RedisError> {
        let client = self.client.clone();
        let key = format!("user_location:{}", username);
        let value = format!("{}:{}", self.pod_id, relay_id);

        tokio::task::spawn_blocking(move || {
            let mut conn = client.get_connection()?;
            let _: () = conn.set_ex(key, value, 300)?;
            Ok::<_, redis::RedisError>(())
        })
            .await
            .unwrap()?;
        
        Ok(())
    }

    pub async fn remove_user_location(&self, username: &str) -> Result<(), redis::RedisError> {
        let client = self.client.clone();
        let key = format!("user_location:{}", username);

        tokio::task::spawn_blocking(move || {
            let mut conn = client.get_connection()?;
            let _: () = conn.del(key)?;
            Ok::<_, redis::RedisError>(())
        })
            .await
            .unwrap()?;
        
        Ok(())
    }
}