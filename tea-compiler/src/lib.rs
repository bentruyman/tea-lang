mod ast;
mod compiler;
mod diagnostics;
mod formatter;
mod lexer;
mod parser;
mod resolver;
mod source;
mod stdlib;
mod stdlib_snapshot;
mod typechecker;

pub use crate::ast::{
    AssignmentExpression, BinaryExpression, BinaryOperator, Block, BreakStatement, CallExpression,
    CatchArm, CatchClause, CatchHandler, CatchKind, ConditionalKind, ConditionalStatement,
    ContinueStatement, DictEntry, DictLiteral, ErrorAnnotation, ErrorField, ErrorStatement,
    ErrorTypeSpecifier, ErrorVariant, Expression, ExpressionKind, ExpressionStatement, ForPattern,
    FunctionParameter, FunctionStatement, Identifier, IndexExpression,
    InterpolatedStringExpression, InterpolatedStringPart, LambdaBody, LambdaExpression,
    ListLiteral, Literal, LoopHeader, LoopKind, LoopStatement, MatchArm, MatchArmBlock,
    MatchExpression, MatchPattern, MatchStatement, MemberExpression, Module, RangeExpression,
    ReturnStatement, SourceSpan, Statement, StructField, StructStatement, ThrowStatement,
    TryExpression, TypeExpression, UnaryExpression, UnaryOperator, UseStatement, VarBinding,
    VarStatement,
};
pub use crate::compiler::{Compilation, CompileOptions, Compiler};
pub use crate::diagnostics::{Diagnostic, DiagnosticLevel, Diagnostics};
pub use crate::formatter::format_source;
pub use crate::lexer::{Keyword, Lexer, Token, TokenKind};
pub use crate::resolver::{ModuleAliasBinding, Resolver, ResolverOutput};
// VM runtime exports removed - using AOT compilation only
pub use crate::source::{SourceFile, SourceId};
pub use crate::stdlib::{find_module as stdlib_find_module, StdFunction, StdModule};
pub use crate::stdlib_snapshot::{load_embedded as load_embedded_stdlib, Snapshot, SnapshotModule};
pub use crate::typechecker::TypeChecker;

#[cfg(feature = "llvm-aot")]
pub mod aot;
