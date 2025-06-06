use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PodMetrics {
    pub pod_id: String,
    pub active_connections: usize,
    pub cpu_usage: f64,
    pub memory_usage: f64,
    pub relay_count: usize,
    pub last_updated: u64,
}

#[derive(Debug, Clone)]
pub struct LoadBalancer {
    pods: Arc<RwLock<HashMap<String, PodMetrics>>>,
    weights: Arc<RwLock<HashMap<String, f64>>>,
}

impl LoadBalancer {
    pub fn new() -> Self {
        Self {
            pods: Arc::new(RwLock::new(HashMap::new())),
            weights: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn update_pod_metrics(&self, metrics: PodMetrics) {
        let mut pods = self.pods.write().await;
        let mut weights = self.weights.write().await;
        
        let connection_factor = 1.0 - (metrics.active_connections as f64 / 1000.0).min(1.0);
        let cpu_factor = 1.0 - (metrics.cpu_usage / 100.0);
        let memory_factor = 1.0 - (metrics.memory_usage / 100.0);
        
        let weight = (connection_factor * 0.5) + (cpu_factor * 0.3) + (memory_factor * 0.2);

        pods.insert(metrics.pod_id.clone(), metrics.clone());
        weights.insert(metrics.pod_id, weight.max(0.1));
    }

    pub async fn select_best_pod(&self) -> Option<String> {
        let weights = self.weights.read().await;

        if weights.is_empty() {
            return None;
        }
        
        let total_weight: f64 = weights.values().sum();
        
        let mut hasher = DefaultHasher::new();
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos().hash(&mut hasher);
        let random_value = (hasher.finish() as f64 / u64::MAX as f64) * total_weight;

        let mut remaining_weight = random_value;
        for (pod_id, weight) in weights.iter() {
            remaining_weight -= weight;
            if remaining_weight <= 0.0 {
                return Some(pod_id.clone());
            }
        }
        
        weights.keys().next().cloned()
    }

    pub async fn get_pod_stats(&self) -> HashMap<String, PodMetrics> {
        self.pods.read().await.clone()
    }
    
    pub async fn cleanup_inactive_pods(&self) {
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let mut pods = self.pods.write().await;
        let mut weights = self.weights.write().await;

        let inactive_pods: Vec<String> = pods
            .iter()
            .filter(|(_, metrics)| current_time - metrics.last_updated > 60)
            .map(|(pod_id, _)| pod_id.clone())
            .collect();

        for pod_id in inactive_pods {
            pods.remove(&pod_id);
            weights.remove(&pod_id);
        }
    }
}