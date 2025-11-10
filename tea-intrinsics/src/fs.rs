use anyhow::Result;
use std::fs;
use std::path::Path as StdPath;
use tea_support::fs_error;
use walkdir::WalkDir;

/// Reads the entire contents of a file as a string
pub fn read_text(path: &str) -> Result<String> {
    fs::read_to_string(path).map_err(|error| anyhow::anyhow!(fs_error("read_text", path, &error)))
}

/// Writes a string to a file, creating it if it doesn't exist
pub fn write_text(path: &str, contents: &str) -> Result<()> {
    fs::write(path, contents.as_bytes())
        .map_err(|error| anyhow::anyhow!(fs_error("write_text", path, &error)))
}

/// Creates a new directory at the specified path
pub fn create_dir(path: &str) -> Result<()> {
    fs::create_dir(path).map_err(|error| anyhow::anyhow!(fs_error("create_dir", path, &error)))
}

/// Recursively creates a directory and all of its parent components if they are missing
pub fn ensure_dir(path: &str) -> Result<()> {
    fs::create_dir_all(path).map_err(|error| anyhow::anyhow!(fs_error("ensure_dir", path, &error)))
}

/// Removes a file or directory at the specified path
/// If the path is a directory, it will be removed recursively
pub fn remove(path: &str) -> Result<()> {
    let std_path = StdPath::new(path);
    if std_path.is_dir() {
        fs::remove_dir_all(std_path)
            .map_err(|error| anyhow::anyhow!(fs_error("remove", path, &error)))
    } else {
        fs::remove_file(std_path).map_err(|error| anyhow::anyhow!(fs_error("remove", path, &error)))
    }
}

/// Checks if a path exists
pub fn exists(path: &str) -> bool {
    StdPath::new(path).exists()
}

/// Lists all entries in a directory (non-recursive)
pub fn list_dir(path: &str) -> Result<Vec<String>> {
    let mut entries = Vec::new();
    let dir =
        fs::read_dir(path).map_err(|error| anyhow::anyhow!(fs_error("list_dir", path, &error)))?;
    for entry in dir {
        match entry {
            Ok(dir_entry) => {
                entries.push(dir_entry.path().to_string_lossy().into_owned());
            }
            Err(error) => return Err(anyhow::anyhow!(fs_error("list_dir", path, &error))),
        }
    }
    entries.sort();
    Ok(entries)
}

/// Walks a directory recursively, returning all file and directory paths
pub fn walk(path: &str) -> Result<Vec<String>> {
    let mut entries = Vec::new();
    for entry in WalkDir::new(path) {
        match entry {
            Ok(dir_entry) => {
                if dir_entry.depth() == 0 {
                    continue;
                }
                entries.push(dir_entry.path().to_string_lossy().into_owned());
            }
            Err(error) => return Err(anyhow::anyhow!("walk failed on path '{}': {}", path, error)),
        }
    }
    entries.sort();
    Ok(entries)
}

/// Renames or moves a file or directory
pub fn rename(source: &str, target: &str) -> Result<()> {
    fs::rename(source, target).map_err(|error| {
        anyhow::anyhow!("rename failed from '{}' to '{}': {}", source, target, error)
    })
}

/// File metadata information
#[derive(Debug, Clone)]
pub struct FileInfo {
    pub is_file: bool,
    pub is_dir: bool,
    pub size: u64,
    pub readonly: bool,
}

/// Gets metadata information about a file or directory
pub fn stat(path: &str) -> Result<FileInfo> {
    let metadata =
        fs::metadata(path).map_err(|error| anyhow::anyhow!(fs_error("stat", path, &error)))?;

    Ok(FileInfo {
        is_file: metadata.is_file(),
        is_dir: metadata.is_dir(),
        size: metadata.len(),
        readonly: metadata.permissions().readonly(),
    })
}
