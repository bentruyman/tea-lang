mod args;
mod assert;
mod builtins;
mod docs;
mod env;
mod fs;
mod intrinsics;
mod json;
mod path;
mod process;
mod regex;

use docs::function_doc;

pub use builtins::BUILTINS;

pub const SOURCE_STDLIB_MODULES: &[&str] = &[
    "std.args",
    "std.env",
    "std.fs",
    "std.parse",
    "std.path",
    "std.process",
    "std.regex",
    "std.string",
];

pub const REFERENCE_STDLIB_MODULES: &[&str] = &[
    "std.args",
    "std.assert",
    "std.env",
    "std.fs",
    "std.json",
    "std.parse",
    "std.path",
    "std.process",
    "std.regex",
    "std.string",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StdFunctionKind {
    Print,
    Println,
    Append,
    Length,
    Assert,
    AssertEq,
    AssertNe,
    AssertFail,
    UtilToString,
    ToString,
    TypeOf,
    Panic,
    Exit,
    Args,
    StringIndexOf,
    StringSplit,
    StringContains,
    StringReplace,
    StringToLower,
    StringToUpper,
    MathFloor,
    MathCeil,
    MathRound,
    MathAbs,
    MathSqrt,
    MathMin,
    MathMax,
    EnvGet,
    EnvGetOr,
    EnvHas,
    EnvRequire,
    EnvSet,
    EnvUnset,
    EnvVars,
    EnvCwd,
    EnvSetCwd,
    EnvTempDir,
    EnvHomeDir,
    EnvConfigDir,
    FsReadText,
    FsWriteText,
    FsCreateDir,
    FsEnsureDir,
    FsRemove,
    FsExists,
    FsListDir,
    FsWalk,
    FsGlob,
    FsCopy,
    PathJoin,
    PathComponents,
    PathDirname,
    PathBasename,
    PathExtension,
    PathNormalize,
    PathAbsolute,
    PathRelative,
    PathIsAbsolute,
    PathSeparator,
    FsRename,
    FsStat,
    AssertSnapshot,
    AssertEmpty,
    // Standard I/O
    ReadLine,
    ReadAll,
    Eprint,
    Eprintln,
    IsTty,
    // Process execution
    ProcessRun,
    ProcessSpawn,
    ProcessWait,
    ProcessKill,
    ProcessReadStdout,
    ProcessReadStderr,
    ProcessWriteStdin,
    ProcessCloseStdin,
    ProcessClose,
    // Args module
    ArgsAll,
    ArgsProgram,
    CliParse,
    // Regex module
    RegexCompile,
    RegexIsMatch,
    RegexFindAll,
    RegexCaptures,
    RegexReplace,
    RegexReplaceAll,
    RegexSplit,
    // JSON module
    JsonEncode,
    JsonDecode,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StdArity {
    Exact(usize),
    Range { min: usize, max: Option<usize> },
}

impl StdArity {
    pub fn allows(self, count: usize) -> bool {
        match self {
            StdArity::Exact(expected) => expected == count,
            StdArity::Range { min, max } => {
                if count < min {
                    return false;
                }
                if let Some(limit) = max {
                    if count > limit {
                        return false;
                    }
                }
                true
            }
        }
    }
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StdType {
    Any,
    Bool,
    Int,
    Float,
    String,
    List,
    Dict,
    Struct,
    Nil,
    Void,
}

#[derive(Clone, Copy, Debug)]
pub struct StdFunction {
    pub name: &'static str,
    pub kind: StdFunctionKind,
    pub arity: StdArity,
    pub params: &'static [StdType],
    pub return_type: StdType,
    pub docstring: &'static str,
}

pub struct StdModule {
    pub path: &'static str,
    pub functions: &'static [StdFunction],
    pub docstring: &'static str,
}

const fn std_function(
    name: &'static str,
    kind: StdFunctionKind,
    arity: StdArity,
    params: &'static [StdType],
    return_type: StdType,
) -> StdFunction {
    StdFunction {
        name,
        kind,
        arity,
        params,
        return_type,
        docstring: function_doc(kind),
    }
}

macro_rules! std_module {
    ($path:literal, $doc:literal, $functions:expr $(,)?) => {
        StdModule {
            path: $path,
            functions: $functions,
            docstring: $doc,
        }
    };
}
pub(crate) use std_module;

pub static MODULES: &[StdModule] = &[
    args::MODULE,
    assert::MODULE,
    env::MODULE,
    fs::MODULE,
    json::MODULE,
    path::MODULE,
    intrinsics::MODULE,
    process::MODULE,
    regex::MODULE,
];

pub fn find_module(path: &str) -> Option<&'static StdModule> {
    MODULES.iter().find(|module| module.path == path)
}

pub fn is_source_stdlib_module(path: &str) -> bool {
    SOURCE_STDLIB_MODULES.contains(&path)
}

pub fn builtin_kind(name: &str) -> Option<StdFunctionKind> {
    BUILTINS
        .iter()
        .find(|function| function.name == name)
        .map(|function| function.kind)
}

pub fn module_function_kind(module_path: &str, name: &str) -> Option<StdFunctionKind> {
    find_module(module_path)?
        .functions
        .iter()
        .find(|function| function.name == name)
        .map(|function| function.kind)
}

pub fn is_browser_safe_stdlib_module(path: &str) -> bool {
    matches!(
        path,
        "std.assert" | "std.intrinsics" | "std.json" | "std.string"
    )
}

pub fn is_browser_safe_function(kind: StdFunctionKind) -> bool {
    matches!(
        kind,
        StdFunctionKind::Print
            | StdFunctionKind::Println
            | StdFunctionKind::Append
            | StdFunctionKind::UtilToString
            | StdFunctionKind::ToString
            | StdFunctionKind::TypeOf
            | StdFunctionKind::Panic
            | StdFunctionKind::Length
            | StdFunctionKind::StringIndexOf
            | StdFunctionKind::StringSplit
            | StdFunctionKind::StringContains
            | StdFunctionKind::StringReplace
            | StdFunctionKind::StringToLower
            | StdFunctionKind::StringToUpper
            | StdFunctionKind::MathFloor
            | StdFunctionKind::MathCeil
            | StdFunctionKind::MathRound
            | StdFunctionKind::MathAbs
            | StdFunctionKind::MathSqrt
            | StdFunctionKind::MathMin
            | StdFunctionKind::MathMax
            | StdFunctionKind::Assert
            | StdFunctionKind::AssertEq
            | StdFunctionKind::AssertNe
            | StdFunctionKind::JsonEncode
            | StdFunctionKind::JsonDecode
    )
}

pub fn module_for_function(name: &str) -> Option<&'static str> {
    MODULES.iter().find_map(|module| {
        module
            .functions
            .iter()
            .find(|function| function.name == name)
            .map(|_| module.path)
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn stdlib_modules_exist() {
        // Just verify we can access the MODULES array
        assert!(!super::MODULES.is_empty(), "stdlib should have modules");
    }
}
