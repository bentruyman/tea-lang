use std::env;
use std::fs;
use std::path::PathBuf;

fn quote(value: &str) -> String {
    format!("{value:?}")
}

fn main() {
    let manifest_dir =
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is set by cargo"));
    let workspace_dir = manifest_dir
        .parent()
        .map(PathBuf::from)
        .unwrap_or_else(|| manifest_dir.clone());
    println!("cargo:rerun-if-env-changed=TEA_BUNDLED_RUNTIME_ARCHIVE");
    println!("cargo:rerun-if-env-changed=TEA_BUNDLED_RUNTIME_NATIVE_LIBS");
    println!("cargo:rerun-if-env-changed=TEA_BUNDLED_RUNTIME_TARGET");

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR is set by cargo"));
    let output = out_dir.join("bundled_assets.rs");

    let archive = env::var("TEA_BUNDLED_RUNTIME_ARCHIVE")
        .ok()
        .map(PathBuf::from);
    let target = env::var("TEA_BUNDLED_RUNTIME_TARGET").ok();
    let native_libs = env::var("TEA_BUNDLED_RUNTIME_NATIVE_LIBS")
        .unwrap_or_default()
        .split_whitespace()
        .map(quote)
        .collect::<Vec<_>>()
        .join(", ");

    let contents = match archive {
        Some(archive) => {
            let archive = archive
                .canonicalize()
                .unwrap_or_else(|_| workspace_dir.join(&archive))
                .display()
                .to_string();
            println!("cargo:rerun-if-changed={archive}");
            let target = quote(target.as_deref().unwrap_or("unknown"));
            format!(
                r##"pub const ENABLED: bool = true;
pub const TARGET: Option<&str> = Some({target});
pub const NATIVE_STATIC_LIBS: &[&str] = &[{native_libs}];
pub const RUNTIME_ARCHIVE: &[u8] = include_bytes!(r#"{archive}"#);
"##
            )
        }
        None => r#"pub const ENABLED: bool = false;
pub const TARGET: Option<&str> = None;
pub const NATIVE_STATIC_LIBS: &[&str] = &[];
pub const RUNTIME_ARCHIVE: &[u8] = &[];
"#
        .to_string(),
    };

    fs::write(output, contents).expect("write bundled asset metadata");
}
