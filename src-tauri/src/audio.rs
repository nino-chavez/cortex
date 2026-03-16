use crate::storage::Database;
use chrono::Utc;
use log::{error, info, warn};
use screencapturekit::prelude::*;
use screencapturekit::cv::CVPixelBufferLockFlags;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

const CHUNK_DURATION_SECS: u64 = 30;
const SAMPLE_RATE: u32 = 16000;

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

fn pending_dir() -> PathBuf {
    cortex_data_dir().join("audio").join("pending")
}

fn models_dir() -> PathBuf {
    cortex_data_dir().join("models").join("whisper")
}

fn model_path() -> PathBuf {
    models_dir().join("ggml-base.en.bin")
}

pub fn is_model_available() -> bool {
    model_path().exists()
}

/// Audio buffer that accumulates PCM samples and flushes to WAV every 30 seconds.
struct AudioChunker {
    samples: Vec<f32>,
    source: String,
    chunk_start: chrono::DateTime<chrono::Utc>,
}

impl AudioChunker {
    fn new(source: &str) -> Self {
        Self {
            samples: Vec::with_capacity(SAMPLE_RATE as usize * CHUNK_DURATION_SECS as usize),
            source: source.to_string(),
            chunk_start: Utc::now(),
        }
    }

    fn push_samples(&mut self, samples: &[f32]) {
        self.samples.extend_from_slice(samples);
    }

    fn should_flush(&self) -> bool {
        self.samples.len() >= (SAMPLE_RATE as usize * CHUNK_DURATION_SECS as usize)
    }

    fn flush(&mut self) -> Option<PathBuf> {
        if self.samples.is_empty() {
            return None;
        }

        let dir = pending_dir();
        std::fs::create_dir_all(&dir).ok()?;

        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let filename = format!("{}_{}.wav", timestamp, self.source);
        let path = dir.join(&filename);

        if let Err(e) = save_wav(&self.samples, &path) {
            error!("Failed to save audio chunk: {:?}", e);
            return None;
        }

        info!("Flushed {} audio chunk: {} samples", self.source, self.samples.len());
        self.samples.clear();
        self.chunk_start = Utc::now();
        Some(path)
    }
}

/// Start continuous audio capture via ScreenCaptureKit.
/// Captures system audio as a continuous stream, chunks into 30s WAV files.
pub fn start_audio_capture(source: &str, stop_flag: Arc<AtomicBool>) {
    let source_name = source.to_string();

    std::thread::spawn(move || {
        info!("Audio capture started ({})", source_name);

        let content = match SCShareableContent::get() {
            Ok(c) => c,
            Err(e) => {
                error!("Failed to get shareable content for audio: {:?}", e);
                return;
            }
        };

        let displays = content.displays();
        if displays.is_empty() {
            error!("No displays found for audio capture");
            return;
        }

        let display = &displays[0];

        let filter = SCContentFilter::create()
            .with_display(display)
            .with_excluding_windows(&[])
            .build();

        let config = SCStreamConfiguration::new()
            .with_width(1) // Minimal video (we only want audio)
            .with_height(1)
            .with_captures_audio(true)
            .with_sample_rate(48000)
            .with_channel_count(1);

        let chunker = Arc::new(Mutex::new(AudioChunker::new(&source_name)));
        let chunker_clone = chunker.clone();
        let stop_clone = stop_flag.clone();

        let mut stream = SCStream::new(&filter, &config);
        stream.add_output_handler(
            move |sample: CMSampleBuffer, of_type: SCStreamOutputType| {
                if of_type != SCStreamOutputType::Audio {
                    return;
                }

                if stop_clone.load(Ordering::Relaxed) {
                    return;
                }

                // Extract audio samples from CMSampleBuffer
                // Audio arrives as interleaved float32 PCM
                if let Some(buffer) = sample.image_buffer() {
                    if let Ok(guard) = buffer.lock(CVPixelBufferLockFlags::READ_ONLY) {
                        let raw = guard.as_slice();
                        // Convert bytes to f32 samples
                        let samples: Vec<f32> = raw
                            .chunks_exact(4)
                            .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
                            .collect();

                        // Downsample from 48kHz to 16kHz (take every 3rd sample)
                        let downsampled: Vec<f32> = samples.iter().step_by(3).copied().collect();

                        let mut chunker = chunker_clone.lock().unwrap();
                        chunker.push_samples(&downsampled);

                        if chunker.should_flush() {
                            chunker.flush();
                        }
                    }
                }
            },
            SCStreamOutputType::Audio,
        );

        if let Err(e) = stream.start_capture() {
            error!("Failed to start audio capture: {:?}", e);
            return;
        }

        // Keep alive until stop
        loop {
            std::thread::sleep(Duration::from_millis(500));
            if stop_flag.load(Ordering::Relaxed) {
                break;
            }
        }

        // Flush remaining audio
        {
            let mut chunker = chunker.lock().unwrap();
            chunker.flush();
        }

        stream.stop_capture().ok();
        info!("Audio capture stopped ({})", source_name);
    });
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
        let t0 = segment.start_timestamp();
        let t1 = segment.end_timestamp();

        if !text.trim().is_empty() {
            segments.push(TranscriptionSegment {
                text: text.trim().to_string(),
                start_cs: t0,
                end_cs: t1,
            });
        }
    }

    if segments.is_empty() { None } else { Some(segments) }
}

