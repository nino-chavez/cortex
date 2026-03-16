use crate::accessibility;
use crate::storage::Database;
use chrono::Utc;
use log::{error, info, warn};
use screencapturekit::cv::CVPixelBufferLockFlags;
use screencapturekit::prelude::*;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use xxhash_rust::xxh3::xxh3_64;

#[derive(Debug, Clone, PartialEq)]
pub enum CaptureStatus {
    Recording,
    Paused,
    Error(String),
    NeedsSetup,
}

impl std::fmt::Display for CaptureStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CaptureStatus::Recording => write!(f, "Recording"),
            CaptureStatus::Paused => write!(f, "Paused"),
            CaptureStatus::Error(msg) => write!(f, "Error: {}", msg),
            CaptureStatus::NeedsSetup => write!(f, "Needs Setup"),
        }
    }
}

pub struct CaptureState {
    pub status: CaptureStatus,
    pub interval_secs: u64,
    pub last_hashes: HashMap<u32, String>,
    pub stop_flag: bool,
}

impl CaptureState {
    pub fn new() -> Self {
        Self {
            status: CaptureStatus::Paused,
            interval_secs: 5,
            last_hashes: HashMap::new(),
            stop_flag: false,
        }
    }
}

pub type SharedCaptureState = Arc<Mutex<CaptureState>>;

fn cortex_data_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".cortex")
}

fn screenshots_dir_for_now() -> PathBuf {
    let now = Utc::now();
    cortex_data_dir()
        .join("screenshots")
        .join(now.format("%Y").to_string())
        .join(now.format("%m").to_string())
        .join(now.format("%d").to_string())
}

fn compute_hash(pixels: &[u8]) -> String {
    format!("{:016x}", xxh3_64(pixels))
}

fn encode_webp(pixels: &[u8], width: u32, height: u32, bytes_per_row: usize) -> Option<Vec<u8>> {
    // screencapturekit provides BGRA with potential row padding; webp expects tightly packed RGBA
    let expected_row = width as usize * 4;
    let mut rgba = vec![0u8; expected_row * height as usize];

    for y in 0..height as usize {
        let src_offset = y * bytes_per_row;
        let dst_offset = y * expected_row;
        for x in 0..width as usize {
            let si = src_offset + x * 4;
            let di = dst_offset + x * 4;
            if si + 3 < pixels.len() && di + 3 < rgba.len() {
                rgba[di] = pixels[si + 2];     // R <- B
                rgba[di + 1] = pixels[si + 1]; // G <- G
                rgba[di + 2] = pixels[si];     // B <- R
                rgba[di + 3] = pixels[si + 3]; // A <- A
            }
        }
    }

    let encoder = webp::Encoder::from_rgba(&rgba, width, height);
    let data = encoder.encode(80.0);
    Some(data.to_vec())
}

fn save_screenshot(
    pixels: &[u8],
    width: u32,
    height: u32,
    bytes_per_row: usize,
    display_id: u32,
) -> Option<(PathBuf, String)> {
    let hash = compute_hash(pixels);
    let webp_data = encode_webp(pixels, width, height, bytes_per_row)?;

    let dir = screenshots_dir_for_now();
    std::fs::create_dir_all(&dir).ok()?;

    let now = Utc::now();
    let filename = format!("{}_{}.webp", now.format("%H%M%S_%3f"), display_id);
    let path = dir.join(&filename);

    std::fs::write(&path, &webp_data).ok()?;
    Some((path, hash))
}

/// Process a single frame: check for change, save if new.
fn process_frame(
    state: &SharedCaptureState,
    db: &Database,
    display_id: u32,
    pixels: &[u8],
    width: u32,
    height: u32,
    bytes_per_row: usize,
) -> bool {
    let hash = compute_hash(pixels);

    // Change detection
    {
        let state_lock = state.lock().unwrap();
        if let Some(last_hash) = state_lock.last_hashes.get(&display_id) {
            if *last_hash == hash {
                return false;
            }
        }
    }

    let app_info = accessibility::get_focused_app();

    // Check excluded apps
    let config = crate::config::CortexConfig::load();
    if config.privacy.excluded_apps.contains(&app_info.bundle_id) {
        return false; // Skip capture for excluded apps
    }

    let (path, hash) = match save_screenshot(pixels, width, height, bytes_per_row, display_id) {
        Some(result) => result,
        None => {
            warn!("Failed to save screenshot for display {}", display_id);
            return false;
        }
    };

    let timestamp = Utc::now().to_rfc3339();
    if let Err(e) = db.insert_capture(
        &timestamp,
        &app_info.app_name,
        &app_info.bundle_id,
        &app_info.window_title,
        display_id,
        path.to_str().unwrap_or(""),
        &hash,
    ) {
        error!("Failed to insert capture: {}", e);
        std::fs::remove_file(&path).ok();
        return false;
    }

    {
        let mut state_lock = state.lock().unwrap();
        state_lock.last_hashes.insert(display_id, hash);
    }

    info!(
        "Captured: {} ({}) - {}",
        app_info.app_name, app_info.bundle_id, app_info.window_title
    );
    true
}

