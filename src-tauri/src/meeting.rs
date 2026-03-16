use crate::capture::SharedCaptureState;
use crate::storage::Database;
use chrono::Utc;
use log::{error, info};
use serde::Serialize;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Serialize)]
pub struct MeetingRow {
    pub id: String,
    pub title: String,
    pub start_time: String,
    pub end_time: String,
    pub summary: String,
    pub participant_count: i32,
}

pub struct MeetingState {
    pub active: bool,
    pub meeting_id: Option<String>,
    pub start_time: Option<String>,
    previous_interval: u64,
}

impl MeetingState {
    pub fn new() -> Self {
        Self {
            active: false,
            meeting_id: None,
            start_time: None,
            previous_interval: 5,
        }
    }
}

pub type SharedMeetingState = Arc<Mutex<MeetingState>>;

/// Start a meeting: generate ID, set 2s capture interval, return meeting_id.
pub fn start_meeting(
    meeting_state: &SharedMeetingState,
    capture_state: &SharedCaptureState,
) -> String {
    let meeting_id = format!("mtg_{}", Utc::now().format("%Y%m%d_%H%M%S"));
    let now = Utc::now().to_rfc3339();

    let mut ms = meeting_state.lock().unwrap();
    ms.active = true;
    ms.meeting_id = Some(meeting_id.clone());
    ms.start_time = Some(now);

    // Override capture interval to 2s for meetings
    let mut cs = capture_state.lock().unwrap();
    ms.previous_interval = cs.interval_secs;
    cs.interval_secs = 2;

    info!("Meeting started: {}", meeting_id);
    meeting_id
}

/// End a meeting: restore interval, create meeting record.
pub fn end_meeting(
    meeting_state: &SharedMeetingState,
    capture_state: &SharedCaptureState,
    db: &Database,
) -> Result<MeetingRow, String> {
    let (meeting_id, start_time, prev_interval) = {
        let mut ms = meeting_state.lock().unwrap();
        let id = ms.meeting_id.take().ok_or("No active meeting")?;
        let start = ms.start_time.take().ok_or("No start time")?;
        let prev = ms.previous_interval;
        ms.active = false;
        (id, start, prev)
    };

    // Restore capture interval
    {
        let mut cs = capture_state.lock().unwrap();
        cs.interval_secs = prev_interval;
    }

    let end_time = Utc::now().to_rfc3339();

    let row = MeetingRow {
        id: meeting_id.clone(),
        title: "Untitled Meeting".to_string(),
        start_time: start_time.clone(),
        end_time: end_time.clone(),
        summary: String::new(),
        participant_count: 1,
    };

    db.insert_meeting(&row.id, &row.title, &row.start_time, &row.end_time, &row.summary, row.participant_count)
        .map_err(|e| format!("Failed to create meeting: {}", e))?;

    info!("Meeting ended: {}", meeting_id);

    // Summary generation happens when Ollama processes it via chat
    // For now, the summary field stays empty until explicitly requested

    Ok(row)
}

fn generate_meeting_summary(meeting_id: &str, db: &Database) {
    let transcriptions = match db.get_meeting_transcriptions(meeting_id) {
        Ok(t) => t,
        Err(e) => {
            error!("Failed to get meeting transcriptions: {}", e);
            return;
        }
    };

    if transcriptions.is_empty() {
        info!("No transcriptions for meeting {}, skipping summary", meeting_id);
        return;
    }

    let combined = transcriptions.join("\n\n");
    let prompt = format!(
        "Summarize this meeting transcript in 3-5 bullet points. Focus on key decisions, action items, and topics discussed.\n\nTranscript:\n{}\n\nSummary:",
        combined.chars().take(4000).collect::<String>()
    );

    // Use Ollama directly for summary
    let client = match reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
    {
        Ok(c) => c,
        Err(_) => return,
    };

    #[derive(serde::Serialize)]
    struct Req { model: String, prompt: String, stream: bool }
    #[derive(serde::Deserialize)]
    struct Resp { response: String }

    let resp = client
        .post("http://localhost:11434/api/generate")
        .json(&Req {
            model: "llama3.1".to_string(),
            prompt,
            stream: false,
        })
        .send();

    if let Ok(r) = resp {
        if let Ok(body) = r.json::<Resp>() {
            db.update_meeting_summary(meeting_id, &body.response).ok();
            info!("Meeting summary generated for {}", meeting_id);
        }
    }
}

/// Get the current meeting ID if a meeting is active.
pub fn current_meeting_id(meeting_state: &SharedMeetingState) -> Option<String> {
    let ms = meeting_state.lock().unwrap();
    if ms.active {
        ms.meeting_id.clone()
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capture::CaptureState;

    fn temp_db() -> (Database, std::path::PathBuf) {
        let dir = std::env::temp_dir().join(format!(
            "cortex_test_{}_{}", std::process::id(), std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.db");
        let db = Database::open(&path).unwrap();
        (db, dir)
    }

    #[test]
    fn start_meeting_sets_interval_and_id() {
        let ms: SharedMeetingState = Arc::new(Mutex::new(MeetingState::new()));
        let cs: SharedCaptureState = Arc::new(Mutex::new(CaptureState::new()));

        let id = start_meeting(&ms, &cs);
        assert!(id.starts_with("mtg_"));

        let cs_lock = cs.lock().unwrap();
        assert_eq!(cs_lock.interval_secs, 2);

        let ms_lock = ms.lock().unwrap();
        assert!(ms_lock.active);
    }

    #[test]
    fn end_meeting_restores_interval_and_creates_record() {
        let (db, dir) = temp_db();
        let ms: SharedMeetingState = Arc::new(Mutex::new(MeetingState::new()));
        let cs: SharedCaptureState = Arc::new(Mutex::new(CaptureState::new()));

        let _id = start_meeting(&ms, &cs);
        let row = end_meeting(&ms, &cs, &db).unwrap();

        assert_eq!(row.title, "Untitled Meeting");
        assert!(!row.start_time.is_empty());
        assert!(!row.end_time.is_empty());

        let cs_lock = cs.lock().unwrap();
        assert_eq!(cs_lock.interval_secs, 5);

        let ms_lock = ms.lock().unwrap();
        assert!(!ms_lock.active);

        // Verify it's in the DB
        let fetched = db.get_meeting(&row.id);
        assert!(fetched.is_some());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn meeting_insert_and_list_roundtrip() {
        let (db, dir) = temp_db();

        db.insert_meeting("mtg_1", "Standup", "2026-03-16T09:00:00Z", "2026-03-16T09:15:00Z", "Discussed sprint", 3).unwrap();
        db.insert_meeting("mtg_2", "Retro", "2026-03-16T14:00:00Z", "2026-03-16T15:00:00Z", "", 5).unwrap();

        let meetings = db.list_meetings(10).unwrap();
        assert_eq!(meetings.len(), 2);
        assert_eq!(meetings[0].title, "Retro"); // newest first

        std::fs::remove_dir_all(&dir).ok();
    }
}
