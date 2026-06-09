use std::sync::Arc;
use tokio::sync::RwLock;
use crate::config::Config;
use crate::rag::RagEngine;

#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub rag_engine: Arc<RwLock<RagEngine>>,
}

impl AppState {
    pub fn new(config: Config, rag_engine: RagEngine) -> Self {
        Self {
            config,
            rag_engine: Arc::new(RwLock::new(rag_engine)),
        }
    }
}
