use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use actix::Addr;
use serde::Serialize;
use crate::actors::relay::RelayActor;

#[derive(Debug, Clone, Serialize)]
pub struct RelayMetrics {
    pub relay_id: u32,
    pub active_connections: usize,
    pub message_throughput: f64, // msgs/sec
    pub avg_response_time: f64,  // ms
    pub last_updated: u64,
}

#[derive(Clone)]
pub struct DynamicRelayBalancer {
    relays: Arc<RwLock<HashMap<u32, Addr<RelayActor>>>>,
    metrics: Arc<RwLock<HashMap<u32, RelayMetrics>>>,
    user_relay_mapping: Arc<RwLock<HashMap<String, u32>>>,
    max_connections_per_relay: usize,
}

impl DynamicRelayBalancer {
    pub fn new(max_connections_per_relay: usize) -> Self {
        Self {
            relays: Arc::new(RwLock::new(HashMap::new())),
            metrics: Arc::new(RwLock::new(HashMap::new())),
            user_relay_mapping: Arc::new(RwLock::new(HashMap::new())),
            max_connections_per_relay,
        }
    }

    pub async fn add_relay(&self, relay_id: u32, relay_addr: Addr<RelayActor>) {
        let mut relays = self.relays.write().await;
        let mut metrics = self.metrics.write().await;

        relays.insert(relay_id, relay_addr);
        metrics.insert(relay_id, RelayMetrics {
            relay_id,
            active_connections: 0,
            message_throughput: 0.0,
            avg_response_time: 0.0,
            last_updated: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        });
    }

    pub async fn update_relay_metrics(&self, relay_id: u32, connections: usize, throughput: f64, response_time: f64) {
        let mut metrics = self.metrics.write().await;

        if let Some(metric) = metrics.get_mut(&relay_id) {
            metric.active_connections = connections;
            metric.message_throughput = throughput;
            metric.avg_response_time = response_time;
            metric.last_updated = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
        }
    }

    pub async fn get_best_relay_for_user(&self, username: &str) -> Option<u32> {
        {
            let mapping = self.user_relay_mapping.read().await;
            if let Some(&existing_relay_id) = mapping.get(username) {
                let metrics = self.metrics.read().await;
                if let Some(metric) = metrics.get(&existing_relay_id) {
                    if metric.active_connections < self.max_connections_per_relay {
                        return Some(existing_relay_id);
                    }
                }
            }
        }
        
        let best_relay_id = self.select_optimal_relay().await?;
        
        let mut mapping = self.user_relay_mapping.write().await;
        mapping.insert(username.to_string(), best_relay_id);

        Some(best_relay_id)
    }

    async fn select_optimal_relay(&self) -> Option<u32> {
        let metrics = self.metrics.read().await;

        if metrics.is_empty() {
            return None;
        }
        
        let mut best_relay_id = None;
        let mut best_score = f64::NEG_INFINITY;

        for (relay_id, metric) in metrics.iter() {
            let capacity_factor = 1.0 - (metric.active_connections as f64 / self.max_connections_per_relay as f64);
            let throughput_factor = 1.0 / (1.0 + metric.message_throughput / 1000.0);
            let response_time_factor = 1.0 / (1.0 + metric.avg_response_time / 100.0);
            
            if metric.active_connections >= self.max_connections_per_relay {
                continue;
            }

            let score = (capacity_factor * 0.5) + (throughput_factor * 0.3) + (response_time_factor * 0.2);

            if score > best_score {
                best_score = score;
                best_relay_id = Some(*relay_id);
            }
        }

        best_relay_id
    }

    pub async fn get_relay_addr(&self, relay_id: u32) -> Option<Addr<RelayActor>> {
        let relays = self.relays.read().await;
        relays.get(&relay_id).cloned()
    }

    pub async fn remove_user(&self, username: &str) {
        let mut mapping = self.user_relay_mapping.write().await;
        mapping.remove(username);
    }
    
    pub async fn rebalance_if_needed(&self) -> Vec<(String, u32, u32)> {
        let metrics = self.metrics.read().await;
        let mapping = self.user_relay_mapping.read().await;

        if metrics.len() < 2 {
            return vec![];
        }

        let mut rebalances = vec![];
        
        let max_load_relay = metrics.iter()
            .max_by_key(|(_, m)| m.active_connections)
            .map(|(id, _)| *id);

        let min_load_relay = metrics.iter()
            .min_by_key(|(_, m)| m.active_connections)
            .map(|(id, _)| *id);

        if let (Some(max_relay), Some(min_relay)) = (max_load_relay, min_load_relay) {
            if max_relay != min_relay {
                let max_connections = metrics.get(&max_relay).unwrap().active_connections;
                let min_connections = metrics.get(&min_relay).unwrap().active_connections;
                
                if max_connections > min_connections + (self.max_connections_per_relay / 3) {
                    let users_to_move: Vec<String> = mapping.iter()
                        .filter(|&(_, &relay_id)| relay_id == max_relay)
                        .take((max_connections - min_connections) / 2)
                        .map(|(username, _)| username.clone())
                        .collect();

                    for username in users_to_move {
                        rebalances.push((username, max_relay, min_relay));
                    }
                }
            }
        }

        rebalances
    }

    pub async fn get_relay_stats(&self) -> HashMap<u32, RelayMetrics> {
        self.metrics.read().await.clone()
    }
}