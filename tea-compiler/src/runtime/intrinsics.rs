/// Native intrinsic functions that provide low-level functionality.
///
/// These functions are implemented in Rust and exposed to Tea code with the
/// `__intrinsic_` prefix. They form the minimal native surface that the
/// Tea standard library is built upon.

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Intrinsic {
    // Type predicates
    IsNil,
    IsBool,
    IsInt,
    IsFloat,
    IsString,
    IsList,
    IsStruct,
    IsError,

    // Conversion
    ToString,

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
    FsReadBytes,
    FsWriteBytes,
    FsWriteBytesAtomic,
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
    FsOpenRead,
    FsReadChunk,
    FsClose,

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

    // I/O
    IoReadLine,
    IoReadAll,
    IoReadBytes,
    IoWrite,
    IoWriteErr,
    IoFlush,

    // Process
    ProcessRun,
    ProcessSpawn,
    ProcessWait,
    ProcessKill,
    ProcessReadStdout,
    ProcessReadStderr,
    ProcessWriteStdin,
    ProcessCloseStdin,
    ProcessClose,

    // Codecs
    JsonEncode,
    JsonDecode,
    YamlEncode,
    YamlDecode,

    // CLI
    CliArgs,
    CliParse,
    CliCapture,
}

impl Intrinsic {
    /// Parse an intrinsic from its Tea function name (with __intrinsic_ prefix)
    pub fn from_name(name: &str) -> Option<Self> {
        let name = name.strip_prefix("__intrinsic_")?;

        Some(match name {
            // Type predicates
            "is_nil" => Self::IsNil,
            "is_bool" => Self::IsBool,
            "is_int" => Self::IsInt,
            "is_float" => Self::IsFloat,
            "is_string" => Self::IsString,
            "is_list" => Self::IsList,
            "is_struct" => Self::IsStruct,
            "is_error" => Self::IsError,

            // Conversion
            "to_string" => Self::ToString,

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
            "fs_read_bytes" => Self::FsReadBytes,
            "fs_write_bytes" => Self::FsWriteBytes,
            "fs_write_bytes_atomic" => Self::FsWriteBytesAtomic,
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
            "fs_open_read" => Self::FsOpenRead,
            "fs_read_chunk" => Self::FsReadChunk,
            "fs_close" => Self::FsClose,

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

            // I/O
            "io_read_line" => Self::IoReadLine,
            "io_read_all" => Self::IoReadAll,
            "io_read_bytes" => Self::IoReadBytes,
            "io_write" => Self::IoWrite,
            "io_write_err" => Self::IoWriteErr,
            "io_flush" => Self::IoFlush,

            // Process
            "process_run" => Self::ProcessRun,
            "process_spawn" => Self::ProcessSpawn,
            "process_wait" => Self::ProcessWait,
            "process_kill" => Self::ProcessKill,
            "process_read_stdout" => Self::ProcessReadStdout,
            "process_read_stderr" => Self::ProcessReadStderr,
            "process_write_stdin" => Self::ProcessWriteStdin,
            "process_close_stdin" => Self::ProcessCloseStdin,
            "process_close" => Self::ProcessClose,

            // Codecs
            "json_encode" => Self::JsonEncode,
            "json_decode" => Self::JsonDecode,
            "yaml_encode" => Self::YamlEncode,
            "yaml_decode" => Self::YamlDecode,

            // CLI
            "cli_args" => Self::CliArgs,
            "cli_parse" => Self::CliParse,
            "cli_capture" => Self::CliCapture,

            _ => return None,
        })
    }

    /// Get the canonical Tea function name for this intrinsic
    pub fn name(self) -> &'static str {
        match self {
            // Type predicates
            Self::IsNil => "__intrinsic_is_nil",
            Self::IsBool => "__intrinsic_is_bool",
            Self::IsInt => "__intrinsic_is_int",
            Self::IsFloat => "__intrinsic_is_float",
            Self::IsString => "__intrinsic_is_string",
            Self::IsList => "__intrinsic_is_list",
            Self::IsStruct => "__intrinsic_is_struct",
            Self::IsError => "__intrinsic_is_error",

            // Conversion
            Self::ToString => "__intrinsic_to_string",

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
            Self::FsReadBytes => "__intrinsic_fs_read_bytes",
            Self::FsWriteBytes => "__intrinsic_fs_write_bytes",
            Self::FsWriteBytesAtomic => "__intrinsic_fs_write_bytes_atomic",
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
            Self::FsOpenRead => "__intrinsic_fs_open_read",
            Self::FsReadChunk => "__intrinsic_fs_read_chunk",
            Self::FsClose => "__intrinsic_fs_close",

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

            // I/O
            Self::IoReadLine => "__intrinsic_io_read_line",
            Self::IoReadAll => "__intrinsic_io_read_all",
            Self::IoReadBytes => "__intrinsic_io_read_bytes",
            Self::IoWrite => "__intrinsic_io_write",
            Self::IoWriteErr => "__intrinsic_io_write_err",
            Self::IoFlush => "__intrinsic_io_flush",

            // Process
            Self::ProcessRun => "__intrinsic_process_run",
            Self::ProcessSpawn => "__intrinsic_process_spawn",
            Self::ProcessWait => "__intrinsic_process_wait",
            Self::ProcessKill => "__intrinsic_process_kill",
            Self::ProcessReadStdout => "__intrinsic_process_read_stdout",
            Self::ProcessReadStderr => "__intrinsic_process_read_stderr",
            Self::ProcessWriteStdin => "__intrinsic_process_write_stdin",
            Self::ProcessCloseStdin => "__intrinsic_process_close_stdin",
            Self::ProcessClose => "__intrinsic_process_close",

            // Codecs
            Self::JsonEncode => "__intrinsic_json_encode",
            Self::JsonDecode => "__intrinsic_json_decode",
            Self::YamlEncode => "__intrinsic_yaml_encode",
            Self::YamlDecode => "__intrinsic_yaml_decode",

            // CLI
            Self::CliArgs => "__intrinsic_cli_args",
            Self::CliParse => "__intrinsic_cli_parse",
            Self::CliCapture => "__intrinsic_cli_capture",
        }
    }

    /// Returns an iterator over all intrinsic variants
    pub fn all() -> impl Iterator<Item = Self> {
        use Intrinsic::*;
        [
            // Type predicates
            IsNil,
            IsBool,
            IsInt,
            IsFloat,
            IsString,
            IsList,
            IsStruct,
            IsError,
            // Conversion
            ToString,
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
            FsReadBytes,
            FsWriteBytes,
            FsWriteBytesAtomic,
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
            FsOpenRead,
            FsReadChunk,
            FsClose,
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
            // I/O
            IoReadLine,
            IoReadAll,
            IoReadBytes,
            IoWrite,
            IoWriteErr,
            IoFlush,
            // Process
            ProcessRun,
            ProcessSpawn,
            ProcessWait,
            ProcessKill,
            ProcessReadStdout,
            ProcessReadStderr,
            ProcessWriteStdin,
            ProcessCloseStdin,
            ProcessClose,
            // Codecs
            JsonEncode,
            JsonDecode,
            YamlEncode,
            YamlDecode,
            // CLI
            CliArgs,
            CliParse,
            CliCapture,
        ]
        .into_iter()
    }
}
