use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use log::{error, info};
use std::sync::Mutex;

/// Thread-safe wrapper around the fastembed model.
pub struct EmbeddingEngine {
    model: Mutex<TextEmbedding>,
}

impl EmbeddingEngine {
    /// Initialize the embedding model. Downloads on first use.
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        info!("Loading embedding model (all-MiniLM-L6-v2)...");
        let model = TextEmbedding::try_new(
            InitOptions::new(EmbeddingModel::AllMiniLML6V2)
                .with_show_download_progress(true),
        )?;
        info!("Embedding model loaded");
        Ok(EmbeddingEngine {
            model: Mutex::new(model),
        })
    }

    /// Generate an embedding vector for a single text.
    pub fn embed_text(&self, text: &str) -> Option<Vec<f32>> {
        let mut model = self.model.lock().unwrap();
        match model.embed(vec![text], None) {
            Ok(embeddings) => embeddings.into_iter().next(),
            Err(e) => {
                error!("Embedding failed: {:?}", e);
                None
            }
        }
    }

    /// Generate embeddings for multiple texts in a batch.
    pub fn embed_batch(&self, texts: Vec<&str>) -> Option<Vec<Vec<f32>>> {
        let mut model = self.model.lock().unwrap();
        match model.embed(texts, None) {
            Ok(embeddings) => Some(embeddings),
            Err(e) => {
                error!("Batch embedding failed: {:?}", e);
                None
            }
        }
    }

    /// Get the embedding dimension (384 for all-MiniLM-L6-v2).
    pub fn dimension(&self) -> usize {
        384
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embed_text_returns_correct_dimension() {
        let engine = EmbeddingEngine::new().expect("Failed to load model");
        let vec = engine.embed_text("hello world").expect("Failed to embed");
        assert_eq!(vec.len(), 384);
    }

    #[test]
    fn similar_texts_have_closer_embeddings() {
        let engine = EmbeddingEngine::new().expect("Failed to load model");

        let v1 = engine.embed_text("the cat sat on the mat").unwrap();
        let v2 = engine.embed_text("a feline rested on the rug").unwrap();
        let v3 = engine.embed_text("quantum physics equations").unwrap();

        let sim_12 = cosine_similarity(&v1, &v2);
        let sim_13 = cosine_similarity(&v1, &v3);

        assert!(
            sim_12 > sim_13,
            "Similar texts should have higher cosine similarity: cat/feline={:.4} vs cat/quantum={:.4}",
            sim_12, sim_13
        );
    }

    fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
        let mag_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let mag_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
        if mag_a == 0.0 || mag_b == 0.0 {
            return 0.0;
        }
        dot / (mag_a * mag_b)
    }
}
