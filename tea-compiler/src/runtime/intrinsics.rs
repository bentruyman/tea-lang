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
    EnvSetCwd,
    EnvTempDir,
    EnvHomeDir,
    EnvConfigDir,

    // Filesystem
    FsReadText,
    FsWriteText,
    FsWriteTextAtomic,
    FsCreateDir,
    FsRemove,
    FsExists,
    FsIsDir,
    FsIsSymlink,
    FsSize,
    FsModified,
    FsPermissions,
    FsIsReadonly,
    FsListDir,
    FsWalk,
    FsGlob,
    FsMetadata,

    // Path
    PathJoin,
    PathComponents,
    PathDirname,
    PathBasename,
    PathExtension,
    PathSetExtension,
    PathStripExtension,
    PathNormalize,
    PathAbsolute,
    PathRelative,
    PathIsAbsolute,
    PathSeparator,

    // Process
    ProcessRun,
    ProcessSpawn,
    ProcessKill,
    ProcessReadStdout,
    ProcessReadStderr,
    ProcessWriteStdin,
    ProcessCloseStdin,
    ProcessClose,

    // CLI
    CliArgs,
    CliParse,
    CliCapture,
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
            "env_set_cwd" => Self::EnvSetCwd,
            "env_temp_dir" => Self::EnvTempDir,
            "env_home_dir" => Self::EnvHomeDir,
            "env_config_dir" => Self::EnvConfigDir,

            // Filesystem
            "fs_read_text" => Self::FsReadText,
            "fs_write_text" => Self::FsWriteText,
            "fs_write_text_atomic" => Self::FsWriteTextAtomic,
            "fs_create_dir" => Self::FsCreateDir,
            "fs_remove" => Self::FsRemove,
            "fs_exists" => Self::FsExists,
            "fs_is_dir" => Self::FsIsDir,
            "fs_is_symlink" => Self::FsIsSymlink,
            "fs_size" => Self::FsSize,
            "fs_modified" => Self::FsModified,
            "fs_permissions" => Self::FsPermissions,
            "fs_is_readonly" => Self::FsIsReadonly,
            "fs_list_dir" => Self::FsListDir,
            "fs_walk" => Self::FsWalk,
            "fs_glob" => Self::FsGlob,
            "fs_metadata" => Self::FsMetadata,

            // Path
            "path_join" => Self::PathJoin,
            "path_components" => Self::PathComponents,
            "path_dirname" => Self::PathDirname,
            "path_basename" => Self::PathBasename,
            "path_extension" => Self::PathExtension,
            "path_set_extension" => Self::PathSetExtension,
            "path_strip_extension" => Self::PathStripExtension,
            "path_normalize" => Self::PathNormalize,
            "path_absolute" => Self::PathAbsolute,
            "path_relative" => Self::PathRelative,
            "path_is_absolute" => Self::PathIsAbsolute,
            "path_separator" => Self::PathSeparator,

            // Process
            "process_run" => Self::ProcessRun,
            "process_spawn" => Self::ProcessSpawn,
            "process_kill" => Self::ProcessKill,
            "process_read_stdout" => Self::ProcessReadStdout,
            "process_read_stderr" => Self::ProcessReadStderr,
            "process_write_stdin" => Self::ProcessWriteStdin,
            "process_close_stdin" => Self::ProcessCloseStdin,
            "process_close" => Self::ProcessClose,

            // CLI
            "cli_args" => Self::CliArgs,
            "cli_parse" => Self::CliParse,
            "cli_capture" => Self::CliCapture,

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
            Self::EnvSetCwd => "__intrinsic_env_set_cwd",
            Self::EnvTempDir => "__intrinsic_env_temp_dir",
            Self::EnvHomeDir => "__intrinsic_env_home_dir",
            Self::EnvConfigDir => "__intrinsic_env_config_dir",

            // Filesystem
            Self::FsReadText => "__intrinsic_fs_read_text",
            Self::FsWriteText => "__intrinsic_fs_write_text",
            Self::FsWriteTextAtomic => "__intrinsic_fs_write_text_atomic",
            Self::FsCreateDir => "__intrinsic_fs_create_dir",
            Self::FsRemove => "__intrinsic_fs_remove",
            Self::FsExists => "__intrinsic_fs_exists",
            Self::FsIsDir => "__intrinsic_fs_is_dir",
            Self::FsIsSymlink => "__intrinsic_fs_is_symlink",
            Self::FsSize => "__intrinsic_fs_size",
            Self::FsModified => "__intrinsic_fs_modified",
            Self::FsPermissions => "__intrinsic_fs_permissions",
            Self::FsIsReadonly => "__intrinsic_fs_is_readonly",
            Self::FsListDir => "__intrinsic_fs_list_dir",
            Self::FsWalk => "__intrinsic_fs_walk",
            Self::FsGlob => "__intrinsic_fs_glob",
            Self::FsMetadata => "__intrinsic_fs_metadata",

            // Path
            Self::PathJoin => "__intrinsic_path_join",
            Self::PathComponents => "__intrinsic_path_components",
            Self::PathDirname => "__intrinsic_path_dirname",
            Self::PathBasename => "__intrinsic_path_basename",
            Self::PathExtension => "__intrinsic_path_extension",
            Self::PathSetExtension => "__intrinsic_path_set_extension",
            Self::PathStripExtension => "__intrinsic_path_strip_extension",
            Self::PathNormalize => "__intrinsic_path_normalize",
            Self::PathAbsolute => "__intrinsic_path_absolute",
            Self::PathRelative => "__intrinsic_path_relative",
            Self::PathIsAbsolute => "__intrinsic_path_is_absolute",
            Self::PathSeparator => "__intrinsic_path_separator",

            // Process
            Self::ProcessRun => "__intrinsic_process_run",
            Self::ProcessSpawn => "__intrinsic_process_spawn",
            Self::ProcessKill => "__intrinsic_process_kill",
            Self::ProcessReadStdout => "__intrinsic_process_read_stdout",
            Self::ProcessReadStderr => "__intrinsic_process_read_stderr",
            Self::ProcessWriteStdin => "__intrinsic_process_write_stdin",
            Self::ProcessCloseStdin => "__intrinsic_process_close_stdin",
            Self::ProcessClose => "__intrinsic_process_close",

            // CLI
            Self::CliArgs => "__intrinsic_cli_args",
            Self::CliParse => "__intrinsic_cli_parse",
            Self::CliCapture => "__intrinsic_cli_capture",
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
            EnvSetCwd,
            EnvTempDir,
            EnvHomeDir,
            EnvConfigDir,
            // Filesystem
            FsReadText,
            FsWriteText,
            FsWriteTextAtomic,
            FsCreateDir,
            FsRemove,
            FsExists,
            FsIsDir,
            FsIsSymlink,
            FsSize,
            FsModified,
            FsPermissions,
            FsIsReadonly,
            FsListDir,
            FsWalk,
            FsGlob,
            FsMetadata,
            // Path
            PathJoin,
            PathComponents,
            PathDirname,
            PathBasename,
            PathExtension,
            PathSetExtension,
            PathStripExtension,
            PathNormalize,
            PathAbsolute,
            PathRelative,
            PathIsAbsolute,
            PathSeparator,
            // Process
            ProcessRun,
            ProcessSpawn,
            ProcessKill,
            ProcessReadStdout,
            ProcessReadStderr,
            ProcessWriteStdin,
            ProcessCloseStdin,
            ProcessClose,
            // CLI
            CliArgs,
            CliParse,
            CliCapture,
        ]
        .into_iter()
    }
}