/// Start the capture loop on a background thread using ScreenCaptureKit.
pub fn start_capture_loop(state: SharedCaptureState, db: Arc<Database>) {
    std::thread::spawn(move || {
        let content = match SCShareableContent::get() {
            Ok(c) => c,
            Err(e) => {
                error!("Failed to get shareable content: {:?}", e);
                let mut state_lock = state.lock().unwrap();
                state_lock.status =
                    CaptureStatus::Error("Screen Recording permission required".to_string());
                return;
            }
        };

        let displays = content.displays();
        if displays.is_empty() {
            error!("No displays found");
            return;
        }

        let display = &displays[0];
        let display_id = display.display_id();
        let width = display.width() as u32;
        let height = display.height() as u32;

        let filter = SCContentFilter::create()
            .with_display(display)
            .with_excluding_windows(&[])
            .build();

        let interval_secs = {
            let state_lock = state.lock().unwrap();
            state_lock.interval_secs
        };

        let frame_interval = CMTime::new(interval_secs as i64, 1);

        let config = SCStreamConfiguration::new()
            .with_width(width)
            .with_height(height)
            .with_pixel_format(PixelFormat::BGRA)
            .with_shows_cursor(true)
            .with_minimum_frame_interval(&frame_interval);

        let state_clone = state.clone();
        let db_clone = db.clone();

        let mut stream = SCStream::new(&filter, &config);
        stream.add_output_handler(
            move |sample: CMSampleBuffer, of_type: SCStreamOutputType| {
                if of_type != SCStreamOutputType::Screen {
                    return;
                }

                // Check if still recording
                {
                    let state_lock = state_clone.lock().unwrap();
                    if state_lock.status != CaptureStatus::Recording {
                        return;
                    }
                }

                // Extract pixel data
                let buffer = match sample.image_buffer() {
                    Some(buf) => buf,
                    None => return,
                };

                let guard = match buffer.lock(CVPixelBufferLockFlags::READ_ONLY) {
                    Ok(g) => g,
                    Err(_) => return,
                };

                let pixels = guard.as_slice();
                let w = guard.width() as u32;
                let h = guard.height() as u32;
                let bpr = guard.bytes_per_row();

                if pixels.is_empty() || w == 0 || h == 0 {
                    return;
                }

                process_frame(&state_clone, &db_clone, display_id, pixels, w, h, bpr);
            },
            SCStreamOutputType::Screen,
        );

        if let Err(e) = stream.start_capture() {
            error!("Failed to start capture: {:?}", e);
            let mut state_lock = state.lock().unwrap();
            state_lock.status = CaptureStatus::Error(format!("Failed to start: {:?}", e));
            return;
        }

        info!("Capture loop started for display {}", display_id);

        // Keep thread alive while recording
        loop {
            std::thread::sleep(std::time::Duration::from_millis(500));
            let should_stop = {
                let state_lock = state.lock().unwrap();
                state_lock.stop_flag
            };
            if should_stop {
                break;
            }
        }

        stream.stop_capture().ok();
        info!("Capture loop stopped");
    });
}

pub fn request_stop(state: &SharedCaptureState) {
    let mut state_lock = state.lock().unwrap();
    state_lock.stop_flag = true;
    state_lock.status = CaptureStatus::Paused;
}

pub fn request_start(state: &SharedCaptureState) {
    let mut state_lock = state.lock().unwrap();
    state_lock.stop_flag = false;
    state_lock.status = CaptureStatus::Recording;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn change_detection_skips_identical_hashes() {
        let pixels_a = vec![255u8; 100];
        let pixels_b = vec![0u8; 100];

        let hash_a = compute_hash(&pixels_a);
        let hash_b = compute_hash(&pixels_b);

        assert_ne!(hash_a, hash_b);

        let hash_a2 = compute_hash(&pixels_a);
        assert_eq!(hash_a, hash_a2);
    }

    #[test]
    fn webp_encoding_produces_valid_data() {
        let pixels: Vec<u8> = vec![
            0, 0, 255, 255,
            0, 255, 0, 255,
            255, 0, 0, 255,
            255, 255, 255, 255,
        ];

        let result = encode_webp(&pixels, 2, 2, 8); // 2 pixels * 4 bytes = 8 bytes per row
        assert!(result.is_some());

        let webp_data = result.unwrap();
        assert!(!webp_data.is_empty());
        assert_eq!(&webp_data[0..4], b"RIFF");
        assert_eq!(&webp_data[8..12], b"WEBP");
    }
}
