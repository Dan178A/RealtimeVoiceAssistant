use axum::{
    routing::{get, delete},
    Router,
};
use std::net::SocketAddr;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

mod config;
mod state;
mod rag;
mod gemini;
mod handlers;

use crate::config::Config;
use crate::state::AppState;
use crate::rag::RagEngine;
use crate::handlers::{
    ws_chat_handler,
    list_documents_handler,
    add_document_handler,
    delete_document_handler,
};

#[tokio::main]
async fn main() {
    // 1. Initialize logging
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Initializing Realtime Voice Assistant Backend in Rust...");

    // 2. Load configurations and state
    let config = Config::from_env();
    let rag_engine = RagEngine::new();
    let state = AppState::new(config.clone(), rag_engine);

    // 3. Configure CORS policy for easy web testing
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // 4. Build Axum routing
    let app = Router::new()
        .route("/ws/chat", get(ws_chat_handler))
        .route("/api/documents", get(list_documents_handler).post(add_document_handler))
        .route("/api/documents/:id", delete(delete_document_handler))
        // Serve all static frontend files from ./static/
        .fallback_service(ServeDir::new("./static"))
        .layer(cors)
        .with_state(state);

    // 5. Start the web server
    let addr = SocketAddr::from(([0, 0, 0, 0], config.server_port));
    tracing::info!("Server listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
