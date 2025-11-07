use anyhow::Result;
use path_clean::PathClean;
use pathdiff::diff_paths;
use std::env;
use std::path::{Component, Path as StdPath, PathBuf, MAIN_SEPARATOR};

/// Gets the platform-specific path separator
pub fn separator() -> String {
    MAIN_SEPARATOR.to_string()
}

/// Joins path parts into a single path
pub fn join(parts: &[String]) -> String {
    let mut path = PathBuf::new();
    for part in parts {
        path.push(part);
    }
    path.to_string_lossy().into_owned()
}

/// Extracts the normal components from a path
pub fn components(path: &str) -> Vec<String> {
    StdPath::new(path)
        .components()
        .filter_map(|c| match c {
            Component::Normal(s) => Some(s.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect()
}

/// Gets the directory name (parent) of a path
pub fn dirname(path: &str) -> String {
    StdPath::new(path)
        .parent()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// Gets the base name (file name) of a path
pub fn basename(path: &str) -> String {
    StdPath::new(path)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// Gets the extension of a path
pub fn extension(path: &str) -> String {
    StdPath::new(path)
        .extension()
        .map(|e| e.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// Normalizes a path by removing redundant separators and resolving . and ..
pub fn normalize(path: &str) -> String {
    let path_buf = StdPath::new(path);
    let normalized = path_buf.clean();
    normalized.to_string_lossy().into_owned()
}

/// Makes a path absolute, optionally relative to a base path
pub fn absolute(path: &str, base: Option<&str>) -> Result<String> {
    let target_path = PathBuf::from(path);

    // If already absolute, just clean and return
    if target_path.is_absolute() {
        let cleaned = target_path.clean();
        return Ok(cleaned.to_string_lossy().into_owned());
    }

    // Get or compute the base path
    let mut base_path = if let Some(base_str) = base {
        PathBuf::from(base_str)
    } else {
        env::current_dir()
            .map_err(|e| anyhow::anyhow!("failed to resolve current directory: {}", e))?
    };

    // If base is not absolute, make it absolute relative to cwd
    if !base_path.is_absolute() {
        let cwd = env::current_dir()
            .map_err(|e| anyhow::anyhow!("failed to resolve current directory: {}", e))?;
        base_path = cwd.join(base_path);
    }

    // Join and clean
    let combined = base_path.join(target_path);
    let cleaned = combined.clean();
    Ok(cleaned.to_string_lossy().into_owned())
}

/// Computes the relative path from one location to another
pub fn relative(from: &str, to: &str) -> String {
    let from_path = StdPath::new(from);
    let to_path = StdPath::new(to);
    diff_paths(to_path, from_path)
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|| to.to_string())
}
