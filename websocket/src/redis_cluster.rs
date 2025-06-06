// src/redis_cluster.rs
use redis::{Client, Commands, Connection};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::actors::{RedisMessage, RedisMessageType};
use std::time::Duration;

#[derive(Clone)]
pub struct RedisClusterManager {
    clients: Vec<Client>,
    pod_id: String,
    channel_mapping: Arc<RwLock<HashMap<String, usize>>>,
    is_cluster_mode: bool,
}

impl RedisClusterManager {
    pub fn new() -> Result<Self, redis::RedisError> {
        let redis_nodes = std::env::var("REDIS_CLUSTER_NODES")
            .unwrap_or_else(|_| "redis://redis-0.redis.default.svc.cluster.local:6379,redis://redis-1.redis.default.svc.cluster.local:6379,redis://redis-2.redis.default.svc.cluster.local:6379".to_string());

        let node_urls: Vec<&str> = redis_nodes.split(',').collect();
        let mut clients = Vec::new();

        // Tentar conectar a cada nó
        for url in &node_urls {
            let url = url.trim();
            println!("Tentando conectar ao Redis: {}", url);

            match Client::open(url) {
                Ok(client) => {
                    // Testar conexão
                    match client.get_connection() {
                        Ok(mut conn) => {
                            match redis::cmd("PING").query::<String>(&mut conn) {
                                Ok(response) if response == "PONG" => {
                                    println!("✅ Conectado com sucesso: {}", url);
                                    clients.push(client);
                                }
                                Ok(response) => {
                                    println!("⚠️  Resposta inesperada de {}: {}", url, response);
                                }
                                Err(e) => {
                                    println!("❌ Falha no PING para {}: {}", url, e);
                                }
                            }
                        }
                        Err(e) => {
                            println!("❌ Falha na conexão para {}: {}", url, e);
                        }
                    }
                }
                Err(e) => {
                    println!("❌ Falha ao criar cliente para {}: {}", url, e);
                }
            }
        }

        // Se nenhum nó cluster funcionar, usar fallback
        if clients.is_empty() {
            let fallback_urls = vec![
                "redis://redis.default.svc.cluster.local:6379",
                "redis://redis-service:6379",
                "redis://localhost:6379"
            ];

            println!("⚠️  Cluster Redis indisponível, tentando fallbacks...");

            for fallback_url in fallback_urls {
                println!("Tentando fallback: {}", fallback_url);
                match Client::open(fallback_url) {
                    Ok(client) => {
                        match client.get_connection() {
                            Ok(mut conn) => {
                                if redis::cmd("PING").query::<String>(&mut conn).is_ok() {
                                    println!("✅ Fallback funcionando: {}", fallback_url);
                                    clients.push(client);
                                    break;
                                }
                            }
                            Err(e) => {
                                println!("❌ Fallback falhou {}: {}", fallback_url, e);
                            }
                        }
                    }
                    Err(e) => {
                        println!("❌ Erro no cliente fallback {}: {}", fallback_url, e);
                    }
                }
            }
        }

        if clients.is_empty() {
            return Err(redis::RedisError::from((
                redis::ErrorKind::IoError,
                "Não foi possível conectar a nenhuma instância Redis"
            )));
        }

        let pod_id = std::env::var("POD_NAME")
            .unwrap_or_else(|_| format!("pod-{}", std::process::id()));

        let is_cluster_mode = clients.len() > 1;

        println!("🎯 Redis Manager inicializado:");
        println!("  📦 Pod ID: {}", pod_id);
        println!("  🔗 Conexões: {}", clients.len());
        println!("  🏗️  Modo cluster: {}", is_cluster_mode);

        Ok(Self {
            clients,
            pod_id,
            channel_mapping: Arc::new(RwLock::new(HashMap::new())),
            is_cluster_mode,
        })
    }

    // Particiona canais baseado em hash consistente
    fn get_client_for_channel(&self, channel: &str) -> &Client {
        if !self.is_cluster_mode || self.clients.len() == 1 {
            return &self.clients[0];
        }

        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        channel.hash(&mut hasher);
        let index = (hasher.finish() % self.clients.len() as u64) as usize;

        &self.clients[index]
    }

