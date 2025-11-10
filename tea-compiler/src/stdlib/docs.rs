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
        StdFunctionKind::EnvGet => "Lookup an environment variable (returns Nil if unset).",
        StdFunctionKind::EnvHas => "Return true if an environment variable is set.",
        StdFunctionKind::EnvSet => "Set an environment variable for child processes.",
        StdFunctionKind::EnvUnset => "Unset an environment variable.",
        StdFunctionKind::EnvVars => "Return a dictionary of the current environment.",
        StdFunctionKind::EnvCwd => "Return the current working directory.",
        StdFunctionKind::FsReadText => "Read an entire text file into a string.",
        StdFunctionKind::FsWriteText => "Write a string to a file, replacing existing contents.",
        StdFunctionKind::FsCreateDir => "Create a directory and intermediate folders as needed.",
        StdFunctionKind::FsEnsureDir => "Ensure a directory exists, creating it if necessary.",
        StdFunctionKind::FsRemove => "Remove a file or directory recursively.",
        StdFunctionKind::FsExists => "Return true if a path exists.",
        StdFunctionKind::FsListDir => "List entries in a directory.",
        StdFunctionKind::FsWalk => "Walk a directory tree depth-first.",
        StdFunctionKind::FsRename => "Rename or move a file or directory.",
        StdFunctionKind::FsStat => "Get metadata information about a file or directory.",
        StdFunctionKind::PathJoin => "Join multiple path segments.",
        StdFunctionKind::PathComponents => "Split a path into components.",
        StdFunctionKind::PathDirname => "Return the directory portion of a path.",
        StdFunctionKind::PathBasename => "Return the final component of a path.",
        StdFunctionKind::PathExtension => "Return the extension of a path if present.",
        StdFunctionKind::PathNormalize => "Normalize a path by removing redundant segments.",
        StdFunctionKind::PathAbsolute => "Convert a path to an absolute path.",
        StdFunctionKind::PathRelative => "Compute a relative path between two paths.",
        StdFunctionKind::PathSeparator => "Return the platform path separator.",
        StdFunctionKind::AssertSnapshot => {
            "Compare actual text against a stored snapshot, with optional hint."
        }
        StdFunctionKind::AssertEmpty => "Assert that a string or collection is empty.",
    }
}
