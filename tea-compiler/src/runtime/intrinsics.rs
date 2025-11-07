/// Native intrinsic functions using macro-based registration.
///
/// This module provides a declarative way to define intrinsics with minimal boilerplate.
/// Each intrinsic is defined once with all its metadata and implementation.
use anyhow::Result;

use super::intrinsics_impl::*;
use super::value::Value;
use super::vm::Vm;
use crate::stdlib::{StdArity, StdFunctionKind, StdType};

/// Macro to declaratively define all intrinsics in one place.
///
/// This eliminates the need for:
/// - Separate enum variant declarations
/// - Manual from_name() implementations
/// - Manual name() implementations
/// - Manual all() iterators
/// - Duplicate StdFunctionKind enum
///
/// Each intrinsic is defined with:
/// - name: The function name exposed to Tea (without __intrinsic_ prefix)
/// - kind: The StdFunctionKind variant
/// - arity: Function arity
/// - params: Parameter types
/// - return_type: Return type
/// - impl_fn: Implementation function reference
macro_rules! define_intrinsics {
    (
        $(
            {
                name: $name:literal,
                kind: $kind:ident,
                arity: $arity:expr,
                params: [$($param:expr),*],
                return_type: $return_type:expr,
                impl_fn: $impl_fn:expr
            }
        ),* $(,)?
    ) => {
        /// Intrinsic function metadata and dispatch
        pub struct IntrinsicDef {
            pub name: &'static str,
            pub kind: StdFunctionKind,
            pub arity: StdArity,
            pub params: &'static [StdType],
            pub return_type: StdType,
            pub impl_fn: fn(&mut Vm, Vec<Value>) -> Result<Value>,
        }

        /// Registry of all intrinsics
        pub static INTRINSICS: &[IntrinsicDef] = &[
            $(
                IntrinsicDef {
                    name: $name,
                    kind: StdFunctionKind::$kind,
                    arity: $arity,
                    params: &[$($param),*],
                    return_type: $return_type,
                    impl_fn: $impl_fn,
                },
            )*
        ];

        /// Get intrinsic by StdFunctionKind
        pub fn get_intrinsic(kind: StdFunctionKind) -> Option<&'static IntrinsicDef> {
            INTRINSICS.iter().find(|i| i.kind == kind)
        }

        /// Get intrinsic by name
        #[allow(dead_code)]
        pub fn get_intrinsic_by_name(name: &str) -> Option<&'static IntrinsicDef> {
            INTRINSICS.iter().find(|i| i.name == name)
        }
    };
}

