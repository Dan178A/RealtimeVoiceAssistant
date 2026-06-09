use reqwest::Client;
use serde_json::json;
use futures_util::StreamExt;
use std::pin::Pin;
use futures_util::Stream;
use base64::prelude::*;

pub struct GeminiClient {
    client: Client,
    api_key: String,
}

impl GeminiClient {
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
        }
    }

    /// Generates vector embeddings for a given text using `text-embedding-004`
    pub async fn generate_embedding(&self, text: &str) -> Result<Vec<f32>, Box<dyn std::error::Error + Send + Sync>> {
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/text-embedding-004:embedContent?key={}",
            self.api_key
        );

        let payload = json!({
            "model": "models/text-embedding-004",
            "content": {
                "parts": [{ "text": text }]
            }
        });

        let response = self.client.post(&url)
            .json(&payload)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(format!("Gemini Embedding API Error ({}): {}", url, error_text).into());
        }

        let res_json: serde_json::Value = response.json().await?;
        let values = res_json.pointer("/embedding/values")
            .ok_or("Failed to extract embedding values from response")?
            .as_array()
            .ok_or("Embedding values are not an array")?;

        let mut embedding = Vec::with_capacity(values.len());
        for val in values {
            embedding.push(val.as_f64().ok_or("Embedding value is not a float")? as f32);
        }

        Ok(embedding)
    }

    /// Transcribes WebM audio bytes using `gemini-2.5-flash`
    pub async fn transcribe_audio(&self, audio_bytes: &[u8]) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash:generateContent?key={}",
            self.api_key
        );

        let audio_base64 = BASE64_STANDARD.encode(audio_bytes);

        let payload = json!({
            "contents": [{
                "parts": [
                    {
                        "inlineData": {
                            "mimeType": "audio/webm",
                            "data": audio_base64
                        }
                    },
                    {
                        "text": "Transcribe exactamente lo que se dice. Detecta el idioma automáticamente. Devuelve SOLO el texto de la transcripción, sin comillas, notas o explicaciones adicionales."
                    }
                ]
            }]
        });

        let response = self.client.post(&url)
            .json(&payload)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(format!("Gemini Audio Transcription API Error: {}", error_text).into());
        }

        let res_json: serde_json::Value = response.json().await?;
        let text = res_json.pointer("/candidates/0/content/parts/0/text")
            .ok_or("Failed to extract transcription text from response")?
            .as_str()
            .ok_or("Transcription text is not a string")?
            .trim()
            .to_string();

        Ok(text)
    }

    /// Streams completion response using `gemini-2.5-flash`
    pub async fn generate_content_stream(
        &self,
        prompt: &str,
        context: &str,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<String, reqwest::Error>> + Send>>, Box<dyn std::error::Error + Send + Sync>> {
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash:streamGenerateContent?key={}",
            self.api_key
        );

        let system_instruction = "Eres un asistente de voz en tiempo real. Responde de forma natural, clara y muy breve (máximo 2-3 oraciones). Responde en el mismo idioma en el que te habla el usuario.";
        
        let prompt_final = format!(
            "{}\n\n<contexto_adicional>\n{}\n</contexto_adicional>\n\nUsuario: {}",
            system_instruction, context, prompt
        );

        let payload = json!({
            "contents": [{
                "parts": [{ "text": prompt_final }]
            }]
        });

        let response = self.client.post(&url)
            .json(&payload)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(format!("Gemini Content Streaming API Error: {}", error_text).into());
        }

        let bytes_stream = response.bytes_stream();
        let mut parser = GeminiStreamParser::new();

        let text_stream = bytes_stream.map(move |item| {
            match item {
                Ok(bytes) => {
                    let text = String::from_utf8_lossy(&bytes);
                    let tokens = parser.push(&text);
                    Ok(tokens.join(""))
                }
                Err(e) => Err(e),
            }
        });

        Ok(Box::pin(text_stream))
    }
}

/// Helper structure to parse the segmented JSON array returned by Gemini's streaming API
struct GeminiStreamParser {
    buffer: String,
}

impl GeminiStreamParser {
    fn new() -> Self {
        Self { buffer: String::new() }
    }

    fn push(&mut self, text: &str) -> Vec<String> {
        self.buffer.push_str(text);
        let mut results = Vec::new();

        while let Some(start_idx) = self.buffer.find('{') {
            let mut depth = 0;
            let mut end_idx = None;

            for (i, c) in self.buffer[start_idx..].char_indices() {
                if c == '{' {
                    depth += 1;
                } else if c == '}' {
                    depth -= 1;
                    if depth == 0 {
                        end_idx = Some(start_idx + i + 1);
                        break;
                    }
                }
            }

            if let Some(end) = end_idx {
                let json_str = &self.buffer[start_idx..end];
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(json_str) {
                    if let Some(text) = val.pointer("/candidates/0/content/parts/0/text").and_then(|v| v.as_str()) {
                        results.push(text.to_string());
                    }
                }
                self.buffer = self.buffer[end..].to_string();
            } else {
                break;
            }
        }

        results
    }
}
