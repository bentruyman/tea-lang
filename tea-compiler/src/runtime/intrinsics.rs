/// Native intrinsic functions that provide low-level functionality.
///
/// These functions are implemented in Rust and exposed to Tea code with the
/// `__intrinsic_` prefix. They form the minimal native surface that the
/// Tea standard library is built upon.

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Intrinsic {
    // Conversion
    ToString,

    // String utilities
    StringIndexOf,
    StringSplit,
    StringContains,
    StringReplace,

    // Assertions
    Fail,
    AssertSnapshot,

    // Environment
    EnvGet,
    EnvSet,
    EnvUnset,
    EnvHas,
    EnvVars,
    EnvCwd,

    // Filesystem
    FsReadText,
    FsWriteText,
    FsCreateDir,
    FsRemove,
    FsExists,
    FsListDir,
    FsWalk,

    // Path
    PathJoin,
    PathComponents,
    PathDirname,
    PathBasename,
    PathExtension,
    PathNormalize,
    PathAbsolute,
    PathRelative,
    PathSeparator,
}

impl Intrinsic {
    /// Parse an intrinsic from its Tea function name (with __intrinsic_ prefix)
    #[allow(dead_code)]
    pub fn from_name(name: &str) -> Option<Self> {
        let name = name.strip_prefix("__intrinsic_")?;

        Some(match name {
            // Conversion
            "to_string" => Self::ToString,

            // String utilities
            "string_index_of" => Self::StringIndexOf,
            "string_split" => Self::StringSplit,
            "string_contains" => Self::StringContains,
            "string_replace" => Self::StringReplace,

            // Assertions
            "fail" => Self::Fail,
            "assert_snapshot" => Self::AssertSnapshot,

            // Environment
            "env_get" => Self::EnvGet,
            "env_set" => Self::EnvSet,
            "env_unset" => Self::EnvUnset,
            "env_has" => Self::EnvHas,
            "env_vars" => Self::EnvVars,
            "env_cwd" => Self::EnvCwd,

            // Filesystem
            "fs_read_text" => Self::FsReadText,
            "fs_write_text" => Self::FsWriteText,
            "fs_create_dir" => Self::FsCreateDir,
            "fs_remove" => Self::FsRemove,
            "fs_exists" => Self::FsExists,
            "fs_list_dir" => Self::FsListDir,
            "fs_walk" => Self::FsWalk,

            // Path
            "path_join" => Self::PathJoin,
            "path_components" => Self::PathComponents,
            "path_dirname" => Self::PathDirname,
            "path_basename" => Self::PathBasename,
            "path_extension" => Self::PathExtension,
            "path_normalize" => Self::PathNormalize,
            "path_absolute" => Self::PathAbsolute,
            "path_relative" => Self::PathRelative,
            "path_separator" => Self::PathSeparator,

            // Process

            // CLI
            _ => return None,
        })
    }

    /// Get the canonical Tea function name for this intrinsic
    #[allow(dead_code)]
    pub fn name(self) -> &'static str {
        match self {
            // Conversion
            Self::ToString => "__intrinsic_to_string",

            // String utilities
            Self::StringIndexOf => "__intrinsic_string_index_of",
            Self::StringSplit => "__intrinsic_string_split",
            Self::StringContains => "__intrinsic_string_contains",
            Self::StringReplace => "__intrinsic_string_replace",

            // Assertions
            Self::Fail => "__intrinsic_fail",
            Self::AssertSnapshot => "__intrinsic_assert_snapshot",

            // Environment
            Self::EnvGet => "__intrinsic_env_get",
            Self::EnvSet => "__intrinsic_env_set",
            Self::EnvUnset => "__intrinsic_env_unset",
            Self::EnvHas => "__intrinsic_env_has",
            Self::EnvVars => "__intrinsic_env_vars",
            Self::EnvCwd => "__intrinsic_env_cwd",

            // Filesystem
            Self::FsReadText => "__intrinsic_fs_read_text",
            Self::FsWriteText => "__intrinsic_fs_write_text",
            Self::FsCreateDir => "__intrinsic_fs_create_dir",
            Self::FsRemove => "__intrinsic_fs_remove",
            Self::FsExists => "__intrinsic_fs_exists",
            Self::FsListDir => "__intrinsic_fs_list_dir",
            Self::FsWalk => "__intrinsic_fs_walk",

            // Path
            Self::PathJoin => "__intrinsic_path_join",
            Self::PathComponents => "__intrinsic_path_components",
            Self::PathDirname => "__intrinsic_path_dirname",
            Self::PathBasename => "__intrinsic_path_basename",
            Self::PathExtension => "__intrinsic_path_extension",
            Self::PathNormalize => "__intrinsic_path_normalize",
            Self::PathAbsolute => "__intrinsic_path_absolute",
            Self::PathRelative => "__intrinsic_path_relative",
            Self::PathSeparator => "__intrinsic_path_separator",
            // Process

            // CLI
        }
    }

    /// Returns an iterator over all intrinsic variants
    #[allow(dead_code)]
    pub fn all() -> impl Iterator<Item = Self> {
        use Intrinsic::*;
        [
            // Conversion
            ToString,
            // String utilities
            StringIndexOf,
            StringSplit,
            StringContains,
            StringReplace,
            // Assertions
            Fail,
            AssertSnapshot,
            // Environment
            EnvGet,
            EnvSet,
            EnvUnset,
            EnvHas,
            EnvVars,
            EnvCwd,
            // Filesystem
            FsReadText,
            FsWriteText,
            FsCreateDir,
            FsRemove,
            FsExists,
            FsListDir,
            FsWalk,
            // Path
            PathJoin,
            PathComponents,
            PathDirname,
            PathBasename,
            PathExtension,
            PathNormalize,
            PathAbsolute,
            PathRelative,
            PathSeparator,
            // Process
            // CLI
        ]
        .into_iter()
    }
}
