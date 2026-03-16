use crate::storage::Database;
use log::{error, info};
use serde::Serialize;

const OLLAMA_BASE_URL: &str = "http://localhost:11434";
const MODEL: &str = "llama3.1";

#[derive(Debug, Clone, Serialize)]
pub struct SummaryResponse {
    pub summary: String,
    pub source_count: usize,
}

fn call_ollama(prompt: &str) -> Result<String, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| format!("HTTP client error: {}", e))?;

    #[derive(serde::Serialize)]
    struct Req { model: String, prompt: String, stream: bool }
    #[derive(serde::Deserialize)]
    struct Resp { response: String }

    let resp = client
        .post(format!("{}/api/generate", OLLAMA_BASE_URL))
        .json(&Req { model: MODEL.to_string(), prompt: prompt.to_string(), stream: false })
        .send()
        .map_err(|e| format!("Ollama request failed: {}", e))?;

    let body: Resp = resp.json().map_err(|e| format!("Parse error: {}", e))?;
    Ok(body.response)
}

/// Summarize captures/transcriptions in a time period.
pub fn summarize_period(db: &Database, from: &str, to: &str) -> Result<SummaryResponse, String> {
    let captures = db.get_captures_in_range(from, to)
        .map_err(|e| format!("DB error: {}", e))?;

    if captures.is_empty() {
        return Ok(SummaryResponse { summary: "No captures found in this time period.".to_string(), source_count: 0 });
    }

    let context: String = captures.iter().take(30).map(|c| {
        format!("[{} - {}] {}", c.timestamp, c.app_name, c.window_title)
    }).collect::<Vec<_>>().join("\n");

    let prompt = format!(
        "Summarize what the user was doing during this time period based on their screen captures. \
         Use bullet points, focus on key activities and applications.\n\nCapture log:\n{}\n\nSummary:",
        context
    );

    let summary = call_ollama(&prompt)?;
    info!("Period summary generated ({} sources)", captures.len());
    Ok(SummaryResponse { summary, source_count: captures.len() })
}

/// Summarize activity in a specific application.
pub fn summarize_app(db: &Database, app_name: &str, date: &str) -> Result<SummaryResponse, String> {
    let captures = db.get_captures_by_app(app_name, 50)
        .map_err(|e| format!("DB error: {}", e))?;

    let day_captures: Vec<_> = captures.into_iter()
        .filter(|c| c.timestamp.starts_with(date))
        .collect();

    if day_captures.is_empty() {
        return Ok(SummaryResponse { summary: format!("No captures from {} on {}.", app_name, date), source_count: 0 });
    }

    let context: String = day_captures.iter().map(|c| {
        format!("[{}] {}", c.timestamp, c.window_title)
    }).collect::<Vec<_>>().join("\n");

    let prompt = format!(
        "Summarize what the user was doing in {} on this day. Use bullet points.\n\nActivity log:\n{}\n\nSummary:",
        app_name, context
    );

    let summary = call_ollama(&prompt)?;
    Ok(SummaryResponse { summary, source_count: day_captures.len() })
}

/// Summarize captures related to a topic using semantic search.
pub fn summarize_topic(
    db: &Database,
    engine: &crate::embedding::EmbeddingEngine,
    topic: &str,
) -> Result<SummaryResponse, String> {
    let query_vec = engine.embed_text(topic).ok_or("Failed to embed topic")?;
    let results = db.semantic_search_captures(&query_vec, 15)
        .map_err(|e| format!("Search error: {}", e))?;

    if results.is_empty() {
        return Ok(SummaryResponse { summary: format!("No relevant captures found for '{}'.", topic), source_count: 0 });
    }

    let mut context_parts = Vec::new();
    for (capture_id, _distance) in &results {
        if let Some(capture) = db.get_capture_by_id(*capture_id).unwrap_or(None) {
            let text = db.get_capture_ocr_text(*capture_id).unwrap_or(None).unwrap_or_default();
            context_parts.push(format!("[{} - {}] {}", capture.timestamp, capture.app_name, text.chars().take(200).collect::<String>()));
        }
    }

    let prompt = format!(
        "Summarize everything related to '{}' based on the user's screen captures. \
         Use bullet points with citations.\n\nRelevant captures:\n{}\n\nSummary:",
        topic, context_parts.join("\n")
    );

    let summary = call_ollama(&prompt)?;
    Ok(SummaryResponse { summary, source_count: results.len() })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summary_response_serializes() {
        let resp = SummaryResponse {
            summary: "User worked on API refactoring.".to_string(),
            source_count: 5,
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("API refactoring"));
        assert!(json.contains("5"));
    }
}
