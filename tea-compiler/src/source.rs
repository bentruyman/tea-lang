use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SourceId(pub u32);

#[derive(Debug, Clone)]
pub struct SourceFile {
    pub id: SourceId,
    pub path: PathBuf,
    pub contents: String,
}

impl SourceFile {
    pub fn new(id: SourceId, path: PathBuf, contents: String) -> Self {
        Self { id, path, contents }
    }
}
