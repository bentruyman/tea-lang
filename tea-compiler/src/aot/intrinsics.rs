/// Intrinsic function enum for AOT compilation.
///
/// This enum maps StdFunctionKind values to compile-time code generation.
/// Each variant corresponds to a standard library function that requires
/// special handling during LLVM IR generation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Intrinsic {
    ToString,
    StringIndexOf,
    StringSplit,
    StringContains,
    StringReplace,
    Fail,
    AssertSnapshot,
    EnvGet,
    EnvSet,
    EnvVars,
    EnvCwd,
    FsReadText,
    FsWriteText,
    FsCreateDir,
    FsRemove,
    FsExists,
    FsListDir,
    FsWalk,
    PathJoin,
    PathComponents,
    PathDirname,
    PathBasename,
    PathExtension,
}

impl Intrinsic {
    /// Parse an intrinsic from its Tea function name (with __intrinsic_ prefix)
    pub fn from_name(name: &str) -> Option<Self> {
        let name = name.strip_prefix("__intrinsic_")?;

        Some(match name {
            "to_string" => Self::ToString,
            "string_index_of" => Self::StringIndexOf,
            "string_split" => Self::StringSplit,
            "string_contains" => Self::StringContains,
            "string_replace" => Self::StringReplace,
            "fail" => Self::Fail,
            "assert_snapshot" => Self::AssertSnapshot,
            "env_get" => Self::EnvGet,
            "env_set" => Self::EnvSet,
            "env_vars" => Self::EnvVars,
            "env_cwd" => Self::EnvCwd,
            "fs_read_text" => Self::FsReadText,
            "fs_write_text" => Self::FsWriteText,
            "fs_create_dir" => Self::FsCreateDir,
            "fs_remove" => Self::FsRemove,
            "fs_exists" => Self::FsExists,
            "fs_list_dir" => Self::FsListDir,
            "fs_walk" => Self::FsWalk,
            "path_join" => Self::PathJoin,
            "path_components" => Self::PathComponents,
            "path_dirname" => Self::PathDirname,
            "path_basename" => Self::PathBasename,
            "path_extension" => Self::PathExtension,
            _ => return None,
        })
    }
}
