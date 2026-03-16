fn main() {
    // Swift runtime rpath for screencapturekit-rs
    println!("cargo:rustc-link-arg=-Wl,-rpath,/usr/lib/swift");

    tauri_build::build()
}
