use super::StdFunctionKind;

pub(super) const fn function_doc(kind: StdFunctionKind) -> &'static str {
    match kind {
        StdFunctionKind::Print => "Write the string representation of a value to stderr.",
        StdFunctionKind::Length => "Return the number of elements in a String, List, or Dict.",
        StdFunctionKind::Exit => "Exit the program with the given exit code.",
        StdFunctionKind::Delete => "Remove an entry from a Dict and return the modified Dict.",
        StdFunctionKind::Clear => "Remove all entries from a Dict and return the empty Dict.",
        StdFunctionKind::Max => "Return the maximum of two Int or Float values.",
        StdFunctionKind::Min => "Return the minimum of two Int or Float values.",
        StdFunctionKind::Append => "Add a value to the end of a List and return the modified List.",
        StdFunctionKind::Assert => {
            "Assert that a condition holds; optionally provide a failure message."
        }
        StdFunctionKind::AssertEq => "Assert that two values are equal.",
        StdFunctionKind::AssertNe => "Assert that two values are not equal.",
        StdFunctionKind::AssertFail => "Unconditionally fail with the provided message.",
        StdFunctionKind::UtilToString => "Convert a value to its string representation.",
        StdFunctionKind::UtilClampInt => "Clamp an integer between the provided bounds.",
        StdFunctionKind::StringIndexOf => {
            "Find the first occurrence of a substring, returning its index or -1."
        }
        StdFunctionKind::StringSplit => "Split a string by a delimiter into a list of substrings.",
        StdFunctionKind::StringContains => {
            "Return true if the string contains the given substring."
        }
        StdFunctionKind::StringReplace => {
            "Replace all occurrences of a substring with another string."
        }
        StdFunctionKind::EnvGet => "Lookup an environment variable (returns Nil if unset).",
        StdFunctionKind::EnvGetOr => "Lookup an environment variable or return a default value.",
        StdFunctionKind::EnvHas => "Return true if an environment variable is set.",
        StdFunctionKind::EnvRequire => "Get an environment variable, raising if it is missing.",
        StdFunctionKind::EnvSet => "Set an environment variable for child processes.",
        StdFunctionKind::EnvUnset => "Unset an environment variable.",
        StdFunctionKind::EnvVars => "Return a dictionary of the current environment.",
        StdFunctionKind::EnvCwd => "Return the current working directory.",
        StdFunctionKind::EnvSetCwd => {
            "Change the current working directory for subsequent processes."
        }
        StdFunctionKind::EnvTempDir => "Return the path to a writable temporary directory.",
        StdFunctionKind::EnvHomeDir => "Return the current user's home directory.",
        StdFunctionKind::EnvConfigDir => {
            "Return a configuration directory path for the current platform."
        }
        StdFunctionKind::FsReadText => "Read an entire text file into a string.",
        StdFunctionKind::FsWriteText => "Write a string to a file, replacing existing contents.",
        StdFunctionKind::FsWriteTextAtomic => {
            "Write text to a file atomically, preserving existing data on failure."
        }
        StdFunctionKind::FsCreateDir => "Create a directory and intermediate folders as needed.",
        StdFunctionKind::FsEnsureDir => "Ensure a directory exists, creating it if necessary.",
        StdFunctionKind::FsEnsureParent => "Ensure the parent directory of a path exists.",
        StdFunctionKind::FsRemove => "Remove a file or directory recursively.",
        StdFunctionKind::FsExists => "Return true if a path exists.",
        StdFunctionKind::FsIsDir => "Return true if the path is a directory.",
        StdFunctionKind::FsListDir => "List entries in a directory.",
        StdFunctionKind::FsWalk => "Walk a directory tree depth-first.",
        StdFunctionKind::FsSize => "Return the size of a file in bytes.",
        StdFunctionKind::FsModified => "Return the last modified timestamp for a path.",
        StdFunctionKind::FsIsReadonly => "Return true if the path is read-only.",
        StdFunctionKind::FsIsSymlink => "Return true if the path is a symbolic link.",
        StdFunctionKind::FsGlob => "Return paths matching a glob pattern.",
        StdFunctionKind::FsMetadata => "Return metadata for a path.",
        StdFunctionKind::FsPermissions => "Return the permissions for a path.",
        StdFunctionKind::PathJoin => "Join multiple path segments.",
        StdFunctionKind::PathComponents => "Split a path into components.",
        StdFunctionKind::PathDirname => "Return the directory portion of a path.",
        StdFunctionKind::PathBasename => "Return the final component of a path.",
        StdFunctionKind::PathExtension => "Return the extension of a path if present.",
        StdFunctionKind::PathSetExtension => "Replace the extension on a path.",
        StdFunctionKind::PathStripExtension => "Remove the extension from a path.",
        StdFunctionKind::PathNormalize => "Normalize a path by removing redundant segments.",
        StdFunctionKind::PathAbsolute => "Convert a path to an absolute path.",
        StdFunctionKind::PathRelative => "Compute a relative path between two paths.",
        StdFunctionKind::PathIsAbsolute => "Return true if the path is absolute.",
        StdFunctionKind::PathSeparator => "Return the platform path separator.",
        StdFunctionKind::CliCapture => "Run a command and capture stdout/stderr with options.",
        StdFunctionKind::CliArgs => "Return the current process's command-line arguments.",
        StdFunctionKind::CliParse => "Parse CLI arguments into structured data based on a spec.",
        StdFunctionKind::ProcessRun => {
            "Run a command to completion, returning its status and output."
        }
        StdFunctionKind::ProcessSpawn => "Spawn a process and return a handle for interaction.",
        StdFunctionKind::ProcessKill => "Send a termination signal to a process.",
        StdFunctionKind::ProcessReadStdout => "Read stdout from a spawned process.",
        StdFunctionKind::ProcessReadStderr => "Read stderr from a spawned process.",
        StdFunctionKind::ProcessWriteStdin => "Write to a spawned process's stdin.",
        StdFunctionKind::ProcessCloseStdin => "Close the stdin handle of a spawned process.",
        StdFunctionKind::ProcessClose => "Close all handles associated with a process.",
        StdFunctionKind::AssertSnapshot => {
            "Compare actual text against a stored snapshot, with optional hint."
        }
        StdFunctionKind::AssertEmpty => "Assert that a string or collection is empty.",
    }
}
