//! Vendored LLVM and LLD static libraries for tea-lang
//!
//! This crate provides static LLVM 17 and LLD libraries for embedding into the tea compiler.
//! The libraries are built per-platform and linked statically to eliminate runtime dependencies.
//!
//! ## License
//!
//! LLVM is licensed under Apache-2.0 with LLVM exception.
//! See https://llvm.org/LICENSE.txt for full license text.

use std::path::PathBuf;

/// LLVM version embedded in this crate
pub const LLVM_VERSION: &str = "17.0.6";

/// Target triple for the vendored libraries
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
pub const TARGET: &str = "aarch64-apple-darwin";

/// Returns the LLVM license text
pub fn llvm_license() -> &'static str {
    include_str!("../LLVM-LICENSE.txt")
}

/// Returns the install directory for vendored libraries
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
pub fn install_dir() -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir).join("install-macos-arm64")
}

#[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
pub fn install_dir() -> PathBuf {
    panic!("tea-llvm-vendor: unsupported platform - only macOS arm64 is currently supported")
}

/// Returns the runtime artifacts directory
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
pub fn runtime_artifacts_dir() -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir).join("runtime-artifacts-macos-arm64")
}

#[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
pub fn runtime_artifacts_dir() -> PathBuf {
    panic!("tea-llvm-vendor: unsupported platform - only macOS arm64 is currently supported")
}

/// Returns the path to the embedded tea-runtime staticlib
pub fn runtime_staticlib_path() -> PathBuf {
    runtime_artifacts_dir().join("libtea_runtime.a")
}

/// Returns the path to the embedded entry stub object
pub fn entry_stub_path() -> PathBuf {
    runtime_artifacts_dir().join("entry_stub.o")
}

/// Returns the embedded runtime staticlib as bytes (for writing to cache)
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
pub fn runtime_staticlib_bytes() -> Option<&'static [u8]> {
    // Will be populated after building runtime artifacts
    // For now, return None and we'll read from disk
    None
}

/// Returns the embedded entry stub as bytes (for writing to cache)
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
pub fn entry_stub_bytes() -> Option<&'static [u8]> {
    // Will be populated after building runtime artifacts
    // For now, return None and we'll read from disk
    None
}
