use crate::storage::Database;
use chrono::Utc;
use log::{error, info, warn};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

fn cortex_data_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".cortex")
}

fn audio_dir_for_now() -> PathBuf {
    let now = Utc::now();
    cortex_data_dir()
        .join("audio")
        .join(now.format("%Y").to_string())
        .join(now.format("%m").to_string())
        .join(now.format("%d").to_string())
}

fn models_dir() -> PathBuf {
    cortex_data_dir().join("models").join("whisper")
}

fn model_path() -> PathBuf {
    models_dir().join("ggml-base.en.bin")
}

/// Check if the whisper model is downloaded.
pub fn is_model_available() -> bool {
    model_path().exists()
}

/// Transcribe a WAV file using whisper-rs. Returns timestamped segments.
pub fn transcribe_wav(wav_path: &str) -> Option<Vec<TranscriptionSegment>> {
    let model = model_path();
    if !model.exists() {
        warn!("Whisper model not found at {:?}", model);
        return None;
    }

    let ctx = match whisper_rs::WhisperContext::new_with_params(
        model.to_str()?,
        whisper_rs::WhisperContextParameters::default(),
    ) {
        Ok(c) => c,
        Err(e) => {
            error!("Failed to load whisper model: {:?}", e);
            return None;
        }
    };

    let mut state = match ctx.create_state() {
        Ok(s) => s,
        Err(e) => {
            error!("Failed to create whisper state: {:?}", e);
            return None;
        }
    };

    // Read WAV file (must be 16kHz mono)
    let reader = match hound::WavReader::open(wav_path) {
        Ok(r) => r,
        Err(e) => {
            error!("Failed to open WAV file {}: {:?}", wav_path, e);
            return None;
        }
    };

    let samples: Vec<f32> = reader
        .into_samples::<i16>()
        .filter_map(|s| s.ok())
        .map(|s| s as f32 / 32768.0)
        .collect();

    if samples.is_empty() {
        return None;
    }

    let mut params = whisper_rs::FullParams::new(whisper_rs::SamplingStrategy::Greedy { best_of: 1 });
    params.set_language(Some("en"));
    params.set_print_progress(false);

    if let Err(e) = state.full(params, &samples) {
        error!("Whisper transcription failed: {:?}", e);
        return None;
    }

    let n = state.full_n_segments();
    let mut segments = Vec::new();

    for i in 0..n {
        let segment = match state.get_segment(i) {
            Some(s) => s,
            None => continue,
        };
        let text = segment.to_str_lossy().unwrap_or_default().to_string();
        let t0 = segment.start_timestamp(); // centiseconds
        let t1 = segment.end_timestamp();

        if !text.trim().is_empty() {
            segments.push(TranscriptionSegment {
                text: text.trim().to_string(),
                start_cs: t0,
                end_cs: t1,
            });
        }
    }

    if segments.is_empty() {
        None
    } else {
        Some(segments)
    }
}

#[derive(Debug, Clone)]
pub struct TranscriptionSegment {
    pub text: String,
    pub start_cs: i64, // centiseconds from chunk start
    pub end_cs: i64,
}

/// Save raw PCM samples as a 16kHz mono WAV file.
pub fn save_wav(samples: &[f32], path: &std::path::Path) -> Result<(), hound::Error> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 16000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut writer = hound::WavWriter::create(path, spec)?;
    for &sample in samples {
        let s = (sample * 32767.0).clamp(-32768.0, 32767.0) as i16;
        writer.write_sample(s)?;
    }
    writer.finalize()?;
    Ok(())
}

/// Start the audio transcription worker.
pub fn start_audio_worker(db: Arc<Database>, stop_flag: Arc<AtomicBool>) {
    std::thread::spawn(move || {
        info!("Audio transcription worker started");

        loop {
            if stop_flag.load(Ordering::Relaxed) {
                break;
            }

            // Check for WAV files in a "pending" queue directory
            let pending_dir = cortex_data_dir().join("audio").join("pending");
            if !pending_dir.exists() {
                std::thread::sleep(Duration::from_secs(5));
                continue;
            }

            let entries: Vec<_> = std::fs::read_dir(&pending_dir)
                .ok()
                .map(|rd| {
                    rd.filter_map(|e| e.ok())
                        .filter(|e| e.path().extension().map(|ext| ext == "wav").unwrap_or(false))
                        .collect()
                })
                .unwrap_or_default();

            if entries.is_empty() {
                std::thread::sleep(Duration::from_secs(5));
                continue;
            }

            for entry in entries {
                if stop_flag.load(Ordering::Relaxed) {
                    break;
                }

                let path = entry.path();
                let filename = path.file_stem().unwrap_or_default().to_string_lossy().to_string();

                // Parse source from filename: {timestamp}_{source}.wav
                let source = if filename.contains("_mic") { "mic" } else { "system" };

                info!("Transcribing: {:?}", path);

                if let Some(segments) = transcribe_wav(path.to_str().unwrap_or("")) {
                    let full_text: String = segments.iter().map(|s| s.text.as_str()).collect::<Vec<_>>().join(" ");
                    let now = Utc::now().to_rfc3339();

                    if let Err(e) = db.insert_transcription(
                        None,
                        &now,
                        &now,
                        &full_text,
                        source,
                        path.to_str().unwrap_or(""),
                    ) {
                        error!("Failed to insert transcription: {:?}", e);
                        continue;
                    }

                    // Move to processed directory
                    let processed_dir = audio_dir_for_now();
                    std::fs::create_dir_all(&processed_dir).ok();
                    let dest = processed_dir.join(path.file_name().unwrap_or_default());
                    std::fs::rename(&path, &dest).ok();

                    info!("Transcribed {} ({} segments)", filename, segments.len());
                } else {
                    warn!("No speech detected in {:?}", path);
                    // Move to processed anyway to avoid re-processing
                    std::fs::remove_file(&path).ok();
                }
            }
        }

        info!("Audio transcription worker stopped");
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_wav_creates_valid_file() {
        let dir = std::env::temp_dir().join(format!(
            "cortex_wav_test_{}", std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.wav");

        // Generate 1 second of silence
        let samples = vec![0.0f32; 16000];
        save_wav(&samples, &path).unwrap();

        assert!(path.exists());
        let metadata = std::fs::metadata(&path).unwrap();
        assert!(metadata.len() > 44); // WAV header is 44 bytes

        // Verify it's readable
        let reader = hound::WavReader::open(&path).unwrap();
        assert_eq!(reader.spec().sample_rate, 16000);
        assert_eq!(reader.spec().channels, 1);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn transcription_segment_struct() {
        let seg = TranscriptionSegment {
            text: "hello world".to_string(),
            start_cs: 0,
            end_cs: 300,
        };
        assert_eq!(seg.text, "hello world");
        assert_eq!(seg.start_cs, 0);
        assert_eq!(seg.end_cs, 300); // 3 seconds
    }
}
