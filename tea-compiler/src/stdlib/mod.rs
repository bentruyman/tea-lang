mod assert;
mod builtins;
mod docs;
mod env;
mod fs;
mod intrinsics;
mod path;

use docs::function_doc;

pub use builtins::BUILTINS;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StdFunctionKind {
    Print,
    Println,
    Length,
    Exit,
    Delete,
    Clear,
    Max,
    Min,
    Append,
    Assert,
    AssertEq,
    AssertNe,
    AssertFail,
    UtilToString,
    ToString,
    TypeOf,
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
    EnvHas,
    EnvSet,
    EnvUnset,
    EnvVars,
    EnvCwd,
    FsReadText,
    FsWriteText,
    FsCreateDir,
    FsEnsureDir,
    FsRemove,
    FsExists,
    FsListDir,
    FsWalk,
    PathJoin,
    PathComponents,
    PathDirname,
    PathBasename,
    PathExtension,
    PathNormalize,
    PathAbsolute,
    PathRelative,
    PathSeparator,
    FsRename,
    FsStat,
    AssertSnapshot,
    AssertEmpty,
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
    assert::MODULE,
    env::MODULE,
    fs::MODULE,
    path::MODULE,
    intrinsics::MODULE,
];

pub fn find_module(path: &str) -> Option<&'static StdModule> {
    MODULES.iter().find(|module| module.path == path)
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
