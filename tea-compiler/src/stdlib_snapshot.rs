/// Standard library snapshot and loading infrastructure
///
/// The snapshot contains pre-compiled stdlib modules that are embedded
/// in the tea-cli binary. This allows Tea programs to use the stdlib
/// without requiring external files.
use std::collections::HashMap;

/// A compiled module in the snapshot
#[derive(Debug, Clone)]
pub struct SnapshotModule {
    /// Module path (e.g., "std.util")
    pub path: String,
    /// Compiled bytecode
    pub bytecode: Vec<u8>,
    /// Exported function signatures for typechecking
    pub exports: Vec<Export>,
}

/// An exported function from a module
#[derive(Debug, Clone)]
pub struct Export {
    /// Function name
    pub name: String,
    /// Parameter count (for arity checking)
    pub arity: usize,
    /// Whether it accepts variable arguments
    pub variadic: bool,
    /// Documentation string
    pub doc: Option<String>,
}

/// The stdlib snapshot
#[derive(Debug, Clone)]
pub struct Snapshot {
    /// Version of the stdlib
    pub version: String,
    /// Modules in the snapshot
    pub modules: HashMap<String, SnapshotModule>,
}

impl Snapshot {
    /// Create an empty snapshot
    pub fn new(version: String) -> Self {
        Self {
            version,
            modules: HashMap::new(),
        }
    }

    /// Add a module to the snapshot
    pub fn add_module(&mut self, module: SnapshotModule) {
        self.modules.insert(module.path.clone(), module);
    }

    /// Get a module by path
    pub fn get_module(&self, path: &str) -> Option<&SnapshotModule> {
        self.modules.get(path)
    }

    /// Serialize to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        // For now, use JSON serialization
        // TODO: Switch to a more efficient format (CBOR, MessagePack, or bincode)
        serde_json::to_vec(&self).expect("failed to serialize snapshot")
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(bytes)
    }
}

// Implement Serialize/Deserialize for snapshot types
impl serde::Serialize for Snapshot {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("Snapshot", 2)?;
        state.serialize_field("version", &self.version)?;
        state.serialize_field("modules", &self.modules)?;
        state.end()
    }
}

impl<'de> serde::Deserialize<'de> for Snapshot {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        struct SnapshotData {
            version: String,
            modules: HashMap<String, SnapshotModule>,
        }
        let data = SnapshotData::deserialize(deserializer)?;
        Ok(Snapshot {
            version: data.version,
            modules: data.modules,
        })
    }
}

impl serde::Serialize for SnapshotModule {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("SnapshotModule", 3)?;
        state.serialize_field("path", &self.path)?;
        state.serialize_field("bytecode", &self.bytecode)?;
        state.serialize_field("exports", &self.exports)?;
        state.end()
    }
}

impl<'de> serde::Deserialize<'de> for SnapshotModule {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        struct ModuleData {
            path: String,
            bytecode: Vec<u8>,
            exports: Vec<Export>,
        }
        let data = ModuleData::deserialize(deserializer)?;
        Ok(SnapshotModule {
            path: data.path,
            bytecode: data.bytecode,
            exports: data.exports,
        })
    }
}

impl serde::Serialize for Export {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("Export", 4)?;
        state.serialize_field("name", &self.name)?;
        state.serialize_field("arity", &self.arity)?;
        state.serialize_field("variadic", &self.variadic)?;
        state.serialize_field("doc", &self.doc)?;
        state.end()
    }
}

impl<'de> serde::Deserialize<'de> for Export {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        struct ExportData {
            name: String,
            arity: usize,
            variadic: bool,
            doc: Option<String>,
        }
        let data = ExportData::deserialize(deserializer)?;
        Ok(Export {
            name: data.name,
            arity: data.arity,
            variadic: data.variadic,
            doc: data.doc,
        })
    }
}

/// The embedded stdlib snapshot (generated by build.rs)
pub static EMBEDDED_SNAPSHOT: &[u8] = &[];

/// Load the embedded stdlib snapshot
pub fn load_embedded() -> Option<Snapshot> {
    if EMBEDDED_SNAPSHOT.is_empty() {
        return None;
    }
    Snapshot::from_bytes(EMBEDDED_SNAPSHOT).ok()
}
