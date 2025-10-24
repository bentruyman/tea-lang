mod debug;
mod assert;
mod cli;
mod env;
mod fs;
mod io;
mod json;
mod path;
mod print;
mod process;
mod util;
mod yaml;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StdFunctionKind {
    Print,
    Assert,
    AssertEq,
    AssertNe,
    AssertFail,
    UtilLen,
    UtilToString,
    UtilClampInt,
    UtilIsNil,
    UtilIsBool,
    UtilIsInt,
    UtilIsFloat,
    UtilIsString,
    UtilIsList,
    UtilIsStruct,
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
    FsWriteTextAtomic,
    FsReadBytes,
    FsWriteBytes,
    FsWriteBytesAtomic,
    FsCreateDir,
    FsEnsureDir,
    FsEnsureParent,
    FsRemove,
    FsExists,
    FsIsDir,
    FsListDir,
    FsWalk,
    FsSize,
    FsModified,
    FsIsReadonly,
    FsIsSymlink,
    FsGlob,
    FsMetadata,
    FsPermissions,
    FsOpenRead,
    FsReadChunk,
    FsClose,
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
    IoReadLine,
    IoReadAll,
    IoReadBytes,
    IoWrite,
    IoWriteErr,
    IoFlush,
    JsonEncode,
    JsonDecode,
    YamlEncode,
    YamlDecode,
    AssertSnapshot,
    AssertEmpty,
    CliCapture,
    CliArgs,
    CliParse,
    ProcessRun,
    ProcessSpawn,
    ProcessWait,
    ProcessKill,
    ProcessReadStdout,
    ProcessReadStderr,
    ProcessWriteStdin,
    ProcessCloseStdin,
    ProcessClose,
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
}

pub struct StdModule {
    pub path: &'static str,
    pub functions: &'static [StdFunction],
}

pub static MODULES: &[StdModule] = &[
    debug::MODULE,
    print::MODULE,
    assert::MODULE,
    util::MODULE,
    env::MODULE,
    fs::MODULE,
    path::MODULE,
    io::MODULE,
    cli::MODULE,
    process::MODULE,
    json::MODULE,
    yaml::MODULE,
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
