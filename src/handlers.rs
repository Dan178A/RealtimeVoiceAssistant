use axum::{
    extract::{State, Path, WebSocketUpgrade, ws::{Message, WebSocket}},
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use futures_util::{StreamExt, SinkExt};
use std::sync::Arc;
use crate::state::AppState;
use crate::gemini::GeminiClient;

#[derive(Deserialize)]
pub struct AddDocumentRequest {
    pub content: String,
}

#[derive(Serialize)]
pub struct AddDocumentResponse {
    pub id: String,
    pub content: String,
    pub has_embedding: bool,
}

/// Handler for listing all documents in the RAG knowledge base
pub async fn list_documents_handler(
    State(state): State<AppState>,
) -> impl IntoResponse {
    let engine = state.rag_engine.read().await;
    let docs = engine.get_all_documents();
    
    let response: Vec<serde_json::Value> = docs.iter().map(|doc| {
        serde_json::json!({
            "id": doc.id,
            "content": doc.content,
            "has_embedding": doc.embedding.is_some()
        })
    }).collect();

    Json(response)
}

/// Handler for adding a document to the RAG knowledge base (generates embeddings asynchronously)
pub async fn add_document_handler(
    State(state): State<AppState>,
    Json(payload): Json<AddDocumentRequest>,
) -> impl IntoResponse {
    let trimmed_content = payload.content.trim().to_string();
    if trimmed_content.is_empty() {
        return (
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "Content cannot be empty" })),
        ).into_response();
    }

    let gemini_client = GeminiClient::new(state.config.gemini_api_key.clone());
    
    // Generate embedding for the new document
    let embedding = match gemini_client.generate_embedding(&trimmed_content).await {
        Ok(emb) => Some(emb),
        Err(err) => {
            tracing::warn!("Failed to generate embedding for new document: {}. Storing text-only.", err);
            None
        }
    };

    let mut engine = state.rag_engine.write().await;
    let doc = engine.add_document(trimmed_content, embedding);

    (
        axum::http::StatusCode::CREATED,
        Json(AddDocumentResponse {
            id: doc.id,
            content: doc.content,
            has_embedding: doc.embedding.is_some(),
        }),
    ).into_response()
}

/// Handler for deleting a document from the RAG knowledge base
pub async fn delete_document_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let mut engine = state.rag_engine.write().await;
    let success = engine.delete_document(&id);

    if success {
        (
            axum::http::StatusCode::OK,
            Json(serde_json::json!({ "success": true, "message": "Document deleted successfully" })),
        ).into_response()
    } else {
        (
            axum::http::StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "Document not found" })),
        ).into_response()
    }
}

/// Upgrades HTTP connection to WebSocket
pub async fn ws_chat_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

/// Handles the full WebSocket lifetime (unified text & audio chunks)
async fn handle_socket(socket: WebSocket, state: AppState) {
    let (mut sender, mut receiver) = socket.split();
    let gemini_client = Arc::new(GeminiClient::new(state.config.gemini_api_key.clone()));
    
    tracing::info!("New client connected (Unified Text/Audio Mode).");

    while let Some(msg_result) = receiver.next().await {
        let msg = match msg_result {
            Ok(m) => m,
            Err(e) => {
                tracing::error!("WebSocket error: {}", e);
                break;
            }
        };

        match msg {
            Message::Text(text_content) => {
                let query = text_content.trim().to_string();
                if query.is_empty() {
                    continue;
                }
                
                tracing::info!("🎙️ User text query received: {}", query);
                
                // Process text query sequentially
                if let Err(e) = process_text_query(&query, &mut sender, Arc::clone(&gemini_client), state.clone()).await {
                    tracing::error!("Error processing text query: {}", e);
                }
            }
            Message::Binary(audio_bytes) => {
                if audio_bytes.is_empty() {
                    continue;
                }
                
                tracing::info!("🎙️ Binary audio received (size: {} bytes). Transcribing...", audio_bytes.len());
                
                // Transcribe audio using Gemini sequentially
                match gemini_client.transcribe_audio(&audio_bytes).await {
                    Ok(transcription) => {
                        let text = transcription.trim().to_string();
                        tracing::info!("📝 Audio Transcribed: '{}'", text);
                        
                        if text.is_empty() {
                            let _ = sender.send(Message::Text("[FIN_RESPUESTA]".to_string())).await;
                            continue;
                        }

                        // 1. Send the transcription to the frontend
                        let _ = sender.send(Message::Text(format!("[TRANSCRIPCION] {}", text))).await;

                        // 2. Process text and stream response
                        if let Err(e) = process_text_query(&text, &mut sender, Arc::clone(&gemini_client), state.clone()).await {
                            tracing::error!("Error processing transcribed text: {}", e);
                        }
                    }
                    Err(err) => {
                        tracing::error!("Failed to transcribe audio: {}", err);
                        let _ = sender.send(Message::Text(format!("⚠️ Error en transcripción de audio: {}", err))).await;
                        let _ = sender.send(Message::Text("[FIN_RESPUESTA]".to_string())).await;
                    }
                }
            }
            Message::Close(_) => {
                tracing::info!("Client disconnected (Closed socket).");
                break;
            }
            _ => {}
        }
    }
}

/// Helper to generate embedding, retrieve RAG context, and stream Gemini content back
async fn process_text_query(
    query: &str,
    sender: &mut futures_util::stream::SplitSink<WebSocket, Message>,
    gemini_client: Arc<GeminiClient>,
    state: AppState,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // 1. Get query embedding if possible
    let query_embedding = match gemini_client.generate_embedding(query).await {
        Ok(emb) => Some(emb),
        Err(err) => {
            tracing::warn!("Failed to generate query embedding: {}. Using fallback search.", err);
            None
        }
    };

    // 2. Search RAG
    let rag_context = {
        let engine = state.rag_engine.read().await;
        let results = engine.search(query, query_embedding.as_ref(), 3);
        
        let mut context_str = String::new();
        for (doc, score) in results {
            tracing::info!("RAG Match (score={:.3}): {}", score, doc.content);
            if score > 0.1 { // Minimal threshold to avoid completely unrelated noise
                context_str.push_str(&doc.content);
                context_str.push_str("\n");
            }
        }
        context_str
    };

    let context_for_gemini = if rag_context.is_empty() {
        "Sin información relevante en la base de datos de conocimiento."
    } else {
        &rag_context
    };

    // 3. Stream from Gemini
    let mut response_stream = gemini_client.generate_content_stream(query, context_for_gemini).await?;

    while let Some(chunk_result) = response_stream.next().await {
        match chunk_result {
            Ok(chunk) => {
                if !chunk.is_empty() {
                    sender.send(Message::Text(chunk)).await?;
                }
            }
            Err(e) => {
                tracing::error!("Error from Gemini stream: {}", e);
                sender.send(Message::Text(format!("⚠️ Error de streaming: {}", e))).await?;
                break;
            }
        }
    }

    // 4. Send end-of-response signal
    sender.send(Message::Text("[FIN_RESPUESTA]".to_string())).await?;
    tracing::info!("✅ Response streamed completely.");

    Ok(())
}
