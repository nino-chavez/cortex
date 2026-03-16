fn main() {
    // Swift runtime rpath for screencapturekit-rs
    println!("cargo:rustc-link-arg=-Wl,-rpath,/usr/lib/swift");

    // Compile and link the Swift package for Vision OCR bridge
    swift_rs::SwiftLinker::new("13.0")
        .with_package("SwiftLib", "swift-lib/")
        .link();

    tauri_build::build()
}
