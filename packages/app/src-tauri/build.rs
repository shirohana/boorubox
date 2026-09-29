fn main() {
    // `ffmpeg.rs` reads this under `cfg(test)` to find `binaries/ffmpeg-<triple>`: a test binary
    // runs from target/debug/deps, not beside the sidecar.
    println!(
        "cargo:rustc-env=BOORUBOX_TARGET={}",
        std::env::var("TARGET").expect("cargo sets TARGET for build scripts")
    );
    tauri_build::build()
}
