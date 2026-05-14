fn main() {
    // Expose the build-time target triple to the runtime — used by the
    // Apple Intelligence resolver to locate the `dicto-apple-polish`
    // sidecar binary in dev mode (Tauri suffixes externalBins with the
    // target triple).
    let target = std::env::var("TARGET").unwrap_or_default();
    println!("cargo:rustc-env=DICTO_TARGET_TRIPLE={target}");

    tauri_build::build()
}