// Define all intrinsics in one place
define_intrinsics! {
    // ===== Conversion =====
    {
        name: "to_string",
        kind: UtilToString,
        arity: StdArity::Exact(1),
        params: [StdType::Any],
        return_type: StdType::String,
        impl_fn: util::to_string
    },

    // ===== String Utilities =====
    {
        name: "string_index_of",
        kind: StringIndexOf,
        arity: StdArity::Exact(2),
        params: [StdType::String, StdType::String],
        return_type: StdType::Int,
        impl_fn: string::index_of
    },
    {
        name: "string_split",
        kind: StringSplit,
        arity: StdArity::Exact(2),
        params: [StdType::String, StdType::String],
        return_type: StdType::List,
        impl_fn: string::split
    },
    {
        name: "string_contains",
        kind: StringContains,
        arity: StdArity::Exact(2),
        params: [StdType::String, StdType::String],
        return_type: StdType::Bool,
        impl_fn: string::contains
    },
    {
        name: "string_replace",
        kind: StringReplace,
        arity: StdArity::Exact(3),
        params: [StdType::String, StdType::String, StdType::String],
        return_type: StdType::String,
        impl_fn: string::replace
    },

    // ===== Assertions =====
    {
        name: "fail",
        kind: AssertFail,
        arity: StdArity::Exact(1),
        params: [StdType::String],
        return_type: StdType::Void,
        impl_fn: assert::fail
    },
    {
        name: "assert_snapshot",
        kind: AssertSnapshot,
        arity: StdArity::Range { min: 2, max: Some(3) },
        params: [StdType::String, StdType::String, StdType::String],
        return_type: StdType::Void,
        impl_fn: assert::snapshot
    },

    // ===== Environment =====
    {
        name: "env_get",
        kind: EnvGet,
        arity: StdArity::Exact(1),
        params: [StdType::String],
        return_type: StdType::String,
        impl_fn: env::get
    },
    {
        name: "env_set",
        kind: EnvSet,
        arity: StdArity::Exact(2),
        params: [StdType::String, StdType::String],
        return_type: StdType::Void,
        impl_fn: env::set
    },
    {
        name: "env_unset",
        kind: EnvUnset,
        arity: StdArity::Exact(1),
        params: [StdType::String],
        return_type: StdType::Void,
        impl_fn: env::unset
    },
    {
        name: "env_has",
        kind: EnvHas,
        arity: StdArity::Exact(1),
        params: [StdType::String],
        return_type: StdType::Bool,
        impl_fn: env::has
    },
    {
        name: "env_vars",
        kind: EnvVars,
        arity: StdArity::Exact(0),
        params: [],
        return_type: StdType::Dict,
        impl_fn: env::vars
    },
    {
        name: "env_cwd",
        kind: EnvCwd,
        arity: StdArity::Exact(0),
        params: [],
        return_type: StdType::String,
        impl_fn: env::cwd
    },

    // ===== Filesystem =====
    {
        name: "fs_read_text",
        kind: FsReadText,
        arity: StdArity::Exact(1),
        params: [StdType::String],
        return_type: StdType::String,
        impl_fn: fs::read_text
    },
    {
        name: "fs_write_text",
        kind: FsWriteText,
        arity: StdArity::Exact(2),
        params: [StdType::String, StdType::String],
        return_type: StdType::Void,
        impl_fn: fs::write_text
    },
    {
        name: "fs_create_dir",
        kind: FsCreateDir,
        arity: StdArity::Exact(1),
        params: [StdType::String],
        return_type: StdType::Void,
        impl_fn: fs::create_dir
    },
    {
        name: "fs_ensure_dir",
        kind: FsEnsureDir,
        arity: StdArity::Exact(1),
        params: [StdType::String],
        return_type: StdType::Void,
        impl_fn: fs::ensure_dir
    },
    {
        name: "fs_remove",
        kind: FsRemove,
        arity: StdArity::Exact(1),
        params: [StdType::String],
        return_type: StdType::Void,
        impl_fn: fs::remove
    },
    {
        name: "fs_exists",
        kind: FsExists,
        arity: StdArity::Exact(1),
        params: [StdType::String],
        return_type: StdType::Bool,
        impl_fn: fs::exists
    },
    {
        name: "fs_list_dir",
        kind: FsListDir,
        arity: StdArity::Exact(1),
        params: [StdType::String],
        return_type: StdType::List,
        impl_fn: fs::list_dir
    },
    {
        name: "fs_walk",
        kind: FsWalk,
        arity: StdArity::Exact(1),
        params: [StdType::String],
        return_type: StdType::List,
        impl_fn: fs::walk
    },

    // ===== Path =====
    {
        name: "path_join",
        kind: PathJoin,
        arity: StdArity::Exact(1),
        params: [StdType::List],
        return_type: StdType::String,
        impl_fn: path::join
    },
    {
        name: "path_components",
        kind: PathComponents,
        arity: StdArity::Exact(1),
        params: [StdType::String],
        return_type: StdType::List,
        impl_fn: path::components
    },
    {
        name: "path_dirname",
        kind: PathDirname,
        arity: StdArity::Exact(1),
        params: [StdType::String],
        return_type: StdType::String,
        impl_fn: path::dirname
    },
    {
        name: "path_basename",
        kind: PathBasename,
        arity: StdArity::Exact(1),
        params: [StdType::String],
        return_type: StdType::String,
        impl_fn: path::basename
    },
    {
        name: "path_extension",
        kind: PathExtension,
        arity: StdArity::Exact(1),
        params: [StdType::String],
        return_type: StdType::String,
        impl_fn: path::extension
    },
    {
        name: "path_normalize",
        kind: PathNormalize,
        arity: StdArity::Exact(1),
        params: [StdType::String],
        return_type: StdType::String,
        impl_fn: path::normalize
    },
    {
        name: "path_absolute",
        kind: PathAbsolute,
        arity: StdArity::Range { min: 1, max: Some(2) },
        params: [StdType::String, StdType::String],
        return_type: StdType::String,
        impl_fn: path::absolute
    },
    {
        name: "path_relative",
        kind: PathRelative,
        arity: StdArity::Exact(2),
        params: [StdType::String, StdType::String],
        return_type: StdType::String,
        impl_fn: path::relative
    },
    {
        name: "path_separator",
        kind: PathSeparator,
        arity: StdArity::Exact(0),
        params: [],
        return_type: StdType::String,
        impl_fn: path::separator
    },
}

// Keep the old Intrinsic enum for backward compatibility with AOT
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
    EnvUnset,
    EnvHas,
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
    PathNormalize,
    PathAbsolute,
    PathRelative,
    PathSeparator,
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
            "env_unset" => Self::EnvUnset,
            "env_has" => Self::EnvHas,
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
            "path_normalize" => Self::PathNormalize,
            "path_absolute" => Self::PathAbsolute,
            "path_relative" => Self::PathRelative,
            "path_separator" => Self::PathSeparator,
            _ => return None,
        })
    }
}