    pub async fn publish_message(
        &self,
        channel: &str,
        from_relay_id: u32,
        message_type: RedisMessageType,
    ) -> Result<(), redis::RedisError> {
        let message = RedisMessage {
            from_pod_id: self.pod_id.clone(),
            from_relay_id,
            message_type,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        };

        let payload = serde_json::to_string(&message)
            .map_err(|e| redis::RedisError::from((
                redis::ErrorKind::TypeError,
                "Serialization",
                e.to_string()
            )))?;

        let client = self.get_client_for_channel(channel);

        // Usar blocking task para evitar problemas de async
        let client = client.clone();
        let channel = channel.to_string();

        tokio::task::spawn_blocking(move || {
            let mut conn = client.get_connection()?;
            let _: () = conn.publish(channel, payload)?;
            Ok::<_, redis::RedisError>(())
        })
            .await
            .map_err(|e| redis::RedisError::from((
                redis::ErrorKind::IoError,
                "Task",
                e.to_string()
            )))??;

        Ok(())
    }

    pub async fn subscribe_to_channel(&self, channel: &str) -> Result<tokio::sync::mpsc::UnboundedReceiver<RedisMessage>, redis::RedisError> {
        let client = self.get_client_for_channel(channel).clone();
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let pod_id = self.pod_id.clone();
        let channel = channel.to_string();

        tokio::spawn(async move {
            loop {
                match client.get_async_pubsub().await {
                    Ok(mut pubsub) => {
                        if let Ok(_) = pubsub.subscribe(&channel).await {
                            println!("Conectado ao canal Redis: {}", channel);

                            use futures_util::StreamExt;
                            let mut stream = pubsub.into_on_message();

                            while let Some(msg) = stream.next().await {
                                if let Ok(payload) = msg.get_payload::<String>() {
                                    if let Ok(redis_message) = serde_json::from_str::<RedisMessage>(&payload) {
                                        if redis_message.from_pod_id != pod_id {
                                            if tx.send(redis_message).is_err() {
                                                println!("Canal fechado para {}", channel);
                                                return;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Erro na conexão Redis para {}: {}", channel, e);
                    }
                }

                // Reconectar após 3 segundos em caso de erro
                tokio::time::sleep(Duration::from_secs(3)).await;
                println!("Tentando reconectar ao Redis para canal: {}", channel);
            }
        });

        Ok(rx)
    }

    // Implementar circuit breaker para tolerância a falhas
    pub async fn publish_with_fallback(
        &self,
        primary_channel: &str,
        fallback_channel: &str,
        from_relay_id: u32,
        message_type: RedisMessageType,
    ) -> Result<(), redis::RedisError> {
        match self.publish_message(primary_channel, from_relay_id, message_type.clone()).await {
            Ok(_) => Ok(()),
            Err(e) => {
                eprintln!("Falha no canal primário {}, tentando fallback {}: {}",
                          primary_channel, fallback_channel, e);
                self.publish_message(fallback_channel, from_relay_id, message_type).await
            }
        }
    }

    // Health check das conexões
    pub async fn health_check(&self) -> bool {
        let client = &self.clients[0]; // Testar pelo menos uma conexão

        let client = client.clone();
        match tokio::task::spawn_blocking(move || {
            let mut conn = client.get_connection()?;
            redis::cmd("PING").query::<String>(&mut conn)
        }).await {
            Ok(Ok(response)) => response == "PONG",
            _ => false,
        }
    }

    pub async fn set_user_location(&self, username: &str, relay_id: u32) -> Result<(), redis::RedisError> {
        let client = self.get_client_for_channel(&format!("user:{}", username)).clone();
        let key = format!("user_location:{}", username);
        let value = format!("{}:{}", self.pod_id, relay_id);

        tokio::task::spawn_blocking(move || {
            let mut conn = client.get_connection()?;
            let _: () = conn.set_ex(key, value, 300)?;
            Ok::<_, redis::RedisError>(())
        })
            .await
            .map_err(|e| redis::RedisError::from((
                redis::ErrorKind::IoError,
                "Task",
                e.to_string()
            )))??;

        Ok(())
    }

    pub async fn remove_user_location(&self, username: &str) -> Result<(), redis::RedisError> {
        let client = self.get_client_for_channel(&format!("user:{}", username)).clone();
        let key = format!("user_location:{}", username);

        tokio::task::spawn_blocking(move || {
            let mut conn = client.get_connection()?;
            let _: () = conn.del(key)?;
            Ok::<_, redis::RedisError>(())
        })
            .await
            .map_err(|e| redis::RedisError::from((
                redis::ErrorKind::IoError,
                "Task",
                e.to_string()
            )))??;

        Ok(())
    }

    pub fn get_cluster_info(&self) -> HashMap<String, String> {
        let mut info = HashMap::new();
        info.insert("pod_id".to_string(), self.pod_id.clone());
        info.insert("client_count".to_string(), self.clients.len().to_string());
        info.insert("is_cluster_mode".to_string(), self.is_cluster_mode.to_string());
        info
    }
}