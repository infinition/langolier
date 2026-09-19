use std::path::PathBuf;
use std::process::Command;

fn main() {
    #[cfg(feature = "desktop")]
    tauri_build::build();
    println!("cargo::rustc-check-cfg=cfg(apple_model)");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        apple_model();
    }
}

/// Compiles the bridge to the model Apple ships with the system and links it
/// in. The framework only exists in recent SDKs, so an older toolchain simply
/// builds without it: `apple_model` stays unset and the Rust side reports the
/// system as too old. That keeps one source tree building on every runner.
fn apple_model() {
    let source = "swift/AppleModel.swift";
    println!("cargo:rerun-if-changed={source}");
    let Some(sdk) = sdk_path() else { return };
    if !sdk
        .join("System/Library/Frameworks/FoundationModels.framework")
        .exists()
    {
        println!("cargo:warning=SDK without FoundationModels: building without the Apple engine");
        return;
    }
    let arch = match std::env::var("CARGO_CFG_TARGET_ARCH").as_deref() {
        Ok("aarch64") => "arm64",
        Ok("x86_64") => "x86_64",
        other => {
            println!("cargo:warning=unknown architecture {other:?}: no Apple engine");
            return;
        }
    };
    let out = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR"));
    let lib = out.join("liblangolierapple.a");
    let status = Command::new("swiftc")
        .args([
            "-O",
            "-emit-library",
            "-static",
            "-module-name",
            "LangolierApple",
            "-target",
            &format!("{arch}-apple-macosx11.0"),
            "-sdk",
        ])
        .arg(&sdk)
        .arg("-o")
        .arg(&lib)
        .arg(source)
        .status();
    match status {
        Ok(s) if s.success() => {}
        Ok(s) => {
            println!("cargo:warning=swiftc failed ({s}): building without the Apple engine");
            return;
        }
        Err(e) => {
            println!("cargo:warning=swiftc unavailable ({e}): building without the Apple engine");
            return;
        }
    }
    println!("cargo:rustc-link-search=native={}", out.display());
    println!("cargo:rustc-link-lib=static=langolierapple");
    // Targeting an older macOS pulls in the shims that carry newer Swift
    // features back; they live in the toolchain, not in the system.
    if let Some(shims) = toolchain_swift() {
        println!("cargo:rustc-link-search=native={}", shims.display());
    }
    println!("cargo:rustc-link-lib=framework=FoundationModels");
    println!("cargo:rustc-link-lib=framework=Foundation");
    // The Swift runtime ships with macOS; the static library expects to find it.
    println!("cargo:rustc-link-search=native=/usr/lib/swift");
    println!(
        "cargo:rustc-link-search=native={}/usr/lib/swift",
        sdk.display()
    );
    println!("cargo:rustc-link-arg=-Wl,-rpath,/usr/lib/swift");
    println!("cargo:rustc-cfg=apple_model");
}

/// Where the toolchain keeps the static Swift compatibility libraries.
fn toolchain_swift() -> Option<PathBuf> {
    let out = Command::new("xcrun").args(["-f", "swiftc"]).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let swiftc = PathBuf::from(String::from_utf8(out.stdout).ok()?.trim());
    // .../usr/bin/swiftc -> .../usr/lib/swift/macosx
    let dir = swiftc.parent()?.parent()?.join("lib/swift/macosx");
    dir.is_dir().then_some(dir)
}

fn sdk_path() -> Option<PathBuf> {
    let out = Command::new("xcrun")
        .args(["--sdk", "macosx", "--show-sdk-path"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let path = String::from_utf8(out.stdout).ok()?;
    let path = PathBuf::from(path.trim());
    path.is_dir().then_some(path)
}