#[derive(Debug, Clone)]
pub struct TranscriptionSegment {
    pub text: String,
    pub start_cs: i64,
    pub end_cs: i64,
}

/// Save raw PCM samples as a 16kHz mono WAV file.
pub fn save_wav(samples: &[f32], path: &std::path::Path) -> Result<(), hound::Error> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: SAMPLE_RATE,
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

/// Start the audio transcription worker — polls pending/ for WAV files.
pub fn start_transcription_worker(db: Arc<Database>, stop_flag: Arc<AtomicBool>) {
    std::thread::spawn(move || {
        info!("Audio transcription worker started");

        loop {
            if stop_flag.load(Ordering::Relaxed) {
                break;
            }

            let pdir = pending_dir();
            if !pdir.exists() {
                std::thread::sleep(Duration::from_secs(5));
                continue;
            }

            let entries: Vec<_> = std::fs::read_dir(&pdir)
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
                let source = if filename.contains("_mic") { "mic" } else { "system" };

                info!("Transcribing: {:?}", path);

                if let Some(segments) = transcribe_wav(path.to_str().unwrap_or("")) {
                    let full_text: String = segments.iter().map(|s| s.text.as_str()).collect::<Vec<_>>().join(" ");
                    let now = Utc::now().to_rfc3339();

                    if let Err(e) = db.insert_transcription(None, &now, &now, &full_text, source, path.to_str().unwrap_or("")) {
                        error!("Failed to insert transcription: {:?}", e);
                        continue;
                    }

                    let processed_dir = audio_dir_for_now();
                    std::fs::create_dir_all(&processed_dir).ok();
                    let dest = processed_dir.join(path.file_name().unwrap_or_default());
                    std::fs::rename(&path, &dest).ok();
                    info!("Transcribed {} ({} segments)", filename, segments.len());
                } else {
                    warn!("No speech detected in {:?}", path);
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

        let samples = vec![0.0f32; 16000];
        save_wav(&samples, &path).unwrap();

        assert!(path.exists());
        let reader = hound::WavReader::open(&path).unwrap();
        assert_eq!(reader.spec().sample_rate, 16000);
        assert_eq!(reader.spec().channels, 1);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn audio_chunker_flushes_at_threshold() {
        let mut chunker = AudioChunker::new("system");
        assert!(!chunker.should_flush());

        // Push 30 seconds of samples at 16kHz
        let samples = vec![0.0f32; SAMPLE_RATE as usize * CHUNK_DURATION_SECS as usize];
        chunker.push_samples(&samples);
        assert!(chunker.should_flush());
    }

    #[test]
    fn transcription_segment_struct() {
        let seg = TranscriptionSegment {
            text: "hello world".to_string(),
            start_cs: 0,
            end_cs: 300,
        };
        assert_eq!(seg.text, "hello world");
        assert_eq!(seg.end_cs, 300);
    }
}
