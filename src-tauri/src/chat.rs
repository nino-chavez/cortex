use crate::embedding::EmbeddingEngine;
use crate::storage::Database;
use log::{error, info};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

const OLLAMA_BASE_URL: &str = "http://localhost:11434";
const DEFAULT_MODEL: &str = "llama3.1";
const TOP_K_CONTEXT: usize = 10;

#[derive(Debug, Clone, Serialize)]
pub struct ChatResponse {
    pub text: String,
    pub citations: Vec<Citation>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Citation {
    pub capture_id: i64,
    pub timestamp: String,
    pub app_name: String,
    pub snippet: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaStatus {
    pub available: bool,
    pub model_loaded: bool,
    pub model_name: String,
}

/// Check if Ollama is running and the model is available.
pub fn check_ollama() -> OllamaStatus {
    let client = match reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .build()
    {
        Ok(c) => c,
        Err(_) => {
            return OllamaStatus {
                available: false,
                model_loaded: false,
                model_name: DEFAULT_MODEL.to_string(),
            };
        }
    };

    // Check if Ollama is running
    let available = client
        .get(format!("{}/api/tags", OLLAMA_BASE_URL))
        .send()
        .is_ok();

    if !available {
        return OllamaStatus {
            available: false,
            model_loaded: false,
            model_name: DEFAULT_MODEL.to_string(),
        };
    }

    // Check if model is available
    #[derive(Deserialize)]
    struct TagsResponse {
        models: Vec<ModelInfo>,
    }
    #[derive(Deserialize)]
    struct ModelInfo {
        name: String,
    }

    let model_loaded = client
        .get(format!("{}/api/tags", OLLAMA_BASE_URL))
        .send()
        .ok()
        .and_then(|r| r.json::<TagsResponse>().ok())
        .map(|tags| tags.models.iter().any(|m| m.name.starts_with(DEFAULT_MODEL)))
        .unwrap_or(false);

    OllamaStatus {
        available,
        model_loaded,
        model_name: DEFAULT_MODEL.to_string(),
    }
}

/// Build a RAG prompt from retrieved context.
fn build_rag_prompt(query: &str, contexts: &[Citation]) -> String {
    let mut prompt = String::from(
        "You are Cortex, a local AI assistant that helps users recall information from their screen captures and audio transcriptions.\n\n\
         RULES:\n\
         - Answer ONLY using the provided context below. Do NOT use general knowledge.\n\
         - If the context does not contain enough information to answer, say \"I don't have enough captured data to answer that.\"\n\
         - For every claim you make, cite the source using [Source: {timestamp} - {app_name}] format.\n\
         - Do NOT fabricate or hallucinate sources. Only cite sources that appear in the context below.\n\
         - Be concise and direct.\n\n\
         Context from user's capture history:\n\n",
    );

    for (i, ctx) in contexts.iter().enumerate() {
        prompt.push_str(&format!(
            "---\n[{}] Time: {} | App: {}\n{}\n",
            i + 1,
            ctx.timestamp,
            ctx.app_name,
            ctx.snippet
        ));
    }

    prompt.push_str(&format!(
        "\n---\nUser question: {}\n\nAnswer with citations:",
        query
    ));

    prompt
}

/// Run the full RAG pipeline: embed query → search → build prompt → call Ollama.
pub fn chat_message(
    query: &str,
    db: &Database,
    engine: &EmbeddingEngine,
) -> Result<ChatResponse, String> {
    // Step 1: Embed the query
    let query_embedding = engine
        .embed_text(query)
        .ok_or("Failed to embed query")?;

    // Step 2: Search for relevant context via sqlite-vec
    let search_results = db
        .semantic_search_captures(&query_embedding, TOP_K_CONTEXT as i64)
        .map_err(|e| format!("Search failed: {}", e))?;

    // Step 3: Build citations from search results
    let mut citations = Vec::new();
    for (capture_id, _distance) in &search_results {
        if let Ok(Some(capture)) = db.get_capture_by_id(*capture_id) {
            let ocr_text = db
                .get_capture_ocr_text(*capture_id)
                .unwrap_or(None)
                .unwrap_or_default();

            citations.push(Citation {
                capture_id: *capture_id,
                timestamp: capture.timestamp.clone(),
                app_name: capture.app_name.clone(),
                snippet: ocr_text
                    .chars()
                    .take(500)
                    .collect(),
            });
        }
    }

    // Step 4: Build RAG prompt
    let prompt = build_rag_prompt(query, &citations);

    // Step 5: Call Ollama
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| format!("HTTP client error: {}", e))?;

    #[derive(Serialize)]
    struct OllamaRequest {
        model: String,
        prompt: String,
        stream: bool,
    }

    #[derive(Deserialize)]
    struct OllamaResponse {
        response: String,
    }

    let response = client
        .post(format!("{}/api/generate", OLLAMA_BASE_URL))
        .json(&OllamaRequest {
            model: DEFAULT_MODEL.to_string(),
            prompt,
            stream: false,
        })
        .send()
        .map_err(|e| format!("Ollama request failed: {}. Is Ollama running?", e))?;

    let ollama_resp: OllamaResponse = response
        .json()
        .map_err(|e| format!("Failed to parse Ollama response: {}", e))?;

    info!("Chat response generated with {} citations", citations.len());

    Ok(ChatResponse {
        text: ollama_resp.response,
        citations,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_rag_prompt_includes_context_and_query() {
        let citations = vec![
            Citation {
                capture_id: 1,
                timestamp: "2026-03-16T10:00:00Z".to_string(),
                app_name: "VS Code".to_string(),
                snippet: "fn main() { println!(\"hello\"); }".to_string(),
            },
        ];

        let prompt = build_rag_prompt("what was I coding?", &citations);
        assert!(prompt.contains("VS Code"));
        assert!(prompt.contains("fn main()"));
        assert!(prompt.contains("what was I coding?"));
        assert!(prompt.contains("[1]"));
    }

    #[test]
    fn check_ollama_returns_status() {
        // This test just verifies the function doesn't panic.
        // Actual Ollama availability depends on the test environment.
        let status = check_ollama();
        assert_eq!(status.model_name, DEFAULT_MODEL);
    }
}
