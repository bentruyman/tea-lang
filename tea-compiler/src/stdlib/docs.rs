use super::StdFunctionKind;

pub(super) const fn function_doc(kind: StdFunctionKind) -> &'static str {
    match kind {
        StdFunctionKind::Print => "Write the string representation of a value to stdout.",
        StdFunctionKind::Println => {
            "Write the string representation of a value to stdout with a newline."
        }
        StdFunctionKind::Append => "Append a value to a mutable list in place.",
        StdFunctionKind::ToString => "Convert any value to its string representation.",
        StdFunctionKind::TypeOf => "Return the runtime type name of a value as a string.",
        StdFunctionKind::Panic => "Terminate the program immediately with an error message.",
        StdFunctionKind::Exit => "Exit the program with the specified exit code.",
        StdFunctionKind::Args => "Return command-line arguments as a list of strings.",
        StdFunctionKind::Length => "Return the number of elements in a String, List, or Dict.",
        StdFunctionKind::Assert => {
            "Assert that a condition holds; optionally provide a failure message."
        }
        StdFunctionKind::AssertEq => "Assert that two values are equal.",
        StdFunctionKind::AssertNe => "Assert that two values are not equal.",
        StdFunctionKind::AssertFail => "Unconditionally fail with the provided message.",
        StdFunctionKind::UtilToString => "Convert a value to its string representation.",
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
        StdFunctionKind::StringToLower => "Convert a string to lowercase.",
        StdFunctionKind::StringToUpper => "Convert a string to uppercase.",
        StdFunctionKind::MathFloor => "Round a float down to the nearest integer.",
        StdFunctionKind::MathCeil => "Round a float up to the nearest integer.",
        StdFunctionKind::MathRound => "Round a float to the nearest integer.",
        StdFunctionKind::MathAbs => "Return the absolute value of a float.",
        StdFunctionKind::MathSqrt => "Return the square root of a float.",
        StdFunctionKind::MathMin => "Return the minimum of two floats.",
        StdFunctionKind::MathMax => "Return the maximum of two floats.",
        StdFunctionKind::EnvGet => {
            "Lookup an environment variable, returning an empty string when unset."
        }
        StdFunctionKind::EnvGetOr => {
            "Lookup an environment variable, returning a fallback when unset."
        }
        StdFunctionKind::EnvHas => "Return true when the environment variable is set.",
        StdFunctionKind::EnvRequire => "Return an environment variable or fail if it is unset.",
        StdFunctionKind::EnvSet => "Set an environment variable for child processes.",
        StdFunctionKind::EnvUnset => "Unset an environment variable for child processes.",
        StdFunctionKind::EnvVars => "Return a dictionary of the current environment.",
        StdFunctionKind::EnvCwd => "Return the current working directory.",
        StdFunctionKind::EnvSetCwd => "Change the current working directory.",
        StdFunctionKind::EnvTempDir => "Return the platform temporary directory.",
        StdFunctionKind::EnvHomeDir => "Return the user home directory when available.",
        StdFunctionKind::EnvConfigDir => "Return the user configuration directory when available.",
        StdFunctionKind::FsReadText => "Read an entire text file into a string.",
        StdFunctionKind::FsReadBytes => "Read an entire file into a list of byte values.",
        StdFunctionKind::FsWriteText => "Write a string to a file, replacing existing contents.",
        StdFunctionKind::FsWriteBytes => {
            "Write a list of byte values to a file, replacing existing contents."
        }
        StdFunctionKind::FsWriteBytesAtomic => "Write a list of byte values to a file atomically.",
        StdFunctionKind::FsCreateDir => {
            "Create a directory. Parent directories must already exist."
        }
        StdFunctionKind::FsEnsureDir => "Create a directory and all missing parent folders.",
        StdFunctionKind::FsRemove => "Remove a file or directory recursively.",
        StdFunctionKind::FsExists => "Return true when a filesystem path exists.",
        StdFunctionKind::FsListDir => "List entries in a directory.",
        StdFunctionKind::FsWalk => "Recursively walk a directory and return all entries.",
        StdFunctionKind::FsGlob => "Return filesystem entries that match a glob pattern.",
        StdFunctionKind::FsCopy => "Copy a file to a new location.",
        StdFunctionKind::FsRename => "Rename or move a file or directory.",
        StdFunctionKind::FsStat => "Get metadata information about a file or directory.",
        StdFunctionKind::PathJoin => "Join multiple path segments.",
        StdFunctionKind::PathComponents => "Split a path into components.",
        StdFunctionKind::PathDirname => "Return the directory portion of a path.",
        StdFunctionKind::PathBasename => "Return the final component of a path.",
        StdFunctionKind::PathExtension => "Return the extension of a path if present.",
        StdFunctionKind::PathNormalize => "Normalize a path by resolving '.' and '..' segments.",
        StdFunctionKind::PathAbsolute => "Resolve a path to an absolute path.",
        StdFunctionKind::PathRelative => "Compute the relative path from one location to another.",
        StdFunctionKind::PathIsAbsolute => "Return true when the path is absolute.",
        StdFunctionKind::PathSeparator => "Return the platform-specific path separator.",
        StdFunctionKind::UrlEncodeComponent => "Percent-encode a URL component.",
        StdFunctionKind::UrlDecodeComponent => "Decode a percent-encoded URL component.",
        StdFunctionKind::UrlBuildQuery => {
            "Encode a dictionary of query parameters as a query string."
        }
        StdFunctionKind::UrlAppendQuery => "Append encoded query parameters to an absolute URL.",
        StdFunctionKind::UrlJoin => "Resolve a URL path against an absolute base URL.",
        StdFunctionKind::AssertSnapshot => {
            "Compare actual text against a stored snapshot, with optional hint."
        }
        StdFunctionKind::AssertEmpty => "Assert that a string or collection is empty.",
        // Standard I/O
        StdFunctionKind::ReadLine => "Read a single line from stdin (without trailing newline).",
        StdFunctionKind::ReadAll => "Read all content from stdin as a string.",
        StdFunctionKind::Eprint => "Print a value to stderr without a trailing newline.",
        StdFunctionKind::Eprintln => "Print a value to stderr with a trailing newline.",
        StdFunctionKind::IsTty => "Return true if stdin is connected to an interactive terminal.",
        // Process execution
        StdFunctionKind::ProcessRun => {
            "Run a command synchronously and return its exit code, stdout, and stderr."
        }
        StdFunctionKind::ProcessSpawn => "Start a command without waiting, returning a handle.",
        StdFunctionKind::ProcessWait => {
            "Wait for a spawned process to complete and return its result."
        }
        StdFunctionKind::ProcessKill => "Terminate a spawned process.",
        StdFunctionKind::ProcessReadStdout => "Read data from a spawned process's stdout.",
        StdFunctionKind::ProcessReadStderr => "Read data from a spawned process's stderr.",
        StdFunctionKind::ProcessWriteStdin => "Write data to a spawned process's stdin.",
        StdFunctionKind::ProcessCloseStdin => "Close a spawned process's stdin pipe.",
        StdFunctionKind::ProcessClose => {
            "Close a spawned process handle without waiting for completion."
        }
        // Args intrinsics
        StdFunctionKind::ArgsAll => "Return all command-line arguments as a list of strings.",
        StdFunctionKind::ArgsProgram => "Return the program name (argv[0] with path stripped).",
        StdFunctionKind::CliParse => {
            "Parse command-line arguments using a declarative command spec."
        }
        // Regex module
        StdFunctionKind::RegexCompile => "Compile a regex pattern and return a handle.",
        StdFunctionKind::RegexIsMatch => "Test if the pattern matches anywhere in the text.",
        StdFunctionKind::RegexFindAll => "Find all non-overlapping matches as a list of strings.",
        StdFunctionKind::RegexCaptures => "Get capture groups from the first match.",
        StdFunctionKind::RegexReplace => "Replace the first match with a replacement string.",
        StdFunctionKind::RegexReplaceAll => "Replace all matches with a replacement string.",
        StdFunctionKind::RegexSplit => "Split the text by the pattern into a list of strings.",
        // JSON module
        StdFunctionKind::JsonEncode => "Encode a value as a JSON string.",
        StdFunctionKind::JsonDecode => "Decode a JSON string into a Tea value.",
        // HTTP module
        StdFunctionKind::HttpSend => "Send an HTTP request and return a response dictionary.",
    }
}
