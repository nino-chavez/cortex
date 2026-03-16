use swift_rs::{swift, SRString};

swift!(fn recognize_text(file_path: &SRString) -> SRString);

/// Run Apple Vision OCR on an image file. Returns extracted text or None if empty/failed.
pub fn recognize_text_from_file(path: &str) -> Option<String> {
    let sr_path: SRString = path.into();
    let result = unsafe { recognize_text(&sr_path) };
    let text = result.as_str().to_string();
    if text.trim().is_empty() {
        None
    } else {
        Some(text)
    }
}
