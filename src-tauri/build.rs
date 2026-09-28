use std::path::Path;
use std::process::Command;

fn main() {
    // The Swift helper (Vision, Photos) is a Tauri sidecar: build it before Tauri checks that
    // `binaries/organiza-helper-<target>` exists.
    let target = std::env::var("TARGET").unwrap_or_default();
    println!("cargo:rustc-env=ORGANIZA_TARGET_TRIPLE={target}");
    println!("cargo:rerun-if-changed=../native/macos-helper/Sources/main.swift");
    println!("cargo:rerun-if-changed=../native/macos-helper/build.sh");
    let out = Path::new("binaries").join(format!("organiza-helper-{target}"));
    let source = Path::new("../native/macos-helper/Sources/main.swift");
    let stale = match (out.metadata().and_then(|m| m.modified()), source.metadata().and_then(|m| m.modified())) {
        (Ok(built), Ok(src)) => src > built,
        _ => true,
    };
    if stale && target.ends_with("apple-darwin") {
        let status = Command::new("sh")
            .arg("../native/macos-helper/build.sh")
            .env("TAURI_ENV_TARGET_TRIPLE", &target)
            .status()
            .expect("could not run the Swift helper build");
        assert!(status.success(), "building the Swift helper failed");
    }
    tauri_build::build()
}
