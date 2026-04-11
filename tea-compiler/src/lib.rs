mod analysis;
mod ast;
mod browser;
mod compiler;
mod diagnostics;
mod expansion;
mod formatter;
mod lexer;
mod loader;
mod parser;
#[cfg(not(target_arch = "wasm32"))]
mod reference;
mod resolver;
mod source;
mod stdlib;
mod typechecker;

pub use crate::analysis::SemanticAnalysis;
pub use crate::ast::{
    AssignmentExpression, BinaryExpression, BinaryOperator, Block, BreakStatement, CallArgument,
    CallExpression, CatchArm, CatchClause, CatchHandler, CatchKind, ConditionalExpression,
    ConditionalKind, ConditionalStatement, ContinueStatement, DictEntry, DictLiteral,
    ErrorAnnotation, ErrorField, ErrorStatement, ErrorTypeSpecifier, ErrorVariant, Expression,
    ExpressionKind, ExpressionStatement, ForPattern, FunctionParameter, FunctionStatement,
    Identifier, IndexExpression, InterpolatedStringExpression, InterpolatedStringPart, LambdaBody,
    LambdaExpression, ListLiteral, Literal, LoopHeader, LoopKind, LoopStatement, MatchArm,
    MatchArmBlock, MatchExpression, MatchPattern, MatchStatement, MemberExpression, Module,
    RangeExpression, ReturnStatement, SourceSpan, Statement, StructField, StructStatement,
    ThrowStatement, TryExpression, TypeExpression, UnaryExpression, UnaryOperator, UseStatement,
    VarBinding, VarStatement,
};
pub use crate::compiler::{
    Compilation, CompileOptions, CompileTarget, Compiler, ParsedModule, ResolvedModule,
};
pub use crate::diagnostics::{Diagnostic, DiagnosticLevel, Diagnostics};
pub use crate::expansion::ExpandedModule;
pub use crate::formatter::format_source;
pub use crate::lexer::{Keyword, Lexer, Token, TokenKind};
pub use crate::loader::{InMemoryModuleLoader, ModuleLoader};
#[cfg(not(target_arch = "wasm32"))]
pub use crate::reference::{
    build_reference_manifest, ReferenceEntry, ReferenceEntryKind, ReferenceFunction,
    ReferenceManifest, BUILTIN_REFERENCE_SUMMARY,
};
pub use crate::resolver::{ModuleAliasBinding, Resolver, ResolverOutput};
pub use crate::source::{SourceFile, SourceId};
pub use crate::stdlib::{
    find_module as stdlib_find_module, StdFunction, StdModule, StdType, BUILTINS as STDLIB_BUILTINS,
};
pub use crate::typechecker::TypeChecker;

#[cfg(not(target_arch = "wasm32"))]
pub use crate::loader::NativeModuleLoader;

#[cfg(feature = "llvm-backend")]
pub mod aot;
