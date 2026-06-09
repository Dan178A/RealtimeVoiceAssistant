use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Document {
    pub id: String,
    pub content: String,
    pub embedding: Option<Vec<f32>>,
}

pub struct RagEngine {
    pub documents: Vec<Document>,
}

impl RagEngine {
    pub fn new() -> Self {
        let mut engine = Self {
            documents: Vec::new(),
        };
        engine.load_defaults();
        engine
    }

    fn load_defaults(&mut self) {
        let defaults = vec![
            "Información de la empresa: Nuestro horario de atención es de 24 horas al día, 7 días a la semana (24/7). Soportamos múltiples idiomas, incluidos español, inglés, francés y alemán.",
            "Información de contacto: Puedes contactarnos vía email en soporte@ejemplo.com o por teléfono al +1 (555) 123-4567.",
            "Políticas de privacidad y seguridad: Todos los datos de audio transmitidos a través de WebSockets se procesan de forma temporal y segura. Cumplimos con estándares estrictos de protección de datos.",
            "Características del Asistente: Este asistente utiliza Google Gemini 2.5 Flash para responder preguntas. Tiene dos modos: Modo Texto Rápido y Modo Audio Híbrido. La palabra de activación predeterminada es 'asistente' y para finalizar di 'adiós' o 'apagar'."
        ];

        for text in defaults {
            self.documents.push(Document {
                id: Uuid::new_v4().to_string(),
                content: text.to_string(),
                embedding: None,
            });
        }
    }

    pub fn add_document(&mut self, content: String, embedding: Option<Vec<f32>>) -> Document {
        let doc = Document {
            id: Uuid::new_v4().to_string(),
            content,
            embedding,
        };
        self.documents.push(doc.clone());
        doc
    }

    pub fn delete_document(&mut self, id: &str) -> bool {
        let len_before = self.documents.len();
        self.documents.retain(|doc| doc.id != id);
        self.documents.len() < len_before
    }

    pub fn get_all_documents(&self) -> Vec<Document> {
        self.documents.clone()
    }

    pub fn search(&self, query: &str, query_embedding: Option<&Vec<f32>>, limit: usize) -> Vec<(Document, f32)> {
        let mut scored_docs: Vec<(Document, f32)> = self.documents.iter().map(|doc| {
            let score = if let (Some(q_emb), Some(doc_emb)) = (query_embedding, &doc.embedding) {
                // Vector search
                Self::cosine_similarity(q_emb, doc_emb)
            } else {
                // Fallback to text matching
                Self::keyword_similarity(&doc.content, query)
            };
            (doc.clone(), score)
        }).collect();

        // Sort by score descending
        scored_docs.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored_docs.truncate(limit);
        scored_docs
    }

    fn cosine_similarity(v1: &[f32], v2: &[f32]) -> f32 {
        if v1.len() != v2.len() || v1.is_empty() {
            return 0.0;
        }
        let dot_product: f32 = v1.iter().zip(v2.iter()).map(|(a, b)| a * b).sum();
        let norm_v1: f32 = v1.iter().map(|a| a * a).sum::<f32>().sqrt();
        let norm_v2: f32 = v2.iter().map(|b| b * b).sum::<f32>().sqrt();
        if norm_v1 == 0.0 || norm_v2 == 0.0 {
            0.0
        } else {
            dot_product / (norm_v1 * norm_v2)
        }
    }

    fn keyword_similarity(doc: &str, query: &str) -> f32 {
        let query_words: Vec<&str> = query
            .split_whitespace()
            .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()))
            .filter(|w| !w.is_empty())
            .collect();
        
        if query_words.is_empty() {
            return 0.0;
        }

        let doc_lower = doc.to_lowercase();
        let mut match_count = 0;
        for word in &query_words {
            if doc_lower.contains(&word.to_lowercase()) {
                match_count += 1;
            }
        }
        
        // Simple ratio of query terms matched
        match_count as f32 / query_words.len() as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rag_defaults_load() {
        let engine = RagEngine::new();
        assert!(!engine.documents.is_empty());
        assert_eq!(engine.documents.len(), 4);
    }

    #[test]
    fn test_add_delete_document() {
        let mut engine = RagEngine::new();
        let doc = engine.add_document("Hola mundo".to_string(), None);
        assert_eq!(engine.documents.len(), 5);
        
        let deleted = engine.delete_document(&doc.id);
        assert!(deleted);
        assert_eq!(engine.documents.len(), 4);
    }

    #[test]
    fn test_keyword_similarity() {
        let doc_content = "El horario de atención es de 9 AM a 5 PM de lunes a viernes.";
        
        // Match query
        let score_match = RagEngine::keyword_similarity(doc_content, "horario de atención");
        assert!(score_match > 0.0);

        // No match query
        let score_no_match = RagEngine::keyword_similarity(doc_content, "comida favorita");
        assert_eq!(score_no_match, 0.0);
    }

    #[test]
    fn test_cosine_similarity() {
        let vec1 = vec![1.0, 0.0, 0.0];
        let vec2 = vec![1.0, 0.0, 0.0];
        let vec3 = vec![0.0, 1.0, 0.0];

        // Identical vectors should have similarity of 1.0
        let sim1 = RagEngine::cosine_similarity(&vec1, &vec2);
        assert!((sim1 - 1.0).abs() < 1e-5);

        // Orthogonal vectors should have similarity of 0.0
        let sim2 = RagEngine::cosine_similarity(&vec1, &vec3);
        assert!(sim2.abs() < 1e-5);
    }
}

