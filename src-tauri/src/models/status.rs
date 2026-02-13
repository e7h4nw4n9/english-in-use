use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "status", content = "message")]
pub enum ServiceStatus {
    Connected,
    Disconnected(String),
    NotConfigured,
    Testing,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionStatus {
    pub r2: ServiceStatus,
    pub d1: ServiceStatus,
}
