use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};
use path_clean::PathClean;

use crate::stdlib;

pub trait ModuleLoader: Send + Sync {
    fn resolve_import(&self, base_path: &Path, import: &str) -> Result<Option<PathBuf>>;
    fn canonicalize(&self, path: &Path) -> Result<PathBuf>;
    fn load_module(&self, path: &Path) -> Result<String>;
}

fn normalized_path(path: &Path) -> PathBuf {
    path.clean()
}

fn resolved_path(base_path: &Path, import: &str) -> PathBuf {
    let base_dir = if base_path.is_dir() {
        base_path.to_path_buf()
    } else {
        base_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."))
    };

    let mut path = if Path::new(import).is_absolute() {
        PathBuf::from(import)
    } else {
        base_dir.join(import)
    };

    if path.extension().is_none() {
        path.set_extension("tea");
    }

    normalized_path(&path)
}

fn browser_stdlib_path(module_path: &str) -> Option<PathBuf> {
    match module_path {
        "std.string" => Some(PathBuf::from("/__tea_browser_stdlib/string/mod.tea")),
        _ => None,
    }
}

// Bundled installs do not have a repo-local `stdlib/` tree to import from.
fn embedded_source_stdlib_path(module_path: &str) -> Option<PathBuf> {
    let module_name = module_path.strip_prefix("std.")?;
    match module_name {
        "args" | "env" | "fs" | "parse" | "path" | "process" | "regex" | "string" => {
            Some(PathBuf::from(format!("/__tea_stdlib/{module_name}/mod.tea")))
        }
        _ => None,
    }
}

fn embedded_source_stdlib_contents(path: &Path) -> Option<&'static str> {
    match path.to_str()? {
        "/__tea_stdlib/args/mod.tea" => Some(include_str!("../../stdlib/args/mod.tea")),
        "/__tea_stdlib/env/mod.tea" => Some(include_str!("../../stdlib/env/mod.tea")),
        "/__tea_stdlib/fs/mod.tea" => Some(include_str!("../../stdlib/fs/mod.tea")),
        "/__tea_stdlib/parse/mod.tea" => Some(include_str!("../../stdlib/parse/mod.tea")),
        "/__tea_stdlib/path/mod.tea" => Some(include_str!("../../stdlib/path/mod.tea")),
        "/__tea_stdlib/process/mod.tea" => Some(include_str!("../../stdlib/process/mod.tea")),
        "/__tea_stdlib/regex/mod.tea" => Some(include_str!("../../stdlib/regex/mod.tea")),
        "/__tea_stdlib/string/mod.tea" => Some(include_str!("../../stdlib/string/mod.tea")),
        _ => None,
    }
}

pub struct InMemoryModuleLoader {
    files: HashMap<PathBuf, String>,
}

impl InMemoryModuleLoader {
    pub fn new(files: HashMap<PathBuf, String>) -> Self {
        let files = files
            .into_iter()
            .map(|(path, contents)| (normalized_path(&path), contents))
            .collect();
        Self { files }
    }

    pub fn with_browser_stdlib(mut self) -> Self {
        self.files.insert(
            PathBuf::from("/__tea_browser_stdlib/string/mod.tea"),
            include_str!("../../stdlib/string/mod.tea").to_string(),
        );
        self
    }
}

impl ModuleLoader for InMemoryModuleLoader {
    fn resolve_import(&self, base_path: &Path, import: &str) -> Result<Option<PathBuf>> {
        if let Some(path) = browser_stdlib_path(import) {
            return Ok(Some(path));
        }

        if import.starts_with("std.") || import.starts_with("support.") {
            return Ok(None);
        }

        Ok(Some(resolved_path(base_path, import)))
    }

    fn canonicalize(&self, path: &Path) -> Result<PathBuf> {
        Ok(normalized_path(path))
    }

    fn load_module(&self, path: &Path) -> Result<String> {
        let path = normalized_path(path);
        self.files
            .get(&path)
            .cloned()
            .ok_or_else(|| anyhow!("failed to read module at '{}'", path.display()))
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub struct NativeModuleLoader;

#[cfg(not(target_arch = "wasm32"))]
impl NativeModuleLoader {
    fn resolve_source_stdlib_module(&self, module_path: &str, base_path: &Path) -> Option<PathBuf> {
        if !stdlib::is_source_stdlib_module(module_path) {
            return None;
        }

        let module_name = module_path.strip_prefix("std.")?;
        let mut roots = Vec::new();

        let mut current = if base_path.is_dir() {
            Some(base_path)
        } else {
            base_path.parent()
        };
        while let Some(path) = current {
            roots.push(path.to_path_buf());
            current = path.parent();
        }

        if let Ok(cwd) = std::env::current_dir() {
            let mut current = Some(cwd.as_path());
            while let Some(path) = current {
                let path_buf = path.to_path_buf();
                if !roots.contains(&path_buf) {
                    roots.push(path_buf);
                }
                current = path.parent();
            }
        }

        for root in roots {
            let candidate = root.join("stdlib").join(module_name).join("mod.tea");
            if candidate.exists() {
                return Some(candidate);
            }
        }

        embedded_source_stdlib_path(module_path)
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl ModuleLoader for NativeModuleLoader {
    fn resolve_import(&self, base_path: &Path, import: &str) -> Result<Option<PathBuf>> {
        if let Some(path) = self.resolve_source_stdlib_module(import, base_path) {
            return Ok(Some(path));
        }

        if import.starts_with("std.") || import.starts_with("support.") {
            return Ok(None);
        }

        Ok(Some(resolved_path(base_path, import)))
    }

    fn canonicalize(&self, path: &Path) -> Result<PathBuf> {
        if embedded_source_stdlib_contents(path).is_some() {
            return Ok(normalized_path(path));
        }

        Ok(path.canonicalize()?)
    }

    fn load_module(&self, path: &Path) -> Result<String> {
        let path = normalized_path(path);
        if let Some(source) = embedded_source_stdlib_contents(&path) {
            return Ok(source.to_string());
        }

        Ok(std::fs::read_to_string(path)?)
    }
}
