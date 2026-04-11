use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use sha2::{Digest, Sha256};

include!(concat!(env!("OUT_DIR"), "/bundled_assets.rs"));

pub fn enabled() -> bool {
    ENABLED && !RUNTIME_ARCHIVE.is_empty()
}

pub fn target() -> Option<&'static str> {
    TARGET
}

pub fn native_static_libs() -> &'static [&'static str] {
    NATIVE_STATIC_LIBS
}

pub fn toolchain_label() -> Option<&'static str> {
    enabled().then_some("bundled-linkkit")
}

pub fn materialize_runtime_archive(root: &Path) -> Result<PathBuf> {
    if !enabled() {
        anyhow::bail!("bundled runtime archive is not available in this build");
    }

    let mut hasher = Sha256::new();
    hasher.update(RUNTIME_ARCHIVE);
    let digest = format!("{:x}", hasher.finalize());
    let digest_prefix = &digest[..16];
    let target = target().unwrap_or("host");

    let archive_path = root
        .join("linkkit")
        .join(target)
        .join(env!("CARGO_PKG_VERSION"))
        .join(format!("libtea_runtime-{digest_prefix}.a"));

    if archive_path.exists() {
        return Ok(archive_path);
    }

    if let Some(parent) = archive_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    fs::write(&archive_path, RUNTIME_ARCHIVE)
        .with_context(|| format!("failed to write {}", archive_path.display()))?;

    Ok(archive_path)
}
