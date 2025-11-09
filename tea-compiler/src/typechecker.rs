use std::collections::{HashMap, HashSet};

use crate::ast::{
    BinaryExpression, BinaryOperator, Block, CallArgument, CallExpression, CatchHandler, CatchKind,
    ConditionalExpression, ConditionalKind, ConditionalStatement, DictLiteral, ErrorAnnotation,
    ErrorTypeSpecifier, Expression, ExpressionKind, ForPattern, FunctionStatement, Identifier,
    IndexExpression, InterpolatedStringPart, LambdaBody, LambdaExpression, ListLiteral, Literal,
    LoopHeader, LoopKind, LoopStatement, MatchExpression, MatchPattern, MatchStatement, Module,
    ReturnStatement, SourceSpan, Statement, StructStatement, TestStatement, TryExpression,
    TypeExpression, TypeParameter, UnaryExpression, UnaryOperator, VarStatement,
};
use crate::diagnostics::Diagnostics;
use crate::lexer::{Keyword, Token, TokenKind};
use crate::stdlib::{self, StdArity, StdFunction, StdFunctionKind, StdType};
use serde_json::Value as JsonValue;
use serde_yaml::Value as YamlValue;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum Type {
    Bool,
    Int,
    Float,
    String,
    Nil,
    Void,
    Error(ErrorType),
    Optional(Box<Type>),
    List(Box<Type>),
    Dict(Box<Type>),
    Function(Vec<Type>, Box<Type>),
    Struct(StructType),
    Enum(EnumType),
    Union(UnionType),
    GenericParameter(String),
    Unknown,
}

impl Type {
    pub(crate) fn describe(&self) -> String {
        match self {
            Type::Bool => "Bool".to_string(),
            Type::Int => "Int".to_string(),
            Type::Float => "Float".to_string(),
            Type::String => "String".to_string(),
            Type::Nil => "Nil".to_string(),
            Type::Void => "Void".to_string(),
            Type::Optional(inner) => format!("{}?", inner.describe()),
            Type::List(element) => format!("List[{}]", element.describe()),
            Type::Dict(value) => format!("Dict[String, {}]", value.describe()),
            Type::Function(params, return_type) => {
                let param_str = if params.is_empty() {
                    String::from("()")
                } else {
                    let joined = params
                        .iter()
                        .map(|param| param.describe())
                        .collect::<Vec<_>>()
                        .join(", ");
                    format!("({joined})")
                };
                format!("Func{} -> {}", param_str, return_type.describe())
            }
            Type::Struct(struct_type) => {
                if struct_type.type_arguments.is_empty() {
                    struct_type.name.clone()
                } else {
                    let args = struct_type
                        .type_arguments
                        .iter()
                        .map(|arg| arg.describe())
                        .collect::<Vec<_>>()
                        .join(", ");
                    format!("{}[{args}]", struct_type.name)
                }
            }
            Type::Enum(enum_type) => enum_type.name.clone(),
            Type::Union(union_type) => union_type.name.clone(),
            Type::Error(error_type) => match &error_type.variant {
                Some(variant) => format!("{}.{}", error_type.name, variant),
                None => error_type.name.clone(),
            },
            Type::GenericParameter(name) => name.clone(),
            Type::Unknown => "Unknown".to_string(),
        }
    }
    fn is_numeric(&self) -> bool {
        matches!(self, Type::Int | Type::Float)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct StructType {
    pub name: String,
    pub type_arguments: Vec<Type>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct ErrorType {
    pub name: String,
    pub variant: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct UnionType {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct EnumType {
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct FunctionInstance {
    pub type_arguments: Vec<Type>,
    pub param_types: Vec<Type>,
    pub return_type: Type,
}

#[derive(Debug, Clone)]
pub struct StructInstance {
    pub type_arguments: Vec<Type>,
    pub field_types: Vec<Type>,
}

#[derive(Debug, Clone)]
pub(crate) struct FunctionSignature {
    pub params: Vec<Type>,
    pub return_type: Type,
    pub arity: StdArity,
    pub type_parameters: Vec<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct StructDefinition {
    pub type_parameters: Vec<String>,
    pub fields: Vec<StructFieldType>,
}

#[derive(Debug, Clone)]
pub(crate) struct UnionDefinition {
    pub members: Vec<Type>,
}

#[derive(Debug, Clone)]
pub(crate) struct EnumDefinition {
    pub variants: Vec<EnumVariantDefinition>,
}

#[derive(Debug, Clone)]
pub(crate) struct ErrorDefinition {
    pub variants: HashMap<String, ErrorVariantDefinition>,
}

#[derive(Debug, Clone)]
pub(crate) struct ErrorVariantDefinition {
    pub fields: Vec<ErrorFieldDefinition>,
}

#[derive(Debug, Clone)]
pub(crate) struct ErrorFieldDefinition {
    pub name: String,
    pub ty: Type,
}

impl EnumDefinition {
    fn variant(&self, name: &str) -> Option<&EnumVariantDefinition> {
        self.variants.iter().find(|variant| variant.name == name)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct EnumVariantDefinition {
    pub name: String,
}

#[derive(Debug, Clone)]
struct UnionMemberSource {
    type_expression: TypeExpression,
    span: SourceSpan,
}

#[derive(Debug, Clone)]
struct ErrorFieldSource {
    name: String,
    type_expression: TypeExpression,
    span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ErrorTag {
    name: String,
    variant: Option<String>,
}

impl ErrorTag {
    fn new(name: impl Into<String>, variant: Option<impl Into<String>>) -> Self {
        Self {
            name: name.into(),
            variant: variant.map(Into::into),
        }
    }

    fn matches(&self, other: &ErrorTag) -> bool {
        self.name == other.name
            && (self.variant.is_none() || other.variant.is_none() || self.variant == other.variant)
    }
}

#[derive(Debug, Clone, Default)]
struct ErrorSet {
    tags: HashSet<ErrorTag>,
}

impl ErrorSet {
    fn empty() -> Self {
        Self::default()
    }

    fn insert(&mut self, tag: ErrorTag) {
        self.tags.insert(tag);
    }

    fn contains(&self, tag: &ErrorTag) -> bool {
        self.tags.iter().any(|candidate| candidate.matches(tag))
    }
}

#[derive(Debug, Clone)]
struct ModuleFunctionInfo {
    signature: FunctionSignature,
    #[allow(dead_code)]
    kind: StdFunctionKind,
}

#[derive(Debug, Clone, Default)]
struct ModuleBinding {
    functions: HashMap<String, ModuleFunctionInfo>,
}

impl StructDefinition {
    fn field(&self, name: &str) -> Option<&StructFieldType> {
        self.fields.iter().find(|field| field.name == name)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct StructFieldType {
    pub name: String,
    pub ty: Type,
    #[allow(dead_code)]
    pub span: SourceSpan,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum OptionalGuardState {
    IsNil,
    IsNonNil,
}

struct OptionalGuard {
    name: String,
    when_true: Option<OptionalGuardState>,
    when_false: Option<OptionalGuardState>,
}

impl OptionalGuard {
    fn new(
        name: String,
        when_true: Option<OptionalGuardState>,
        when_false: Option<OptionalGuardState>,
    ) -> Self {
        Self {
            name,
            when_true,
            when_false,
        }
    }
}

#[derive(Debug, Clone)]
struct FunctionContext {
    return_type: Type,
    allowed_errors: ErrorSet,
    saw_explicit_return: bool,
    last_expression_type: Option<Type>,
    explicit_return_types: Vec<Type>,
}

pub struct TypeChecker {
    scopes: Vec<HashMap<String, Type>>,
    const_scopes: Vec<HashSet<String>>,
    non_nil_scopes: Vec<HashSet<String>>,
    functions: HashMap<String, FunctionSignature>,
    structs: HashMap<String, StructDefinition>,
    unions: HashMap<String, UnionDefinition>,
    union_name_spans: HashMap<String, SourceSpan>,
    union_member_sources: HashMap<String, Vec<UnionMemberSource>>,
    enums: HashMap<String, EnumDefinition>,
    errors: HashMap<String, ErrorDefinition>,
    error_variant_sources: HashMap<String, HashMap<String, Vec<ErrorFieldSource>>>,
    type_parameters: Vec<HashSet<String>>,
    builtins: HashMap<String, StdFunctionKind>,
    module_aliases: HashMap<String, ModuleBinding>,
    contexts: Vec<FunctionContext>,
    diagnostics: Diagnostics,
    lambda_types: HashMap<usize, Type>,
    function_instances: HashMap<String, Vec<FunctionInstance>>,
    struct_instances: HashMap<String, Vec<StructInstance>>,
    function_call_metadata: HashMap<SourceSpan, (String, FunctionInstance)>,
    struct_call_metadata: HashMap<SourceSpan, (String, StructInstance)>,
    binding_types: HashMap<SourceSpan, Type>,
    argument_expected_types: HashMap<SourceSpan, Type>,
    match_exhaustiveness: HashMap<SourceSpan, Vec<String>>,
    type_test_metadata: HashMap<SourceSpan, Type>,
    suppress_list_element_errors: bool,
}

impl TypeChecker {
    pub fn new() -> Self {
        let mut checker = Self {
            scopes: vec![HashMap::new()],
            const_scopes: vec![HashSet::new()],
            non_nil_scopes: vec![HashSet::new()],
            functions: HashMap::new(),
            structs: HashMap::new(),
            unions: HashMap::new(),
            union_name_spans: HashMap::new(),
            union_member_sources: HashMap::new(),
            enums: HashMap::new(),
            errors: HashMap::new(),
            error_variant_sources: HashMap::new(),
            type_parameters: Vec::new(),
            builtins: HashMap::new(),
            module_aliases: HashMap::new(),
            contexts: Vec::new(),
            diagnostics: Diagnostics::new(),
            lambda_types: HashMap::new(),
            function_instances: HashMap::new(),
            struct_instances: HashMap::new(),
            function_call_metadata: HashMap::new(),
            struct_call_metadata: HashMap::new(),
            binding_types: HashMap::new(),
            argument_expected_types: HashMap::new(),
            match_exhaustiveness: HashMap::new(),
            type_test_metadata: HashMap::new(),
            suppress_list_element_errors: false,
        };
        checker.register_builtin_structs();
        checker.register_builtin_functions();
        checker
    }

    pub fn check_module(&mut self, module: &Module) {
        self.collect_unions(&module.statements);
        self.collect_enums(&module.statements);
        self.collect_structs(&module.statements);
        self.collect_errors(&module.statements);
        self.populate_unions();
        self.populate_errors();
        self.check_statements(&module.statements);
    }

    pub fn into_diagnostics(self) -> Diagnostics {
        self.diagnostics
    }

    pub(crate) fn lambda_types(&self) -> &HashMap<usize, Type> {
        &self.lambda_types
    }

    pub(crate) fn function_instances(&self) -> &HashMap<String, Vec<FunctionInstance>> {
        &self.function_instances
    }

    pub(crate) fn struct_instances(&self) -> &HashMap<String, Vec<StructInstance>> {
        &self.struct_instances
    }

    pub(crate) fn function_call_metadata(
        &self,
    ) -> &HashMap<SourceSpan, (String, FunctionInstance)> {
        &self.function_call_metadata
    }

    pub(crate) fn struct_call_metadata(&self) -> &HashMap<SourceSpan, (String, StructInstance)> {
        &self.struct_call_metadata
    }

    pub(crate) fn binding_types(&self) -> &HashMap<SourceSpan, Type> {
        &self.binding_types
    }

    pub(crate) fn argument_expected_types(&self) -> &HashMap<SourceSpan, Type> {
        &self.argument_expected_types
    }

    pub(crate) fn struct_definitions(&self) -> HashMap<String, StructDefinition> {
        self.structs.clone()
    }

    pub(crate) fn union_definitions(&self) -> HashMap<String, UnionDefinition> {
        self.unions.clone()
    }

    pub(crate) fn enum_definitions(&self) -> HashMap<String, EnumDefinition> {
        self.enums.clone()
    }

    pub(crate) fn error_definitions(&self) -> HashMap<String, ErrorDefinition> {
        self.errors.clone()
    }

    pub(crate) fn match_exhaustiveness(&self) -> &HashMap<SourceSpan, Vec<String>> {
        &self.match_exhaustiveness
    }

    pub(crate) fn type_test_metadata(&self) -> &HashMap<SourceSpan, Type> {
        &self.type_test_metadata
    }

    fn report_error<S: Into<String>>(&mut self, message: S, span: Option<SourceSpan>) {
        let message = message.into();
        if let Some(span) = span {
            self.diagnostics
                .push_with_location(message, span.line, span.column);
        } else {
            self.diagnostics.push(message);
        }
    }

    fn report_warning<S: Into<String>>(&mut self, message: S, span: Option<SourceSpan>) {
        self.diagnostics
            .push_warning_with_span(message.into(), span);
    }

    fn register_builtin_structs(&mut self) {
        let cli_result = StructDefinition {
            type_parameters: Vec::new(),
            fields: vec![
                StructFieldType {
                    name: "exit".to_string(),
                    ty: Type::Int,
                    span: SourceSpan::default(),
                },
                StructFieldType {
                    name: "stdout".to_string(),
                    ty: Type::String,
                    span: SourceSpan::default(),
                },
                StructFieldType {
                    name: "stderr".to_string(),
                    ty: Type::String,
                    span: SourceSpan::default(),
                },
            ],
        };
        self.structs.insert("CliResult".to_string(), cli_result);

        let cli_parse_result = StructDefinition {
            type_parameters: Vec::new(),
            fields: vec![
                StructFieldType {
                    name: "ok".to_string(),
                    ty: Type::Bool,
                    span: SourceSpan::default(),
                },
                StructFieldType {
                    name: "exit".to_string(),
                    ty: Type::Int,
                    span: SourceSpan::default(),
                },
                StructFieldType {
                    name: "command".to_string(),
                    ty: Type::String,
                    span: SourceSpan::default(),
                },
                StructFieldType {
                    name: "path".to_string(),
                    ty: Type::List(Box::new(Type::String)),
                    span: SourceSpan::default(),
                },
                StructFieldType {
                    name: "options".to_string(),
                    ty: Type::Dict(Box::new(Type::Unknown)),
                    span: SourceSpan::default(),
                },
                StructFieldType {
                    name: "positionals".to_string(),
                    ty: Type::Dict(Box::new(Type::Unknown)),
                    span: SourceSpan::default(),
                },
                StructFieldType {
                    name: "scopes".to_string(),
                    ty: Type::List(Box::new(Type::Dict(Box::new(Type::Unknown)))),
                    span: SourceSpan::default(),
                },
                StructFieldType {
                    name: "rest".to_string(),
                    ty: Type::List(Box::new(Type::String)),
                    span: SourceSpan::default(),
                },
                StructFieldType {
                    name: "message".to_string(),
                    ty: Type::String,
                    span: SourceSpan::default(),
                },
                StructFieldType {
                    name: "help".to_string(),
                    ty: Type::String,
                    span: SourceSpan::default(),
                },
            ],
        };
        self.structs
            .insert("CliParseResult".to_string(), cli_parse_result);

        let process_result = StructDefinition {
            type_parameters: Vec::new(),
            fields: vec![
                StructFieldType {
                    name: "exit".to_string(),
                    ty: Type::Int,
                    span: SourceSpan::default(),
                },
                StructFieldType {
                    name: "success".to_string(),
                    ty: Type::Bool,
                    span: SourceSpan::default(),
                },
                StructFieldType {
                    name: "stdout".to_string(),
                    ty: Type::String,
                    span: SourceSpan::default(),
                },
                StructFieldType {
                    name: "stderr".to_string(),
                    ty: Type::String,
                    span: SourceSpan::default(),
                },
                StructFieldType {
                    name: "command".to_string(),
                    ty: Type::String,
                    span: SourceSpan::default(),
                },
            ],
        };
        self.structs
            .insert("ProcessResult".to_string(), process_result);
    }

    fn register_builtin_functions(&mut self) {
        for function in stdlib::BUILTINS {
            let signature = self.std_function_signature(function);
            self.builtins
                .insert(function.name.to_string(), function.kind);
            self.functions
                .insert(function.name.to_string(), signature.clone());
            self.assign_global(
                function.name.to_string(),
                Type::Function(
                    signature.params.clone(),
                    Box::new(signature.return_type.clone()),
                ),
            );
        }
    }

    fn collect_enums(&mut self, statements: &[Statement]) {
        for statement in statements {
            if let Statement::Enum(enum_stmt) = statement {
                if self.enums.contains_key(&enum_stmt.name) {
                    self.report_error(
                        format!("duplicate enum definition '{}'", enum_stmt.name),
                        Some(enum_stmt.name_span),
                    );
                    continue;
                }

                let mut seen = HashSet::new();
                let mut variants = Vec::new();
                for variant in &enum_stmt.variants {
                    if !seen.insert(variant.name.clone()) {
                        self.report_error(
                            format!(
                                "duplicate variant '{}' in enum '{}'",
                                variant.name, enum_stmt.name
                            ),
                            Some(variant.span),
                        );
                        continue;
                    }
                    variants.push(EnumVariantDefinition {
                        name: variant.name.clone(),
                    });
                }

                if variants.is_empty() {
                    self.report_error(
                        format!(
                            "enum '{}' must declare at least one variant",
                            enum_stmt.name
                        ),
                        Some(enum_stmt.name_span),
                    );
                }

                self.enums
                    .insert(enum_stmt.name.clone(), EnumDefinition { variants });
            }
        }
    }

    fn collect_errors(&mut self, statements: &[Statement]) {
        for statement in statements {
            if let Statement::Error(error_stmt) = statement {
                if self.errors.contains_key(&error_stmt.name) {
                    self.report_error(
                        format!("duplicate error definition '{}'", error_stmt.name),
                        Some(error_stmt.name_span),
                    );
                    continue;
                }

                let mut variants = HashMap::new();
                let mut sources: HashMap<String, Vec<ErrorFieldSource>> = HashMap::new();

                for variant in &error_stmt.variants {
                    if variants.contains_key(&variant.name) {
                        self.report_error(
                            format!(
                                "duplicate variant '{}' in error '{}'",
                                variant.name, error_stmt.name
                            ),
                            Some(variant.name_span),
                        );
                        continue;
                    }

                    variants.insert(
                        variant.name.clone(),
                        ErrorVariantDefinition { fields: Vec::new() },
                    );

                    let field_sources = variant
                        .fields
                        .iter()
                        .map(|field| ErrorFieldSource {
                            name: field.name.clone(),
                            type_expression: field.type_annotation.clone(),
                            span: field.name_span,
                        })
                        .collect();
                    sources.insert(variant.name.clone(), field_sources);
                }

                self.errors
                    .insert(error_stmt.name.clone(), ErrorDefinition { variants });
                self.error_variant_sources
                    .insert(error_stmt.name.clone(), sources);
            }
        }
    }

    fn collect_unions(&mut self, statements: &[Statement]) {
        for statement in statements {
            if let Statement::Union(union_stmt) = statement {
                if self.unions.contains_key(&union_stmt.name) {
                    self.report_error(
                        format!("duplicate union definition '{}'", union_stmt.name),
                        Some(union_stmt.name_span),
                    );
                    continue;
                }

                self.unions.insert(
                    union_stmt.name.clone(),
                    UnionDefinition {
                        members: Vec::new(),
                    },
                );
                self.union_name_spans
                    .insert(union_stmt.name.clone(), union_stmt.name_span);
                self.union_member_sources.insert(
                    union_stmt.name.clone(),
                    union_stmt
                        .members
                        .iter()
                        .map(|member| UnionMemberSource {
                            type_expression: member.type_expression.clone(),
                            span: member.span,
                        })
                        .collect(),
                );
            }
        }
    }

    fn collect_structs(&mut self, statements: &[Statement]) {
        // First pass: register struct names to allow forward references.
        for statement in statements {
            if let Statement::Struct(struct_stmt) = statement {
                if self.structs.contains_key(&struct_stmt.name) {
                    self.report_error(
                        format!("duplicate struct definition '{}'", struct_stmt.name),
                        Some(struct_stmt.name_span),
                    );
                } else {
                    self.structs.insert(
                        struct_stmt.name.clone(),
                        StructDefinition {
                            type_parameters: struct_stmt
                                .type_parameters
                                .iter()
                                .map(|param| param.name.clone())
                                .collect(),
                            fields: Vec::new(),
                        },
                    );
                }
            }
        }

        // Second pass: populate field information.
        for statement in statements {
            if let Statement::Struct(struct_stmt) = statement {
                self.populate_struct_fields(struct_stmt);
            }
        }
    }

    fn check_statements(&mut self, statements: &[Statement]) {
        for statement in statements {
            self.check_statement(statement);
        }
    }

    fn populate_struct_fields(&mut self, struct_stmt: &StructStatement) {
        let mut fields = Vec::new();
        let mut seen = HashSet::new();

        self.push_type_parameters(&struct_stmt.type_parameters);

        for field in &struct_stmt.fields {
            if !seen.insert(field.name.clone()) {
                self.report_error(
                    format!(
                        "duplicate field '{}' in struct '{}'",
                        field.name, struct_stmt.name
                    ),
                    Some(field.span),
                );
                continue;
            }

            let field_type = self
                .parse_type(&field.type_annotation)
                .unwrap_or(Type::Unknown);

            if let Type::Struct(ref referenced) = field_type {
                if !self.structs.contains_key(&referenced.name) {
                    self.report_error(
                        format!(
                            "unknown struct type '{}' in definition of '{}'",
                            referenced.name, struct_stmt.name
                        ),
                        Some(field.span),
                    );
                }
            }

            fields.push(StructFieldType {
                name: field.name.clone(),
                ty: field_type,
                span: field.span,
            });
        }

        if let Some(entry) = self.structs.get_mut(&struct_stmt.name) {
            entry.fields = fields;
        }

        self.pop_type_parameters();
    }

    fn populate_unions(&mut self) {
        let union_names: Vec<String> = self.union_member_sources.keys().cloned().collect();
        for name in union_names {
            let sources = match self.union_member_sources.get(&name) {
                Some(sources) => sources.clone(),
                None => continue,
            };
            let mut members = Vec::new();
            let mut seen = HashSet::new();

            for source in sources {
                let ty = self
                    .parse_type(&source.type_expression)
                    .unwrap_or(Type::Unknown);
                if matches!(ty, Type::Unknown) {
                    continue;
                }
                if matches!(&ty, Type::Union(inner) if inner.name == name) {
                    self.report_error(
                        format!("union '{}' cannot include itself", name),
                        Some(source.span),
                    );
                    continue;
                }
                if !self.is_supported_type_test_target(&ty) {
                    self.report_error(
                        format!(
                            "union '{}' member type '{}' is not supported",
                            name,
                            ty.describe()
                        ),
                        Some(source.span),
                    );
                    continue;
                }
                if !seen.insert(ty.clone()) {
                    self.report_error(
                        format!(
                            "duplicate member type '{}' in union '{}'",
                            ty.describe(),
                            name
                        ),
                        Some(source.span),
                    );
                    continue;
                }
                members.push(ty);
            }

            if members.is_empty() {
                let span = self.union_name_spans.get(&name).copied();
                self.report_error(
                    format!(
                        "union '{}' must declare at least one valid member type",
                        name
                    ),
                    span,
                );
            }

            if let Some(definition) = self.unions.get_mut(&name) {
                definition.members = members;
            }
        }
    }

    fn populate_errors(&mut self) {
        let error_names: Vec<String> = self.error_variant_sources.keys().cloned().collect();
        for error_name in error_names {
            let Some(variant_sources) = self.error_variant_sources.remove(&error_name) else {
                continue;
            };

            for (variant_name, sources) in variant_sources {
                let snapshot = self
                    .errors
                    .get(&error_name)
                    .and_then(|definition| definition.variants.get(&variant_name).cloned());

                let Some(mut variant_def) = snapshot else {
                    continue;
                };

                let mut fields = Vec::new();
                let mut seen = HashSet::new();
                for field in sources {
                    if !seen.insert(field.name.clone()) {
                        self.report_error(
                            format!(
                                "duplicate field '{}' in error '{}.{}'",
                                field.name, error_name, variant_name
                            ),
                            Some(field.span),
                        );
                        continue;
                    }

                    let field_type = self.parse_type(&field.type_expression).unwrap_or_else(|| {
                        self.report_error(
                            format!(
                                "could not resolve type for field '{}' in error '{}.{}'",
                                field.name, error_name, variant_name
                            ),
                            Some(field.span),
                        );
                        Type::Unknown
                    });

                    fields.push(ErrorFieldDefinition {
                        name: field.name,
                        ty: field_type,
                    });
                }

                variant_def.fields = fields;

                if let Some(definition) = self.errors.get_mut(&error_name) {
                    definition
                        .variants
                        .insert(variant_name.clone(), variant_def);
                }
            }
        }
    }

    fn resolve_error_annotation(&mut self, annotation: &ErrorAnnotation) -> ErrorSet {
        let mut set = ErrorSet::empty();
        for spec in &annotation.types {
            if let Some(tags) = self.resolve_error_specifier(spec) {
                for tag in tags {
                    set.insert(tag);
                }
            }
        }
        set
    }

    fn resolve_error_specifier(&mut self, spec: &ErrorTypeSpecifier) -> Option<Vec<ErrorTag>> {
        if spec.path.is_empty() {
            return None;
        }

        let error_name = &spec.path[0];
        let definition = match self.errors.get(error_name) {
            Some(definition) => definition,
            None => {
                self.report_error(format!("unknown error '{}'", error_name), Some(spec.span));
                return None;
            }
        };

        if spec.path.len() == 1 {
            return Some(vec![ErrorTag::new(error_name.clone(), None::<String>)]);
        }

        if spec.path.len() == 2 {
            let variant_name = &spec.path[1];
            if !definition.variants.contains_key(variant_name) {
                self.report_error(
                    format!("error '{}': unknown variant '{}'", error_name, variant_name),
                    Some(spec.span),
                );
                return None;
            }
            return Some(vec![ErrorTag::new(
                error_name.clone(),
                Some(variant_name.clone()),
            )]);
        }

        self.report_error(
            format!(
                "invalid error specifier '{}'; expected 'Error' or 'Error.Variant'",
                spec.path.join(".")
            ),
            Some(spec.span),
        );
        None
    }

    fn union_contains_type(
        &self,
        union_name: &str,
        candidate: &Type,
        visited: &mut HashSet<String>,
    ) -> bool {
        if !visited.insert(union_name.to_string()) {
            return false;
        }
        let Some(definition) = self.unions.get(union_name) else {
            visited.remove(union_name);
            return false;
        };
        for member in &definition.members {
            if self.types_compatible_for_union(member, candidate, visited) {
                visited.remove(union_name);
                return true;
            }
        }
        visited.remove(union_name);
        false
    }

    fn union_assignable_to_type(&self, union_name: &str, ty: &Type) -> bool {
        let Some(definition) = self.unions.get(union_name) else {
            return false;
        };
        definition.members.iter().all(|member| {
            let mut visited = HashSet::new();
            self.types_compatible_for_union(member, ty, &mut visited)
        })
    }

    fn union_is_subset(&self, subset: &str, superset: &str) -> bool {
        let Some(definition) = self.unions.get(subset) else {
            return false;
        };
        definition.members.iter().all(|member| {
            let mut visited = HashSet::new();
            self.union_contains_type(superset, member, &mut visited)
        })
    }

    fn flattened_union_members(&self, union_name: &str) -> HashSet<Type> {
        let mut visited = HashSet::new();
        let mut members = HashSet::new();
        self.collect_union_members_recursive(union_name, &mut visited, &mut members);
        members
    }

    fn collect_union_members_recursive(
        &self,
        union_name: &str,
        visited: &mut HashSet<String>,
        members: &mut HashSet<Type>,
    ) {
        if !visited.insert(union_name.to_string()) {
            return;
        }
        let Some(definition) = self.unions.get(union_name) else {
            return;
        };
        for member in &definition.members {
            match member {
                Type::Union(inner) => {
                    self.collect_union_members_recursive(&inner.name, visited, members);
                }
                _ => {
                    members.insert(member.clone());
                }
            }
        }
    }

    fn types_compatible_for_union(
        &self,
        member: &Type,
        candidate: &Type,
        visited: &mut HashSet<String>,
    ) -> bool {
        if member == candidate {
            return true;
        }
        match member {
            Type::Union(inner) => self.union_contains_type(&inner.name, candidate, visited),
            Type::Optional(inner) => {
                matches!(candidate, Type::Nil)
                    || self.types_compatible_for_union(inner, candidate, visited)
                    || matches!(candidate, Type::Optional(other)
                        if self.types_compatible_for_union(inner, other, visited))
            }
            _ => match candidate {
                Type::Optional(inner) => {
                    self.types_compatible_for_union(member, inner, visited)
                        || matches!(member, Type::Nil)
                }
                Type::Union(inner_union) => {
                    self.union_contains_type(&inner_union.name, member, visited)
                }
                _ => false,
            },
        }
    }

    fn validate_type_test(&mut self, value_type: &Type, target_type: &Type, span: SourceSpan) {
        if !self.is_supported_type_test_target(target_type) {
            self.report_error(
                format!(
                    "type test target '{}' is not supported",
                    target_type.describe()
                ),
                Some(span),
            );
            return;
        }

        if matches!(value_type, Type::Unknown) || matches!(target_type, Type::Unknown) {
            return;
        }

        let mut visited = HashSet::new();
        let possible = if let Type::Union(union_type) = value_type {
            self.union_contains_type(&union_type.name, target_type, &mut visited)
        } else if let Type::Union(union_type) = target_type {
            self.union_contains_type(&union_type.name, value_type, &mut visited)
        } else if let Type::Optional(inner) = value_type {
            matches!(target_type, Type::Nil)
                || self.types_compatible_for_union(inner, target_type, &mut visited)
        } else if let Type::Optional(inner) = target_type {
            matches!(value_type, Type::Nil)
                || self.types_compatible_for_union(value_type, inner, &mut visited)
        } else {
            value_type == target_type
                || (matches!(value_type, Type::Nil) && matches!(target_type, Type::Nil))
        };

        if !possible {
            self.report_error(
                format!(
                    "type test will always be false: '{}' is incompatible with '{}'",
                    value_type.describe(),
                    target_type.describe()
                ),
                Some(span),
            );
        }
    }

    fn is_supported_type_test_target(&self, ty: &Type) -> bool {
        match ty {
            Type::Bool
            | Type::Int
            | Type::Float
            | Type::String
            | Type::Nil
            | Type::Struct(_)
            | Type::Enum(_)
            | Type::Union(_) => true,
            Type::Optional(inner) => self.is_supported_type_test_target(inner),
            _ => false,
        }
    }

    fn check_statement(&mut self, statement: &Statement) {
        if let Some(ctx) = self.contexts.last_mut() {
            ctx.last_expression_type = None;
        }

        match statement {
            Statement::Var(var) => self.check_var(var),
            Statement::Expression(expr) => {
                let expr_type = self.infer_expression(&expr.expression);
                if let Some(ctx) = self.contexts.last_mut() {
                    ctx.last_expression_type = Some(expr_type.clone());
                }
            }
            Statement::Conditional(cond) => self.check_conditional(cond),
            Statement::Loop(loop_stmt) => self.check_loop(loop_stmt),
            Statement::Break(_) | Statement::Continue(_) => {
                // Break and continue are checked in the context of loops
                // No additional type checking needed here
            }
            Statement::Return(ret) => self.check_return(ret),
            Statement::Function(func) => self.check_function(func),
            Statement::Test(test_stmt) => self.check_test(test_stmt),
            Statement::Use(use_stmt) => self.register_use(use_stmt),
            Statement::Match(match_stmt) => self.check_match_statement(match_stmt),
            Statement::Struct(_) => {}
            Statement::Union(_) => {}
            Statement::Enum(_) => {}
            Statement::Error(_) => {}
            Statement::Throw(throw_stmt) => {
                let value_type = self.infer_expression(&throw_stmt.expression);
                match value_type {
                    Type::Error(error_type) => {
                        let tag =
                            ErrorTag::new(error_type.name.clone(), error_type.variant.clone());
                        self.register_forwarded_error(tag, throw_stmt.span);
                    }
                    Type::Unknown => {}
                    other => {
                        self.report_error(
                            format!(
                                "throw expression must evaluate to an error, found {}",
                                other.describe()
                            ),
                            Some(throw_stmt.span),
                        );
                    }
                }
            }
        }
    }

    fn check_var(&mut self, statement: &VarStatement) {
        for binding in &statement.bindings {
            let name = &binding.name;
            if statement.is_const && binding.initializer.is_none() {
                self.report_error(
                    format!("const '{}' requires an initializer", name),
                    Some(binding.span),
                );
            }
            let annotated = binding
                .type_annotation
                .as_ref()
                .and_then(|annotation| self.parse_type(annotation));

            let skip_list_inference = if let (Some(Type::List(expected_elem)), Some(init)) =
                (&annotated, &binding.initializer)
            {
                if let ExpressionKind::List(list) = &init.kind {
                    if matches!(expected_elem.as_ref(), Type::Union(_)) {
                        self.suppress_list_element_errors = true;
                        self.validate_list_elements_against_type(list, expected_elem, "list");
                        self.suppress_list_element_errors = false;
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            } else {
                false
            };

            let skip_dict_inference = if let (Some(Type::Dict(expected_value)), Some(init)) =
                (&annotated, &binding.initializer)
            {
                if let ExpressionKind::Dict(dict) = &init.kind {
                    self.validate_dict_values_against_type(dict, expected_value, "dict");
                    true
                } else {
                    false
                }
            } else {
                false
            };

            let inferred = if skip_list_inference || skip_dict_inference {
                annotated.clone().unwrap()
            } else {
                binding
                    .initializer
                    .as_ref()
                    .map(|expr| self.infer_expression(expr))
                    .unwrap_or(Type::Unknown)
            };

            let target_type = if let Some(expected) = annotated.clone() {
                if !skip_list_inference && !skip_dict_inference {
                    self.ensure_compatible(
                        &expected,
                        &inferred,
                        &format!("variable '{}'", name),
                        Some(binding.span),
                    );
                }
                expected
            } else {
                inferred.clone()
            };

            self.insert(name.clone(), target_type.clone(), !statement.is_const);
            self.binding_types.insert(binding.span, target_type.clone());
            self.update_non_nil_fact(name, &inferred);
        }
    }

    fn check_conditional(&mut self, statement: &ConditionalStatement) {
        let condition = self.infer_expression(&statement.condition);
        let bool_type = Type::Bool;
        self.ensure_compatible(
            &bool_type,
            &condition,
            "conditional expression",
            Some(statement.condition.span),
        );

        let guard = self.extract_optional_guard(&statement.condition);
        let guard_name = guard.as_ref().map(|g| g.name.clone());

        let (consequent_state, alternative_state) = match statement.kind {
            ConditionalKind::If => (
                guard.as_ref().and_then(|g| g.when_true),
                guard.as_ref().and_then(|g| g.when_false),
            ),
        };

        let base_scope = self.non_nil_scopes.last().cloned().unwrap_or_default();

        let consequent_scope = if let Some(ref name) = guard_name {
            if let Some(state) = consequent_state {
                self.with_branch_guard(name.as_str(), state, |checker| {
                    checker.check_statements(&statement.consequent.statements);
                })
            } else {
                self.run_branch(|checker| {
                    checker.check_statements(&statement.consequent.statements);
                })
            }
        } else {
            self.run_branch(|checker| {
                checker.check_statements(&statement.consequent.statements);
            })
        };

        let alternative_scope = if let Some(alt) = &statement.alternative {
            Some(if let Some(ref name) = guard_name {
                if let Some(state) = alternative_state {
                    self.with_branch_guard(name.as_str(), state, |checker| {
                        checker.check_statements(&alt.statements);
                    })
                } else {
                    self.run_branch(|checker| {
                        checker.check_statements(&alt.statements);
                    })
                }
            } else {
                self.run_branch(|checker| {
                    checker.check_statements(&alt.statements);
                })
            })
        } else {
            None
        };

        if let Some(ref guard) = guard {
            self.clear_non_nil_fact(&guard.name);

            let (true_continues, false_continues) = match statement.kind {
                ConditionalKind::If => {
                    let true_continues = !self.block_guarantees_exit(&statement.consequent);
                    let false_continues = if let Some(alt) = &statement.alternative {
                        !self.block_guarantees_exit(alt)
                    } else {
                        true
                    };
                    (true_continues, false_continues)
                }
            };

            let true_scope_has = consequent_scope.contains(&guard.name);
            let false_scope_has = if let Some(ref scope) = alternative_scope {
                scope.contains(&guard.name)
            } else {
                base_scope.contains(&guard.name)
            };

            let true_non_nil =
                true_scope_has || matches!(guard.when_true, Some(OptionalGuardState::IsNonNil));
            let false_non_nil =
                false_scope_has || matches!(guard.when_false, Some(OptionalGuardState::IsNonNil));

            if self.paths_imply_non_nil(
                true_continues,
                false_continues,
                true_non_nil,
                false_non_nil,
            ) {
                self.mark_non_nil_fact(&guard.name);
            }
        }
    }

    fn check_loop(&mut self, statement: &LoopStatement) {
        match statement.kind {
            LoopKind::While => {
                let LoopHeader::Condition(condition) = &statement.header else {
                    self.report_error(
                        "loop header is not supported by the type checker yet",
                        Some(statement.span),
                    );
                    return;
                };
                let condition_type = self.infer_expression(condition);
                let bool_type = Type::Bool;
                self.ensure_compatible(
                    &bool_type,
                    &condition_type,
                    "loop condition",
                    Some(condition.span),
                );
                self.check_statements(&statement.body.statements);
            }
            LoopKind::For => {
                let LoopHeader::For { pattern, iterator } = &statement.header else {
                    self.report_error(
                        "internal error: for loop without for header",
                        Some(statement.span),
                    );
                    return;
                };

                // Infer the type of the iterator expression
                let iterator_type = self.infer_expression(iterator);

                // Create a new scope for the loop body
                self.push_scope();

                match pattern {
                    ForPattern::Single(ident) => {
                        // Single variable: iterate over list elements
                        let element_type = match &iterator_type {
                            Type::List(element_type) => element_type.as_ref().clone(),
                            _ => {
                                self.report_error(
                                    format!(
                                        "cannot iterate over type {}, expected List[T]",
                                        iterator_type.describe()
                                    ),
                                    Some(iterator.span),
                                );
                                Type::Unknown
                            }
                        };

                        // Bind the loop variable to the element type
                        self.insert(ident.name.clone(), element_type.clone(), false);
                        self.binding_types.insert(ident.span, element_type);
                    }
                    ForPattern::Pair(key_ident, value_ident) => {
                        // Two variables: iterate over dict entries
                        let value_type = match &iterator_type {
                            Type::Dict(v) => v.as_ref().clone(),
                            _ => {
                                self.report_error(
                                    format!(
                                        "cannot iterate with two variables over type {}, expected Dict[V]",
                                        iterator_type.describe()
                                    ),
                                    Some(iterator.span),
                                );
                                Type::Unknown
                            }
                        };

                        // Bind the key variable (always String) and value variable
                        self.insert(key_ident.name.clone(), Type::String, false);
                        self.binding_types.insert(key_ident.span, Type::String);
                        self.insert(value_ident.name.clone(), value_type.clone(), false);
                        self.binding_types.insert(value_ident.span, value_type);
                    }
                }

                // Check the loop body
                self.check_statements(&statement.body.statements);

                self.pop_scope();
            }
        }
    }

    fn check_match_statement(&mut self, statement: &MatchStatement) {
        let span = statement.span;
        let scrutinee_type = self.infer_expression(&statement.scrutinee);

        let enum_variant_total = if let Type::Enum(enum_type) = &scrutinee_type {
            self.enums
                .get(&enum_type.name)
                .map(|definition| definition.variants.len())
        } else {
            None
        };

        let mut coverage_complete = false;
        let mut matched_bool_values: HashSet<bool> = HashSet::new();
        let mut matched_enum_variants: HashSet<String> = HashSet::new();
        let union_members = if let Type::Union(union_type) = &scrutinee_type {
            Some(self.flattened_union_members(&union_type.name))
        } else {
            None
        };
        let mut matched_union_members: HashSet<Type> = HashSet::new();
        let mut coverage_due_to_wildcard = false;

        for arm in &statement.arms {
            let arm_reachable = !coverage_complete;
            let arm_is_wildcard_only = arm
                .patterns
                .iter()
                .all(|pattern| matches!(pattern, MatchPattern::Wildcard { .. }));
            let suppress_unreachable =
                !arm_reachable && arm_is_wildcard_only && !coverage_due_to_wildcard;

            if !arm_reachable && !suppress_unreachable {
                self.report_warning(
                    "match arm is unreachable; previous patterns cover all values",
                    Some(arm.span),
                );
            }

            let mut arm_adds_coverage = false;

            for pattern in &arm.patterns {
                match pattern {
                    MatchPattern::Wildcard { span: pattern_span } => {
                        if suppress_unreachable {
                            continue;
                        }

                        if !arm_reachable {
                            self.report_warning(
                                "pattern is unreachable; previous patterns cover all values",
                                Some(*pattern_span),
                            );
                            continue;
                        }

                        if coverage_complete {
                            if coverage_due_to_wildcard {
                                self.report_warning(
                                    "pattern is unreachable; previous patterns cover all values",
                                    Some(*pattern_span),
                                );
                            }
                        } else {
                            arm_adds_coverage = true;
                            coverage_complete = true;
                            coverage_due_to_wildcard = true;
                        }
                    }
                    MatchPattern::Expression(pattern_expr) => {
                        let pattern_type = self.infer_expression(pattern_expr);
                        if scrutinee_type != Type::Unknown
                            && pattern_type != Type::Unknown
                            && pattern_type != scrutinee_type
                        {
                            self.report_error(
                                format!(
                                    "pattern type '{}' is incompatible with scrutinee type '{}'",
                                    pattern_type.describe(),
                                    scrutinee_type.describe()
                                ),
                                Some(pattern_expr.span),
                            );
                        }

                        if suppress_unreachable {
                            continue;
                        }

                        if !arm_reachable {
                            continue;
                        }

                        if coverage_complete {
                            self.report_warning(
                                "pattern is unreachable; previous patterns cover all values",
                                Some(pattern_expr.span),
                            );
                            continue;
                        }

                        match (&scrutinee_type, &pattern_expr.kind) {
                            (Type::Bool, ExpressionKind::Literal(Literal::Boolean(value))) => {
                                if !matched_bool_values.insert(*value) {
                                    self.report_warning(
                                        format!(
                                            "pattern `{}` is unreachable; value already matched",
                                            value
                                        ),
                                        Some(pattern_expr.span),
                                    );
                                } else {
                                    arm_adds_coverage = true;
                                    if matched_bool_values.len() == 2 {
                                        coverage_complete = true;
                                    }
                                }
                            }
                            (Type::Enum(enum_type), _) => {
                                let variant_name =
                                    if let ExpressionKind::Member(member) = &pattern_expr.kind {
                                        if let ExpressionKind::Identifier(identifier) =
                                            &member.object.kind
                                        {
                                            if identifier.name == enum_type.name {
                                                Some(member.property.clone())
                                            } else {
                                                None
                                            }
                                        } else {
                                            None
                                        }
                                    } else {
                                        None
                                    };

                                if let Some(variant) = variant_name {
                                    if !matched_enum_variants.insert(variant.clone()) {
                                        self.report_warning(
                                            format!(
                                                "pattern `{}.{}` is unreachable; variant already matched",
                                                enum_type.name, variant
                                            ),
                                            Some(pattern_expr.span),
                                        );
                                    } else {
                                        arm_adds_coverage = true;
                                        if let Some(total) = enum_variant_total {
                                            if matched_enum_variants.len() == total {
                                                coverage_complete = true;
                                            }
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    MatchPattern::Type(type_expr, pattern_span) => {
                        let target_type = self.parse_type(type_expr).unwrap_or(Type::Unknown);
                        self.validate_type_test(&scrutinee_type, &target_type, *pattern_span);
                        if !matches!(target_type, Type::Unknown) {
                            self.type_test_metadata
                                .insert(*pattern_span, target_type.clone());
                        }

                        if suppress_unreachable || !arm_reachable {
                            continue;
                        }

                        if coverage_complete {
                            self.report_warning(
                                "pattern is unreachable; previous patterns cover all values",
                                Some(*pattern_span),
                            );
                            continue;
                        }

                        if let (Some(members), Type::Union(_)) =
                            (union_members.as_ref(), &scrutinee_type)
                        {
                            let mut matched_any = false;
                            if let Type::Union(inner_union) = &target_type {
                                let target_members =
                                    self.flattened_union_members(&inner_union.name);
                                for member in target_members {
                                    if members.contains(&member) {
                                        matched_any = true;
                                        matched_union_members.insert(member);
                                    }
                                }
                            } else if members.contains(&target_type) {
                                matched_any = true;
                                matched_union_members.insert(target_type.clone());
                            }

                            if matched_any {
                                arm_adds_coverage = true;
                                if members.is_subset(&matched_union_members) {
                                    coverage_complete = true;
                                }
                            }
                        }
                    }
                }
            }

            let coverage_trackable = coverage_due_to_wildcard
                || matches!(scrutinee_type, Type::Bool | Type::Enum(_))
                || union_members.is_some();
            if !arm_adds_coverage && arm_reachable && coverage_trackable {
                self.report_warning(
                    "match arm is unreachable; previous patterns cover all values",
                    Some(arm.span),
                );
            }

            self.run_branch(|checker| {
                checker.check_statements(&arm.block.statements);
            });
        }

        let mut missing_patterns: Vec<String> = Vec::new();
        if !coverage_complete {
            match &scrutinee_type {
                Type::Bool => {
                    if !matched_bool_values.contains(&true) {
                        missing_patterns.push("true".to_string());
                    }
                    if !matched_bool_values.contains(&false) {
                        missing_patterns.push("false".to_string());
                    }
                }
                Type::Enum(enum_type) => {
                    if let Some(definition) = self.enums.get(&enum_type.name) {
                        for variant in &definition.variants {
                            if !matched_enum_variants.contains(&variant.name) {
                                missing_patterns
                                    .push(format!("{}.{}", enum_type.name, variant.name));
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        if coverage_complete {
            self.match_exhaustiveness.remove(&span);
        } else if !missing_patterns.is_empty() {
            let preview = missing_patterns
                .iter()
                .take(3)
                .map(|case| format!("`{case}`"))
                .collect::<Vec<_>>()
                .join(", ");
            let message = if missing_patterns.len() <= 3 {
                format!("match statement is not exhaustive; missing {preview}")
            } else {
                format!(
                    "match statement is not exhaustive; missing {} cases (e.g. {})",
                    missing_patterns.len(),
                    preview
                )
            };
            self.report_error(message, Some(span));
            self.match_exhaustiveness.insert(span, missing_patterns);
        } else {
            self.report_error(
                "match statement is not exhaustive; add `_` arm or cover all values".to_string(),
                Some(span),
            );
            self.match_exhaustiveness.remove(&span);
        }
    }

    fn check_return(&mut self, statement: &ReturnStatement) {
        if let Some(ctx) = self.contexts.last_mut() {
            ctx.saw_explicit_return = true;
            ctx.last_expression_type = None;
        }

        let expected = self
            .contexts
            .last()
            .map(|ctx| ctx.return_type.clone())
            .unwrap_or(Type::Unknown);

        match (&statement.expression, expected) {
            (Some(expr), Type::Unknown) => {
                let actual = self.infer_expression(expr);
                if let Some(ctx) = self.contexts.last_mut() {
                    ctx.explicit_return_types.push(actual.clone());
                }
            }
            (Some(expr), expected_type) => {
                let actual = self.infer_expression(expr);
                self.ensure_compatible(
                    &expected_type,
                    &actual,
                    "return expression",
                    Some(statement.span),
                );
                if let Some(ctx) = self.contexts.last_mut() {
                    ctx.explicit_return_types.push(actual.clone());
                }
            }
            (None, Type::Unknown) => {
                if let Some(ctx) = self.contexts.last_mut() {
                    ctx.explicit_return_types.push(Type::Void);
                }
            }
            (None, Type::Void) => {
                if let Some(ctx) = self.contexts.last_mut() {
                    ctx.explicit_return_types.push(Type::Void);
                }
            }
            (None, expected_type) => {
                self.report_error(
                    format!(
                        "return type mismatch: expected {}, found Void",
                        expected_type.describe()
                    ),
                    Some(statement.span),
                );
                if let Some(ctx) = self.contexts.last_mut() {
                    ctx.explicit_return_types.push(Type::Void);
                }
            }
        }
    }

    fn with_branch_guard<F>(
        &mut self,
        name: &str,
        state: OptionalGuardState,
        mut body: F,
    ) -> HashSet<String>
    where
        F: FnMut(&mut Self),
    {
        let saved_scope = self.non_nil_scopes.last().cloned();
        if let Some(scope) = self.non_nil_scopes.last_mut() {
            match state {
                OptionalGuardState::IsNonNil => {
                    scope.insert(name.to_string());
                }
                OptionalGuardState::IsNil => {
                    scope.remove(name);
                }
            }
        }

        body(self);

        let branch_scope = self.non_nil_scopes.last().cloned().unwrap_or_default();

        if let (Some(saved), Some(scope)) = (saved_scope, self.non_nil_scopes.last_mut()) {
            *scope = saved;
        }

        branch_scope
    }

    fn run_branch<F>(&mut self, mut body: F) -> HashSet<String>
    where
        F: FnMut(&mut Self),
    {
        let saved_scope = self.non_nil_scopes.last().cloned();
        body(self);
        let branch_scope = self.non_nil_scopes.last().cloned().unwrap_or_default();
        if let (Some(saved), Some(scope)) = (saved_scope, self.non_nil_scopes.last_mut()) {
            *scope = saved;
        }
        branch_scope
    }

    fn paths_imply_non_nil(
        &self,
        true_continues: bool,
        false_continues: bool,
        true_non_nil: bool,
        false_non_nil: bool,
    ) -> bool {
        match (true_continues, false_continues) {
            (true, true) => true_non_nil && false_non_nil,
            (true, false) => true_non_nil,
            (false, true) => false_non_nil,
            (false, false) => false,
        }
    }

    fn block_guarantees_exit(&self, block: &Block) -> bool {
        block
            .statements
            .last()
            .map(|statement| matches!(statement, Statement::Return(_)))
            .unwrap_or(false)
    }

    fn extract_optional_guard(&self, expression: &Expression) -> Option<OptionalGuard> {
        let ExpressionKind::Binary(binary) = &expression.kind else {
            return None;
        };

        match binary.operator {
            BinaryOperator::Equal => {
                self.optional_guard_from_comparison(&binary.left, &binary.right, true)
            }
            BinaryOperator::NotEqual => {
                self.optional_guard_from_comparison(&binary.left, &binary.right, false)
            }
            _ => None,
        }
    }

    fn optional_guard_from_comparison(
        &self,
        left: &Expression,
        right: &Expression,
        is_equality: bool,
    ) -> Option<OptionalGuard> {
        if let Some(name) = self.identifier_name_if_optional(left) {
            if Self::is_nil_literal(right) {
                return Some(Self::guard_from_comparison(name, is_equality));
            }
        }
        if let Some(name) = self.identifier_name_if_optional(right) {
            if Self::is_nil_literal(left) {
                return Some(Self::guard_from_comparison(name, is_equality));
            }
        }
        None
    }

    fn guard_from_comparison(name: String, is_equality: bool) -> OptionalGuard {
        if is_equality {
            OptionalGuard::new(
                name,
                Some(OptionalGuardState::IsNil),
                Some(OptionalGuardState::IsNonNil),
            )
        } else {
            OptionalGuard::new(
                name,
                Some(OptionalGuardState::IsNonNil),
                Some(OptionalGuardState::IsNil),
            )
        }
    }

    fn identifier_name_if_optional(&self, expression: &Expression) -> Option<String> {
        match &expression.kind {
            ExpressionKind::Identifier(identifier) => {
                if self.binding_is_optional(&identifier.name) {
                    Some(identifier.name.clone())
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn is_nil_literal(expression: &Expression) -> bool {
        matches!(expression.kind, ExpressionKind::Literal(Literal::Nil))
    }

    fn check_function(&mut self, function: &FunctionStatement) {
        self.push_type_parameters(&function.type_parameters);
        let mut param_types = Vec::with_capacity(function.parameters.len());
        for param in &function.parameters {
            if param.type_annotation.is_none() {
                self.report_error(
                    format!(
                        "parameter '{}' in function '{}' must have a type annotation",
                        param.name, function.name
                    ),
                    Some(param.span),
                );
            }
            let ty = param
                .type_annotation
                .as_ref()
                .and_then(|annotation| self.parse_type(annotation))
                .unwrap_or(Type::Unknown);
            param_types.push(ty);
        }

        let declared_return_type = if let Some(annotation) = &function.return_type {
            self.parse_type(annotation)
        } else {
            self.report_error(
                format!("function '{}' must declare a return type", function.name),
                Some(function.name_span),
            );
            None
        }
        .unwrap_or(Type::Unknown);

        let allowed_errors = function
            .error_annotation
            .as_ref()
            .map(|annotation| self.resolve_error_annotation(annotation))
            .unwrap_or_else(ErrorSet::empty);

        let signature = FunctionSignature {
            params: param_types.clone(),
            return_type: declared_return_type.clone(),
            arity: StdArity::Exact(function.parameters.len()),
            type_parameters: function
                .type_parameters
                .iter()
                .map(|param| param.name.clone())
                .collect(),
        };
        self.functions
            .insert(function.name.clone(), signature.clone());
        let function_type = Type::Function(
            signature.params.clone(),
            Box::new(signature.return_type.clone()),
        );
        self.assign_global(function.name.clone(), function_type.clone());
        self.binding_types
            .insert(function.name_span, function_type.clone());

        self.push_scope();
        self.contexts.push(FunctionContext {
            return_type: declared_return_type.clone(),
            allowed_errors: allowed_errors.clone(),
            saw_explicit_return: false,
            last_expression_type: None,
            explicit_return_types: Vec::new(),
        });

        for (param, expected_type) in function.parameters.iter().zip(param_types.iter()) {
            self.insert(param.name.clone(), expected_type.clone(), true);
            self.binding_types.insert(param.span, expected_type.clone());
            if let Some(default) = &param.default_value {
                let actual = self.infer_expression(default);
                self.ensure_compatible(
                    expected_type,
                    &actual,
                    &format!(
                        "default value for parameter '{}' in function '{}'",
                        param.name, function.name
                    ),
                    Some(param.span),
                );
            }
        }

        self.check_statements(&function.body.statements);

        let context = self.contexts.pop().unwrap();
        if !context.saw_explicit_return {
            match context.last_expression_type {
                Some(last_type) => {
                    self.ensure_compatible(
                        &declared_return_type,
                        &last_type,
                        &format!("implicit return in '{}'", function.name),
                        Some(function.name_span),
                    );
                }
                None => {
                    if declared_return_type != Type::Unknown
                        && declared_return_type != Type::Void
                        && declared_return_type != Type::Nil
                    {
                        self.report_error(
                            format!(
                                "function '{}' may exit without returning a value of type {}",
                                function.name,
                                declared_return_type.describe()
                            ),
                            Some(function.name_span),
                        );
                    }
                }
            }
        }

        self.pop_scope();
        self.pop_type_parameters();
    }

    fn check_test(&mut self, test: &TestStatement) {
        self.push_scope();
        self.check_statements(&test.body.statements);
        self.pop_scope();
    }

    fn ensure_compatible(
        &mut self,
        expected: &Type,
        actual: &Type,
        context: &str,
        span: Option<SourceSpan>,
    ) -> bool {
        if matches!(expected, Type::Unknown) || matches!(actual, Type::Unknown) {
            return true;
        }

        match (expected, actual) {
            (Type::Optional(_), Type::Nil) => true,
            (Type::Optional(expected_inner), Type::Optional(actual_inner)) => {
                self.ensure_compatible(expected_inner, actual_inner, context, span)
            }
            (Type::Optional(expected_inner), other) => {
                self.ensure_compatible(expected_inner, other, context, span)
            }
            (Type::List(expected_inner), Type::List(actual_inner)) => {
                self.ensure_compatible(expected_inner, actual_inner, context, span)
            }
            (Type::Dict(expected_inner), Type::Dict(actual_inner)) => {
                self.ensure_compatible(expected_inner, actual_inner, context, span)
            }
            (Type::Union(expected_union), Type::Union(actual_union)) => {
                if expected_union.name == actual_union.name
                    || self.union_is_subset(&actual_union.name, &expected_union.name)
                {
                    true
                } else {
                    self.report_error(
                        format!(
                            "{}: expected {}, found {}",
                            context,
                            expected.describe(),
                            actual.describe()
                        ),
                        span,
                    );
                    false
                }
            }
            (Type::Union(expected_union), actual_type) => {
                let mut visited = HashSet::new();
                if self.union_contains_type(&expected_union.name, actual_type, &mut visited) {
                    true
                } else {
                    self.report_error(
                        format!(
                            "{}: expected {}, found {}",
                            context,
                            expected.describe(),
                            actual.describe()
                        ),
                        span,
                    );
                    false
                }
            }
            (expected_type, Type::Union(actual_union)) => {
                if self.union_assignable_to_type(&actual_union.name, expected_type) {
                    true
                } else {
                    self.report_error(
                        format!(
                            "{}: expected {}, found {}",
                            context,
                            expected.describe(),
                            actual.describe()
                        ),
                        span,
                    );
                    false
                }
            }
            (Type::Error(expected_error), Type::Error(actual_error)) => {
                if expected_error.name == actual_error.name {
                    if expected_error.variant.is_none()
                        || expected_error.variant == actual_error.variant
                    {
                        true
                    } else {
                        self.report_error(
                            format!(
                                "{}: expected {}, found {}",
                                context,
                                expected.describe(),
                                actual.describe()
                            ),
                            span,
                        );
                        false
                    }
                } else {
                    self.report_error(
                        format!(
                            "{}: expected {}, found {}",
                            context,
                            expected.describe(),
                            actual.describe()
                        ),
                        span,
                    );
                    false
                }
            }
            (Type::Error(expected_error), actual_type) => {
                self.report_error(
                    format!(
                        "{}: expected {}, found {}",
                        context,
                        Type::Error(expected_error.clone()).describe(),
                        actual_type.describe()
                    ),
                    span,
                );
                false
            }
            (expected_type, Type::Error(actual_error)) => {
                self.report_error(
                    format!(
                        "{}: expected {}, found {}",
                        context,
                        expected_type.describe(),
                        Type::Error(actual_error.clone()).describe()
                    ),
                    span,
                );
                false
            }
            (
                Type::Function(expected_params, expected_ret),
                Type::Function(actual_params, actual_ret),
            ) => {
                if expected_params.len() != actual_params.len() {
                    self.report_error(
                        format!(
                            "{}: expected function with {} parameters, found {}",
                            context,
                            expected_params.len(),
                            actual_params.len()
                        ),
                        span,
                    );
                    return false;
                }

                let mut compatible = true;
                for (index, (expected_param, actual_param)) in
                    expected_params.iter().zip(actual_params.iter()).enumerate()
                {
                    let param_context = format!("{} parameter {}", context, index + 1);
                    if !self.ensure_compatible(expected_param, actual_param, &param_context, span) {
                        compatible = false;
                    }
                }

                let return_context = format!("{} return type", context);
                if !self.ensure_compatible(expected_ret, actual_ret, &return_context, span) {
                    compatible = false;
                }

                compatible
            }
            _ if expected == actual => true,
            _ => {
                self.report_error(
                    format!(
                        "{}: expected {}, found {}",
                        context,
                        expected.describe(),
                        actual.describe()
                    ),
                    span,
                );
                false
            }
        }
    }

    fn register_forwarded_error(&mut self, tag: ErrorTag, span: SourceSpan) {
        if let Some(context) = self.contexts.last() {
            if !context.allowed_errors.contains(&tag) {
                let display = if let Some(variant) = &tag.variant {
                    format!("{}.{}", tag.name, variant)
                } else {
                    tag.name.clone()
                };
                self.report_error(
                    format!(
                        "error '{}' is not declared in the function's error annotation",
                        display
                    ),
                    Some(span),
                );
            }
        } else {
            self.report_error(
                "throw statements are only allowed inside functions",
                Some(span),
            );
        }
    }

    fn infer_binding_error_type(&mut self, patterns: &[MatchPattern]) -> Option<ErrorType> {
        let mut result: Option<ErrorType> = None;
        for pattern in patterns {
            match pattern {
                MatchPattern::Type(type_expr, pattern_span) => {
                    let ty = self.parse_type(type_expr).unwrap_or(Type::Unknown);
                    if let Type::Error(pattern_error) = ty {
                        if let Some(existing) = &mut result {
                            if existing.name != pattern_error.name {
                                return None;
                            }
                            if existing.variant != pattern_error.variant {
                                existing.variant = None;
                            }
                        } else {
                            result = Some(pattern_error);
                        }
                    } else {
                        self.report_error(
                            "catch case types must be error variants",
                            Some(*pattern_span),
                        );
                        return None;
                    }
                }
                MatchPattern::Wildcard { .. } => return None,
                _ => return None,
            }
        }
        result
    }

    fn analyze_catch_patterns(&mut self, patterns: &[MatchPattern]) {
        for pattern in patterns {
            if let MatchPattern::Expression(expr) = pattern {
                self.infer_expression(expr);
            }
            if let MatchPattern::Type(type_expr, pattern_span) = pattern {
                let ty = self.parse_type(type_expr).unwrap_or(Type::Unknown);
                if !matches!(ty, Type::Unknown) {
                    self.type_test_metadata.insert(*pattern_span, ty);
                }
            }
        }
    }

    fn clear_non_nil_fact(&mut self, name: &str) {
        for scope in self.non_nil_scopes.iter_mut().rev() {
            if scope.remove(name) {
                break;
            }
        }
    }

    fn mark_non_nil_fact(&mut self, name: &str) {
        if let Some(scope) = self.non_nil_scopes.last_mut() {
            scope.insert(name.to_string());
        }
    }

    fn is_non_nil_fact(&self, name: &str) -> bool {
        self.non_nil_scopes
            .iter()
            .rev()
            .any(|scope| scope.contains(name))
    }

    fn binding_is_optional(&self, name: &str) -> bool {
        self.lookup(name)
            .map(|ty| matches!(ty, Type::Optional(_)))
            .unwrap_or(false)
    }

    fn type_is_definitely_non_nil(&self, ty: &Type) -> bool {
        !matches!(ty, Type::Optional(_) | Type::Unknown | Type::Nil)
    }

    fn update_non_nil_fact(&mut self, name: &str, value_type: &Type) {
        self.clear_non_nil_fact(name);
        if self.binding_is_optional(name) && self.type_is_definitely_non_nil(value_type) {
            self.mark_non_nil_fact(name);
        }
    }

    fn substitute_type(&self, ty: &Type, mapping: &HashMap<String, Type>) -> Type {
        match ty {
            Type::GenericParameter(name) => mapping
                .get(name)
                .cloned()
                .unwrap_or_else(|| Type::GenericParameter(name.clone())),
            Type::Optional(inner) => Type::Optional(Box::new(self.substitute_type(inner, mapping))),
            Type::List(inner) => Type::List(Box::new(self.substitute_type(inner, mapping))),
            Type::Dict(inner) => Type::Dict(Box::new(self.substitute_type(inner, mapping))),
            Type::Function(params, return_type) => {
                let substituted_params = params
                    .iter()
                    .map(|param| self.substitute_type(param, mapping))
                    .collect();
                let substituted_return = self.substitute_type(return_type, mapping);
                Type::Function(substituted_params, Box::new(substituted_return))
            }
            Type::Struct(struct_type) => Type::Struct(StructType {
                name: struct_type.name.clone(),
                type_arguments: struct_type
                    .type_arguments
                    .iter()
                    .map(|arg| self.substitute_type(arg, mapping))
                    .collect(),
            }),
            other => other.clone(),
        }
    }

    fn unify_types(
        &mut self,
        expected: &Type,
        actual: &Type,
        mapping: &mut HashMap<String, Type>,
        context: &str,
        span: Option<SourceSpan>,
    ) -> bool {
        match expected {
            Type::GenericParameter(name) => {
                if let Some(existing) = mapping.get(name) {
                    self.ensure_compatible(existing, actual, context, span)
                } else {
                    mapping.insert(name.clone(), actual.clone());
                    true
                }
            }
            Type::Optional(expected_inner) => match actual {
                Type::Optional(actual_inner) => {
                    self.unify_types(expected_inner, actual_inner, mapping, context, span)
                }
                _ => self.ensure_compatible(expected, actual, context, span),
            },
            Type::List(expected_inner) => {
                if let Type::List(actual_inner) = actual {
                    self.unify_types(expected_inner, actual_inner, mapping, context, span)
                } else {
                    self.ensure_compatible(expected, actual, context, span)
                }
            }
            Type::Dict(expected_inner) => {
                if let Type::Dict(actual_inner) = actual {
                    self.unify_types(expected_inner, actual_inner, mapping, context, span)
                } else {
                    self.ensure_compatible(expected, actual, context, span)
                }
            }
            Type::Function(expected_params, expected_ret) => {
                if let Type::Function(actual_params, actual_ret) = actual {
                    if expected_params.len() != actual_params.len() {
                        let message = format!(
                            "{}: expected function with {} parameters, found {}",
                            context,
                            expected_params.len(),
                            actual_params.len()
                        );
                        self.report_error(message, span);
                        return false;
                    }

                    let mut ok = true;
                    for (index, (expected_param, actual_param)) in
                        expected_params.iter().zip(actual_params.iter()).enumerate()
                    {
                        let param_context = format!("{} parameter {}", context, index + 1);
                        if !self.unify_types(
                            expected_param,
                            actual_param,
                            mapping,
                            &param_context,
                            span,
                        ) {
                            ok = false;
                        }
                    }

                    if !self.unify_types(expected_ret, actual_ret, mapping, context, span) {
                        ok = false;
                    }

                    ok
                } else {
                    self.ensure_compatible(expected, actual, context, span)
                }
            }
            Type::Struct(expected_struct) => {
                if let Type::Struct(actual_struct) = actual {
                    if expected_struct.name != actual_struct.name {
                        return self.ensure_compatible(expected, actual, context, span);
                    }

                    let mut ok = true;
                    for (index, (expected_arg, actual_arg)) in expected_struct
                        .type_arguments
                        .iter()
                        .zip(actual_struct.type_arguments.iter())
                        .enumerate()
                    {
                        let arg_context = format!("{} type argument {}", context, index + 1);
                        if !self.unify_types(expected_arg, actual_arg, mapping, &arg_context, span)
                        {
                            ok = false;
                        }
                    }

                    if expected_struct.type_arguments.len() != actual_struct.type_arguments.len() {
                        let message = format!(
                            "{}: expected {} type arguments, found {}",
                            context,
                            expected_struct.type_arguments.len(),
                            actual_struct.type_arguments.len()
                        );
                        self.report_error(message, span);
                        ok = false;
                    }

                    ok
                } else {
                    self.ensure_compatible(expected, actual, context, span)
                }
            }
            _ => self.ensure_compatible(expected, actual, context, span),
        }
    }

    fn merge_binding_type(
        &mut self,
        existing: Type,
        new_type: Type,
        context: &str,
        span: Option<SourceSpan>,
    ) -> Type {
        match (existing, new_type) {
            (Type::Unknown, ty) => ty,
            (ty, Type::Unknown) => ty,
            (Type::Optional(existing_inner), Type::Optional(new_inner)) => {
                let merged = self.merge_binding_type(*existing_inner, *new_inner, context, span);
                Type::Optional(Box::new(merged))
            }
            (Type::Optional(existing_inner), Type::Nil) => Type::Optional(existing_inner),
            (Type::Optional(existing_inner), other) => {
                let merged = self.merge_binding_type(*existing_inner, other, context, span);
                Type::Optional(Box::new(merged))
            }
            (Type::List(existing_inner), Type::List(new_inner)) => {
                let merged = self.merge_binding_type(*existing_inner, *new_inner, context, span);
                Type::List(Box::new(merged))
            }
            (Type::Dict(existing_inner), Type::Dict(new_inner)) => {
                let merged = self.merge_binding_type(*existing_inner, *new_inner, context, span);
                Type::Dict(Box::new(merged))
            }
            (
                Type::Function(existing_params, existing_ret),
                Type::Function(new_params, new_ret),
            ) => {
                if existing_params.len() != new_params.len() {
                    self.report_error(
                        format!(
                            "{}: expected function with {} parameters, found {}",
                            context,
                            existing_params.len(),
                            new_params.len()
                        ),
                        span,
                    );
                    Type::Function(existing_params, existing_ret)
                } else {
                    let merged_params = existing_params
                        .into_iter()
                        .zip(new_params.into_iter())
                        .enumerate()
                        .map(|(index, (left, right))| {
                            let param_context = format!("{} parameter {}", context, index + 1);
                            self.merge_binding_type(left, right, &param_context, span)
                        })
                        .collect();
                    let merged_return = self.merge_binding_type(
                        *existing_ret,
                        *new_ret,
                        &format!("{} return type", context),
                        span,
                    );
                    Type::Function(merged_params, Box::new(merged_return))
                }
            }
            (left, right) if left == right => left,
            (left, right) => {
                if !self.suppress_list_element_errors {
                    let is_primitive_mismatch = matches!(
                        (&left, &right),
                        (Type::Bool, _)
                            | (Type::Int, _)
                            | (Type::Float, _)
                            | (Type::String, _)
                            | (_, Type::Bool)
                            | (_, Type::Int)
                            | (_, Type::Float)
                            | (_, Type::String)
                    ) && !matches!(left, Type::Unknown)
                        && !matches!(right, Type::Unknown);

                    let is_list_element = context.starts_with("list element ");

                    if is_primitive_mismatch && is_list_element {
                        self.report_error(
                            format!(
                                "{}: expected {}, found {}. List elements must be the same type or use a union type, e.g. 'union Result {{ {} {} }}' with List[Result]",
                                context,
                                left.describe(),
                                right.describe(),
                                left.describe(),
                                right.describe()
                            ),
                            span,
                        );
                    } else {
                        self.report_error(
                            format!(
                                "{}: expected {}, found {}",
                                context,
                                left.describe(),
                                right.describe()
                            ),
                            span,
                        );
                    }
                }
                left
            }
        }
    }

    fn record_function_instance(
        &mut self,
        span: SourceSpan,
        name: &str,
        instance: FunctionInstance,
    ) {
        let entry = self
            .function_instances
            .entry(name.to_string())
            .or_insert_with(Vec::new);
        if !entry
            .iter()
            .any(|existing| existing.type_arguments == instance.type_arguments)
        {
            entry.push(instance.clone());
        }
        self.function_call_metadata
            .insert(span, (name.to_string(), instance));
    }

    fn record_struct_instance(&mut self, span: SourceSpan, name: &str, instance: StructInstance) {
        let entry = self
            .struct_instances
            .entry(name.to_string())
            .or_insert_with(Vec::new);
        if !entry
            .iter()
            .any(|existing| existing.type_arguments == instance.type_arguments)
        {
            entry.push(instance.clone());
        }
        self.struct_call_metadata
            .insert(span, (name.to_string(), instance));
    }

    fn std_function_signature(&mut self, function: &StdFunction) -> FunctionSignature {
        let mut params: Vec<Type> = function
            .params
            .iter()
            .map(|param| self.std_type_to_type(*param))
            .collect();
        match function.kind {
            StdFunctionKind::PathJoin => {
                if let Some(first) = params.get_mut(0) {
                    *first = Type::List(Box::new(Type::String));
                }
            }

            _ => {}
        }

        let mut return_type = self.std_type_to_type(function.return_type);
        match function.kind {
            StdFunctionKind::PathComponents => {
                return_type = Type::List(Box::new(Type::String));
            }
            StdFunctionKind::FsListDir | StdFunctionKind::FsWalk => {
                return_type = Type::List(Box::new(Type::String));
            }
            StdFunctionKind::EnvVars => {
                return_type = Type::Dict(Box::new(Type::String));
            }
            _ => {}
        }

        FunctionSignature {
            params,
            return_type,
            arity: function.arity,
            type_parameters: Vec::new(),
        }
    }

    fn register_use(&mut self, use_stmt: &crate::ast::UseStatement) {
        let module_path = use_stmt.module_path.as_str();
        if let Some(module) = stdlib::find_module(module_path) {
            let mut binding = ModuleBinding::default();
            for function in module.functions {
                let signature = self.std_function_signature(function);
                self.builtins
                    .insert(function.name.to_string(), function.kind);
                self.functions
                    .insert(function.name.to_string(), signature.clone());
                self.assign_global(
                    function.name.to_string(),
                    Type::Function(
                        signature.params.clone(),
                        Box::new(signature.return_type.clone()),
                    ),
                );
                binding.functions.insert(
                    function.name.to_string(),
                    ModuleFunctionInfo {
                        signature,
                        kind: function.kind,
                    },
                );
            }
            self.module_aliases
                .insert(use_stmt.alias.name.clone(), binding);
        } else if module_path.starts_with("std.") || module_path.starts_with("support.") {
            // Tea stdlib modules or unknown modules - will be validated during module expansion
            // For now, register as an empty module binding
            let binding = ModuleBinding::default();
            self.module_aliases
                .insert(use_stmt.alias.name.clone(), binding);
        }
    }

    fn std_type_to_type(&self, ty: StdType) -> Type {
        match ty {
            StdType::Any => Type::Unknown,
            StdType::Bool => Type::Bool,
            StdType::Int => Type::Int,
            StdType::Float => Type::Float,
            StdType::String => Type::String,
            StdType::List => Type::List(Box::new(Type::Unknown)),
            StdType::Dict => Type::Dict(Box::new(Type::Unknown)),
            StdType::Struct => Type::Unknown,
            StdType::Nil => Type::Nil,
            StdType::Void => Type::Void,
        }
    }

    fn parse_type(&mut self, type_expr: &TypeExpression) -> Option<Type> {
        if type_expr.tokens.is_empty() {
            self.report_error("missing type annotation after ':'", None);
            return None;
        }

        let type_parameters = self.current_type_parameters();
        let mut parser = TypeAnnotationParser::new(
            &type_expr.tokens,
            &type_parameters,
            &self.structs,
            &self.unions,
            &self.enums,
            &self.errors,
        );
        match parser.parse_type() {
            Ok(ty) => {
                if parser.is_at_end() {
                    Some(ty)
                } else {
                    if let Some(token) = parser.peek() {
                        self.diagnostics.push_with_location(
                            format!("unexpected token '{}' in type annotation", token.lexeme),
                            token.line,
                            token.column,
                        );
                    } else {
                        self.report_error("unexpected end of type annotation", None);
                    }
                    None
                }
            }
            Err(error) => {
                if let (Some(line), Some(column)) = (error.line, error.column) {
                    self.diagnostics
                        .push_with_location(error.message, line, column);
                } else {
                    self.report_error(error.message, None);
                }
                None
            }
        }
    }

    fn parse_type_argument_expressions(&mut self, expressions: &[TypeExpression]) -> Vec<Type> {
        let mut types = Vec::with_capacity(expressions.len());
        for expr in expressions {
            let parsed = self.parse_type(expr).unwrap_or(Type::Unknown);
            types.push(parsed);
        }
        types
    }

    fn infer_expression(&mut self, expression: &Expression) -> Type {
        match &expression.kind {
            ExpressionKind::Literal(literal) => self.type_from_literal(literal),
            ExpressionKind::InterpolatedString(template) => {
                for part in &template.parts {
                    if let InterpolatedStringPart::Expression(expr) = part {
                        self.infer_expression(expr);
                    }
                }
                Type::String
            }
            ExpressionKind::Identifier(identifier) => self
                .lookup(&identifier.name)
                .or_else(|| {
                    self.functions
                        .get(&identifier.name)
                        .map(|sig| sig.return_type.clone())
                })
                .or_else(|| {
                    self.builtins
                        .contains_key(&identifier.name)
                        .then_some(Type::Nil)
                })
                .or_else(|| {
                    self.enums.contains_key(&identifier.name).then(|| {
                        Type::Enum(EnumType {
                            name: identifier.name.clone(),
                        })
                    })
                })
                .or_else(|| {
                    self.structs.contains_key(&identifier.name).then(|| {
                        Type::Struct(StructType {
                            name: identifier.name.clone(),
                            type_arguments: Vec::new(),
                        })
                    })
                })
                .unwrap_or(Type::Unknown),
            ExpressionKind::Unary(unary) => self.type_from_unary(unary, expression.span),
            ExpressionKind::Binary(binary) => self.type_from_binary(binary, expression.span),
            ExpressionKind::Is(is_expr) => {
                let value_type = self.infer_expression(&is_expr.value);
                let target_type = self
                    .parse_type(&is_expr.type_annotation)
                    .unwrap_or(Type::Unknown);
                self.validate_type_test(&value_type, &target_type, is_expr.type_span);
                if !matches!(target_type, Type::Unknown) {
                    self.type_test_metadata
                        .insert(expression.span, target_type.clone());
                }
                Type::Bool
            }
            ExpressionKind::Assignment(assignment) => {
                if let ExpressionKind::Identifier(identifier) = &assignment.target.kind {
                    let value_type = self.infer_expression(&assignment.value);
                    self.assign(&identifier.name, value_type.clone(), Some(identifier.span));
                    value_type
                } else {
                    Type::Unknown
                }
            }
            ExpressionKind::Grouping(expr) => self.infer_expression(expr),
            ExpressionKind::Call(call) => self.type_from_call(call, expression.span),
            ExpressionKind::Lambda(lambda) => self.type_from_lambda(lambda),
            ExpressionKind::List(list) => self.type_from_list(list, expression.span),
            ExpressionKind::Dict(dict) => self.type_from_dict(dict, expression.span),
            ExpressionKind::Member(member) => self.type_from_member(member, expression.span),
            ExpressionKind::Conditional(cond) => self.type_from_conditional(cond, expression.span),
            ExpressionKind::Match(match_expr) => self.type_from_match(match_expr, expression.span),
            ExpressionKind::Try(try_expr) => self.type_from_try(try_expr, expression.span),
            ExpressionKind::Index(index) => self.type_from_index(index, expression.span),
            ExpressionKind::Unwrap(inner) => self.type_from_unwrap(inner, expression.span),
            ExpressionKind::Range(_) => Type::Unknown,
        }
    }

    fn type_from_conditional(
        &mut self,
        expression: &ConditionalExpression,
        span: SourceSpan,
    ) -> Type {
        // Check condition is boolean
        let condition_type = self.infer_expression(&expression.condition);
        if !matches!(condition_type, Type::Bool | Type::Unknown) {
            self.report_error(
                format!(
                    "if-expression condition must be boolean, found {}",
                    condition_type.describe()
                ),
                Some(expression.condition.span),
            );
        }

        // Infer types of both branches
        let consequent_type = self.infer_expression(&expression.consequent);
        let alternative_type = self.infer_expression(&expression.alternative);

        // Both branches must have compatible types - for now, just check exact equality
        // TODO: Implement proper type unification for optionals and unions
        if consequent_type != alternative_type
            && !matches!(consequent_type, Type::Unknown)
            && !matches!(alternative_type, Type::Unknown)
        {
            self.report_error(
                format!(
                    "if-expression branches must have compatible types, found {} and {}",
                    consequent_type.describe(),
                    alternative_type.describe()
                ),
                Some(span),
            );
            Type::Unknown
        } else if matches!(consequent_type, Type::Unknown) {
            alternative_type
        } else {
            consequent_type
        }
    }

    fn type_from_match(&mut self, expression: &MatchExpression, span: SourceSpan) -> Type {
        let scrutinee_type = self.infer_expression(&expression.scrutinee);
        let mut result_type: Option<Type> = None;

        let enum_variant_total = if let Type::Enum(enum_type) = &scrutinee_type {
            self.enums
                .get(&enum_type.name)
                .map(|definition| definition.variants.len())
        } else {
            None
        };

        let union_members = if let Type::Union(union_type) = &scrutinee_type {
            Some(self.flattened_union_members(&union_type.name))
        } else {
            None
        };

        let mut coverage_complete = false;
        let mut matched_bool_values: HashSet<bool> = HashSet::new();
        let mut matched_enum_variants: HashSet<String> = HashSet::new();
        let mut matched_union_members: HashSet<Type> = HashSet::new();
        let mut coverage_due_to_wildcard = false;

        for arm in &expression.arms {
            let arm_reachable = !coverage_complete;
            let arm_is_wildcard_only = arm
                .patterns
                .iter()
                .all(|pattern| matches!(pattern, MatchPattern::Wildcard { .. }));
            let suppress_unreachable =
                !arm_reachable && arm_is_wildcard_only && !coverage_due_to_wildcard;

            if !arm_reachable && !suppress_unreachable {
                self.report_warning(
                    "match arm is unreachable; previous patterns cover all values",
                    Some(arm.span),
                );
            }

            let mut arm_adds_coverage = false;

            for pattern in &arm.patterns {
                match pattern {
                    MatchPattern::Wildcard { span: pattern_span } => {
                        if suppress_unreachable {
                            continue;
                        }

                        if !arm_reachable {
                            self.report_warning(
                                "pattern is unreachable; previous patterns cover all values",
                                Some(*pattern_span),
                            );
                            continue;
                        }

                        if coverage_complete {
                            if coverage_due_to_wildcard {
                                self.report_warning(
                                    "pattern is unreachable; previous patterns cover all values",
                                    Some(*pattern_span),
                                );
                            }
                        } else {
                            arm_adds_coverage = true;
                            coverage_complete = true;
                            coverage_due_to_wildcard = true;
                        }
                    }
                    MatchPattern::Type(type_expr, pattern_span) => {
                        let target_type = self.parse_type(type_expr).unwrap_or(Type::Unknown);

                        self.validate_type_test(&scrutinee_type, &target_type, *pattern_span);

                        if !matches!(target_type, Type::Unknown) {
                            self.type_test_metadata
                                .insert(*pattern_span, target_type.clone());
                        }

                        if suppress_unreachable || !arm_reachable {
                            continue;
                        }

                        if coverage_complete {
                            self.report_warning(
                                "pattern is unreachable; previous patterns cover all values",
                                Some(*pattern_span),
                            );
                            continue;
                        }

                        if let (Some(members), Type::Union(_)) =
                            (union_members.as_ref(), &scrutinee_type)
                        {
                            let mut matched_any = false;
                            if let Type::Union(inner_union) = &target_type {
                                let target_members =
                                    self.flattened_union_members(&inner_union.name);
                                for member in target_members {
                                    if members.contains(&member) {
                                        matched_any = true;
                                        matched_union_members.insert(member);
                                    }
                                }
                            } else if members.contains(&target_type) {
                                matched_any = true;
                                matched_union_members.insert(target_type.clone());
                            }

                            if matched_any {
                                arm_adds_coverage = true;
                                if members.is_subset(&matched_union_members) {
                                    coverage_complete = true;
                                }
                            }
                        }
                    }
                    MatchPattern::Expression(pattern_expr) => {
                        let pattern_type = self.infer_expression(pattern_expr);
                        if scrutinee_type != Type::Unknown
                            && pattern_type != Type::Unknown
                            && pattern_type != scrutinee_type
                        {
                            self.report_error(
                                format!(
                                    "pattern type '{}' is incompatible with scrutinee type '{}'",
                                    pattern_type.describe(),
                                    scrutinee_type.describe()
                                ),
                                Some(pattern_expr.span),
                            );
                        }

                        if suppress_unreachable {
                            continue;
                        }

                        if !arm_reachable {
                            continue;
                        }

                        if coverage_complete {
                            self.report_warning(
                                "pattern is unreachable; previous patterns cover all values",
                                Some(pattern_expr.span),
                            );
                            continue;
                        }

                        match (&scrutinee_type, &pattern_expr.kind) {
                            (Type::Bool, ExpressionKind::Literal(Literal::Boolean(value))) => {
                                if !matched_bool_values.insert(*value) {
                                    self.report_warning(
                                        format!(
                                            "pattern `{}` is unreachable; value already matched",
                                            value
                                        ),
                                        Some(pattern_expr.span),
                                    );
                                } else {
                                    arm_adds_coverage = true;
                                    if matched_bool_values.len() == 2 {
                                        coverage_complete = true;
                                    }
                                }
                            }
                            (Type::Enum(enum_type), _) => {
                                let variant_name =
                                    if let ExpressionKind::Member(member) = &pattern_expr.kind {
                                        if let ExpressionKind::Identifier(identifier) =
                                            &member.object.kind
                                        {
                                            if identifier.name == enum_type.name {
                                                Some(member.property.clone())
                                            } else {
                                                None
                                            }
                                        } else {
                                            None
                                        }
                                    } else {
                                        None
                                    };

                                if let Some(variant) = variant_name {
                                    if !matched_enum_variants.insert(variant.clone()) {
                                        self.report_warning(
                                            format!(
                                                "pattern `{}.{}` is unreachable; variant already matched",
                                                enum_type.name, variant
                                            ),
                                            Some(pattern_expr.span),
                                        );
                                    } else {
                                        arm_adds_coverage = true;
                                        if let Some(total) = enum_variant_total {
                                            if matched_enum_variants.len() == total {
                                                coverage_complete = true;
                                            }
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }

            let coverage_trackable =
                coverage_due_to_wildcard || matches!(scrutinee_type, Type::Bool | Type::Enum(_));
            if !arm_adds_coverage && arm_reachable && coverage_trackable {
                self.report_warning(
                    "match arm is unreachable; previous patterns cover all values",
                    Some(arm.span),
                );
            }

            let arm_type = self.infer_expression(&arm.expression);
            match &mut result_type {
                Some(existing) => {
                    if *existing == Type::Unknown {
                        *existing = arm_type.clone();
                    } else if arm_type != Type::Unknown && arm_type != *existing {
                        self.report_error(
                            format!(
                                "match arm returns '{}', expected '{}'",
                                arm_type.describe(),
                                existing.describe()
                            ),
                            Some(arm.expression.span),
                        );
                    }
                }
                None => result_type = Some(arm_type.clone()),
            }
        }

        let mut missing_patterns: Vec<String> = Vec::new();
        if !coverage_complete {
            match &scrutinee_type {
                Type::Bool => {
                    if !matched_bool_values.contains(&true) {
                        missing_patterns.push("true".to_string());
                    }
                    if !matched_bool_values.contains(&false) {
                        missing_patterns.push("false".to_string());
                    }
                }
                Type::Enum(enum_type) => {
                    if let Some(definition) = self.enums.get(&enum_type.name) {
                        for variant in &definition.variants {
                            if !matched_enum_variants.contains(&variant.name) {
                                missing_patterns
                                    .push(format!("{}.{}", enum_type.name, variant.name));
                            }
                        }
                    }
                }
                Type::Union(_union_type) => {
                    if let Some(members) = union_members.as_ref() {
                        for member in members {
                            if !matched_union_members.contains(member) {
                                missing_patterns.push(member.describe());
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        if coverage_complete {
            self.match_exhaustiveness.remove(&span);
        } else if !missing_patterns.is_empty() {
            let preview = missing_patterns
                .iter()
                .take(3)
                .map(|case| format!("`{case}`"))
                .collect::<Vec<_>>()
                .join(", ");
            let message = if missing_patterns.len() <= 3 {
                format!("match expression is not exhaustive; missing {preview}")
            } else {
                format!(
                    "match expression is not exhaustive; missing {} cases (e.g. {})",
                    missing_patterns.len(),
                    preview
                )
            };
            self.report_error(message, Some(span));
            self.match_exhaustiveness.insert(span, missing_patterns);
        } else {
            self.report_error(
                "match expression is not exhaustive; add `_` arm or cover all values".to_string(),
                Some(span),
            );
            self.match_exhaustiveness.remove(&span);
        }

        result_type.unwrap_or_else(|| {
            self.report_error(
                "match expression without cases has unknown type".to_string(),
                Some(span),
            );
            Type::Unknown
        })
    }

    fn type_from_list(&mut self, list: &ListLiteral, _span: SourceSpan) -> Type {
        if list.elements.is_empty() {
            return Type::List(Box::new(Type::Unknown));
        }

        let mut element_type = Type::Unknown;
        let mut has_nil = false;

        for (index, element) in list.elements.iter().enumerate() {
            let value_type = self.infer_expression(element);

            if matches!(value_type, Type::Nil) {
                has_nil = true;
                continue;
            }

            element_type = self.merge_binding_type(
                element_type,
                value_type,
                &format!("list element {}", index + 1),
                Some(element.span),
            );
        }

        if matches!(element_type, Type::Unknown) && has_nil {
            return Type::List(Box::new(Type::Nil));
        }

        if has_nil && !matches!(element_type, Type::Unknown) {
            return Type::List(Box::new(Type::Optional(Box::new(element_type))));
        }

        Type::List(Box::new(element_type))
    }

    fn validate_list_elements_against_type(
        &mut self,
        list: &ListLiteral,
        expected_elem_type: &Type,
        context: &str,
    ) {
        for (index, element) in list.elements.iter().enumerate() {
            let elem_type = self.infer_expression(element);
            self.ensure_compatible(
                expected_elem_type,
                &elem_type,
                &format!("{} element {}", context, index + 1),
                Some(element.span),
            );
        }
    }

    fn validate_dict_values_against_type(
        &mut self,
        dict: &DictLiteral,
        expected_value_type: &Type,
        context: &str,
    ) {
        for (index, entry) in dict.entries.iter().enumerate() {
            let value_type = self.infer_expression(&entry.value);
            self.ensure_compatible(
                expected_value_type,
                &value_type,
                &format!("{} value {}", context, index + 1),
                Some(entry.value.span),
            );
        }
    }

    fn infer_argument_with_expected_type(
        &mut self,
        arg: &Expression,
        expected: Option<&Type>,
    ) -> Type {
        if let (Some(Type::List(expected_elem)), ExpressionKind::List(_)) = (expected, &arg.kind) {
            if matches!(expected_elem.as_ref(), Type::Union(_)) {
                self.suppress_list_element_errors = true;
                let result = self.infer_expression(arg);
                self.suppress_list_element_errors = false;
                return result;
            }
        }
        self.infer_expression(arg)
    }

    fn type_from_dict(&mut self, dict: &DictLiteral, _span: SourceSpan) -> Type {
        let mut value_type = Type::Unknown;
        for entry in &dict.entries {
            let actual = self.infer_expression(&entry.value);
            value_type = match (&value_type, &actual) {
                (Type::Unknown, _) => actual.clone(),
                (_, Type::Unknown) => value_type.clone(),
                (left, right) if left == right => left.clone(),
                _ => Type::Unknown,
            };
        }
        Type::Dict(Box::new(value_type))
    }

    fn type_from_index(&mut self, index: &IndexExpression, span: SourceSpan) -> Type {
        let object_type = self.infer_expression(&index.object);
        let index_type = self.infer_expression(&index.index);

        match object_type {
            Type::List(element_type) => {
                // Check if it's a range (slice) or single index
                if let ExpressionKind::Range(range_expr) = &index.index.kind {
                    // Validate that range start and end are Int
                    let start_type = self.infer_expression(&range_expr.start);
                    let end_type = self.infer_expression(&range_expr.end);
                    let int_type = Type::Int;
                    self.ensure_compatible(&int_type, &start_type, "range start", Some(span));
                    self.ensure_compatible(&int_type, &end_type, "range end", Some(span));
                    // Return a list of the same element type
                    Type::List(element_type.clone())
                } else {
                    // Single element index
                    let int_type = Type::Int;
                    self.ensure_compatible(&int_type, &index_type, "list index", Some(span));
                    (*element_type).clone()
                }
            }
            Type::Dict(value_type) => {
                let string_type = Type::String;
                self.ensure_compatible(&string_type, &index_type, "dict index", Some(span));
                (*value_type).clone()
            }
            Type::String => {
                // Check if it's a range (slice) or single index
                if let ExpressionKind::Range(range_expr) = &index.index.kind {
                    // Validate that range start and end are Int
                    let start_type = self.infer_expression(&range_expr.start);
                    let end_type = self.infer_expression(&range_expr.end);
                    let int_type = Type::Int;
                    self.ensure_compatible(&int_type, &start_type, "range start", Some(span));
                    self.ensure_compatible(&int_type, &end_type, "range end", Some(span));
                    Type::String
                } else {
                    // Single character index
                    let int_type = Type::Int;
                    self.ensure_compatible(&int_type, &index_type, "string index", Some(span));
                    Type::String
                }
            }
            Type::Unknown => Type::Unknown,
            other => {
                self.report_error(
                    format!(
                        "indexing requires a list, dict, or string value, found {}",
                        other.describe()
                    ),
                    Some(span),
                );
                Type::Unknown
            }
        }
    }

    fn type_from_unwrap(&mut self, expression: &Expression, span: SourceSpan) -> Type {
        let operand_type = self.infer_expression(expression);
        match operand_type {
            Type::Optional(inner) => {
                match &expression.kind {
                    ExpressionKind::Identifier(identifier) => {
                        if !self.is_non_nil_fact(&identifier.name) {
                            self.report_error(
                                format!(
                                    "cannot unwrap optional '{}': value may be nil here",
                                    identifier.name
                                ),
                                Some(span),
                            );
                        }
                    }
                    _ => {
                        self.report_error(
                            "cannot unwrap optional value; value may be nil here",
                            Some(span),
                        );
                    }
                }
                *inner
            }
            Type::Unknown => Type::Unknown,
            other => {
                self.report_error(
                    format!(
                        "unwrap operator '!' requires an optional value, found {}",
                        other.describe()
                    ),
                    Some(span),
                );
                Type::Unknown
            }
        }
    }

    fn type_from_member(
        &mut self,
        member: &crate::ast::MemberExpression,
        member_span: SourceSpan,
    ) -> Type {
        if let ExpressionKind::Identifier(identifier) = &member.object.kind {
            if let Some(binding) = self.module_aliases.get(&identifier.name) {
                if binding.functions.contains_key(&member.property) {
                    return Type::Unknown;
                } else {
                    self.report_error(
                        format!(
                            "module '{}' has no member '{}'",
                            identifier.name, member.property
                        ),
                        Some(member.property_span),
                    );
                    return Type::Unknown;
                }
            }
        }

        let object_type = self.infer_expression(&member.object);
        match object_type {
            Type::Dict(value_type) => (*value_type).clone(),
            Type::Struct(ref struct_type) => {
                if let Some(definition) = self.structs.get(&struct_type.name) {
                    if let Some(field) = definition.field(&member.property) {
                        if definition.type_parameters.len() != struct_type.type_arguments.len() {
                            self.report_error(
                                format!(
                                    "struct '{}' is missing concrete type arguments",
                                    struct_type.name
                                ),
                                Some(member.property_span),
                            );
                            return Type::Unknown;
                        }

                        let mapping: HashMap<String, Type> = definition
                            .type_parameters
                            .iter()
                            .cloned()
                            .zip(struct_type.type_arguments.iter().cloned())
                            .collect();
                        self.substitute_type(&field.ty, &mapping)
                    } else {
                        self.report_error(
                            format!(
                                "struct '{}' has no field named '{}'",
                                struct_type.name, member.property
                            ),
                            Some(member.property_span),
                        );
                        Type::Unknown
                    }
                } else {
                    self.report_error(
                        format!("unknown struct '{}'", struct_type.name),
                        Some(member.property_span),
                    );
                    Type::Unknown
                }
            }
            Type::Error(ref error_type) => {
                let Some(definition) = self.errors.get(&error_type.name) else {
                    self.report_error(
                        format!("unknown error '{}'", error_type.name),
                        Some(member.property_span),
                    );
                    return Type::Unknown;
                };

                let Some(variant_name) = error_type.variant.as_ref() else {
                    self.report_error(
                        format!(
                            "cannot access field '{}' without matching a specific variant",
                            member.property
                        ),
                        Some(member.property_span),
                    );
                    return Type::Unknown;
                };

                if let Some(variant) = definition.variants.get(variant_name) {
                    if let Some(field) = variant
                        .fields
                        .iter()
                        .find(|field| field.name == member.property)
                    {
                        return field.ty.clone();
                    } else {
                        self.report_error(
                            format!(
                                "error variant '{}.{}' has no field named '{}'",
                                error_type.name, variant_name, member.property
                            ),
                            Some(member.property_span),
                        );
                        return Type::Unknown;
                    }
                } else {
                    self.report_error(
                        format!(
                            "error '{}' has no variant named '{}'",
                            error_type.name, variant_name
                        ),
                        Some(member.property_span),
                    );
                    return Type::Unknown;
                }
            }
            Type::Enum(ref enum_type) => {
                if let ExpressionKind::Identifier(identifier) = &member.object.kind {
                    if identifier.name == enum_type.name {
                        if let Some(definition) = self.enums.get(&enum_type.name) {
                            if let Some(_variant) = definition.variant(&member.property) {
                                return Type::Enum(enum_type.clone());
                            } else {
                                self.report_error(
                                    format!(
                                        "enum '{}' has no variant named '{}'",
                                        enum_type.name, member.property
                                    ),
                                    Some(member.property_span),
                                );
                                return Type::Unknown;
                            }
                        } else {
                            self.report_error(
                                format!("unknown enum '{}'", enum_type.name),
                                Some(member.property_span),
                            );
                            return Type::Unknown;
                        }
                    } else {
                        self.report_error(
                            format!(
                                "enum value '{}' has no fields; reference variants as '{}.{}'",
                                identifier.name, enum_type.name, member.property
                            ),
                            Some(member.property_span),
                        );
                        return Type::Unknown;
                    }
                } else {
                    self.report_error(
                        format!(
                            "enum '{}' variants must be accessed using '{}.<variant>'",
                            enum_type.name, enum_type.name
                        ),
                        Some(member.property_span),
                    );
                    return Type::Unknown;
                }
            }
            Type::Unknown => Type::Unknown,
            other => {
                self.report_error(
                    format!(
                        "member access requires a dictionary or struct value, found {}",
                        other.describe()
                    ),
                    Some(member_span),
                );
                Type::Unknown
            }
        }
    }

    fn type_from_literal(&self, literal: &Literal) -> Type {
        match literal {
            Literal::Integer(_) => Type::Int,
            Literal::Float(_) => Type::Float,
            Literal::String(_) => Type::String,
            Literal::Boolean(_) => Type::Bool,
            Literal::Nil => Type::Nil,
        }
    }

    fn type_from_unary(&mut self, unary: &UnaryExpression, span: SourceSpan) -> Type {
        let operand_type = self.infer_expression(&unary.operand);
        match unary.operator {
            UnaryOperator::Positive | UnaryOperator::Negative => {
                if operand_type != Type::Unknown && !operand_type.is_numeric() {
                    self.report_error(
                        format!(
                            "unary +/- expects a numeric operand, found {}",
                            operand_type.describe()
                        ),
                        Some(span),
                    );
                }
                operand_type
            }
            UnaryOperator::Not => {
                if operand_type != Type::Unknown && operand_type != Type::Bool {
                    self.report_error(
                        format!(
                            "'not' expects a Bool operand, found {}",
                            operand_type.describe()
                        ),
                        Some(span),
                    );
                }
                Type::Bool
            }
        }
    }

    fn type_from_try(&mut self, expression: &TryExpression, _span: SourceSpan) -> Type {
        let inner_type = self.infer_expression(&expression.expression);
        if let Some(clause) = &expression.catch {
            match &clause.kind {
                CatchKind::Fallback(expr) => {
                    let fallback_type = self.infer_expression(expr);
                    if inner_type != Type::Unknown && fallback_type != Type::Unknown {
                        self.ensure_compatible(
                            &inner_type,
                            &fallback_type,
                            "catch fallback expression",
                            Some(expr.span),
                        );
                    }
                }
                CatchKind::Arms(arms) => {
                    if let Some(binding) = &clause.binding {
                        for arm in arms {
                            self.push_scope();
                            if let Some(error_type) = self.infer_binding_error_type(&arm.patterns) {
                                let ty = Type::Error(error_type.clone());
                                self.insert(binding.name.clone(), ty.clone(), true);
                                self.binding_types.insert(binding.span, ty);
                            } else {
                                let mut existing = self
                                    .binding_types
                                    .get(&binding.span)
                                    .cloned()
                                    .unwrap_or(Type::Unknown);
                                if let Type::Error(error_type) = &mut existing {
                                    error_type.variant = None;
                                }
                                self.insert(binding.name.clone(), existing.clone(), true);
                                self.binding_types.insert(binding.span, existing);
                            }
                            self.analyze_catch_patterns(&arm.patterns);
                            match &arm.handler {
                                CatchHandler::Expression(expr) => {
                                    let arm_type = self.infer_expression(expr);
                                    if inner_type != Type::Unknown && arm_type != Type::Unknown {
                                        self.ensure_compatible(
                                            &inner_type,
                                            &arm_type,
                                            "catch arm expression",
                                            Some(expr.span),
                                        );
                                    }
                                }
                                CatchHandler::Block(block) => {
                                    self.run_branch(|checker| {
                                        checker.check_statements(&block.statements);
                                    });
                                }
                            }
                            self.pop_scope();
                        }
                    } else {
                        for arm in arms {
                            self.analyze_catch_patterns(&arm.patterns);
                            match &arm.handler {
                                CatchHandler::Expression(expr) => {
                                    let arm_type = self.infer_expression(expr);
                                    if inner_type != Type::Unknown && arm_type != Type::Unknown {
                                        self.ensure_compatible(
                                            &inner_type,
                                            &arm_type,
                                            "catch arm expression",
                                            Some(expr.span),
                                        );
                                    }
                                }
                                CatchHandler::Block(block) => {
                                    self.run_branch(|checker| {
                                        checker.check_statements(&block.statements);
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
        inner_type
    }

    fn type_from_binary(&mut self, binary: &BinaryExpression, span: SourceSpan) -> Type {
        let left = self.infer_expression(&binary.left);
        let right = self.infer_expression(&binary.right);
        match binary.operator {
            BinaryOperator::Add => {
                // Handle string concatenation
                if matches!(left, Type::String) && matches!(right, Type::String) {
                    return Type::String;
                }

                // Handle list concatenation
                if let (Type::List(left_elem), Type::List(right_elem)) = (&left, &right) {
                    // Lists must have compatible element types
                    if left_elem.as_ref() != right_elem.as_ref()
                        && *left_elem.as_ref() != Type::Unknown
                        && *right_elem.as_ref() != Type::Unknown
                    {
                        self.report_error(
                            format!(
                                "list concatenation requires compatible element types, found List<{}> and List<{}>",
                                left_elem.describe(),
                                right_elem.describe()
                            ),
                            Some(span),
                        );
                        return Type::Unknown;
                    }
                    // Return list with the more specific type (prefer non-Unknown)
                    if *left_elem.as_ref() != Type::Unknown {
                        return left;
                    } else {
                        return right;
                    }
                }

                // Handle numeric addition
                if left != Type::Unknown && !left.is_numeric() {
                    self.report_error(
                        format!(
                            "addition requires numeric, string, or list operands, found {}",
                            left.describe()
                        ),
                        Some(binary.left.span),
                    );
                }
                if right != Type::Unknown && !right.is_numeric() {
                    self.report_error(
                        format!(
                            "addition requires numeric, string, or list operands, found {}",
                            right.describe()
                        ),
                        Some(binary.right.span),
                    );
                }

                if left.is_numeric() && right.is_numeric() {
                    if matches!(left, Type::Float) || matches!(right, Type::Float) {
                        Type::Float
                    } else {
                        Type::Int
                    }
                } else {
                    Type::Unknown
                }
            }
            BinaryOperator::Subtract
            | BinaryOperator::Multiply
            | BinaryOperator::Divide
            | BinaryOperator::Modulo => {
                if left != Type::Unknown && !left.is_numeric() {
                    self.report_error(
                        format!(
                            "arithmetic operations require numeric operands, found {}",
                            left.describe()
                        ),
                        Some(binary.left.span),
                    );
                }
                if right != Type::Unknown && !right.is_numeric() {
                    self.report_error(
                        format!(
                            "arithmetic operations require numeric operands, found {}",
                            right.describe()
                        ),
                        Some(binary.right.span),
                    );
                }

                if left.is_numeric() && right.is_numeric() {
                    if matches!(left, Type::Float) || matches!(right, Type::Float) {
                        Type::Float
                    } else {
                        Type::Int
                    }
                } else {
                    Type::Unknown
                }
            }
            BinaryOperator::Coalesce => match left {
                Type::Optional(inner) => {
                    self.ensure_compatible(
                        inner.as_ref(),
                        &right,
                        "coalesce right operand",
                        Some(binary.right.span),
                    );
                    *inner
                }
                Type::Unknown => Type::Unknown,
                other => {
                    self.report_error(
                        format!(
                            "coalesce left operand must be optional, found {}",
                            other.describe()
                        ),
                        Some(binary.left.span),
                    );
                    Type::Unknown
                }
            },
            BinaryOperator::Equal | BinaryOperator::NotEqual => {
                self.ensure_compatible(&left, &right, "equality comparison", Some(span));
                Type::Bool
            }
            BinaryOperator::Greater
            | BinaryOperator::GreaterEqual
            | BinaryOperator::Less
            | BinaryOperator::LessEqual => {
                if left != Type::Unknown && !left.is_numeric() {
                    self.report_error(
                        format!(
                            "comparison expects numeric operands, found {}",
                            left.describe()
                        ),
                        Some(binary.left.span),
                    );
                }
                if right != Type::Unknown && !right.is_numeric() {
                    self.report_error(
                        format!(
                            "comparison expects numeric operands, found {}",
                            right.describe()
                        ),
                        Some(binary.right.span),
                    );
                }
                Type::Bool
            }
            BinaryOperator::And | BinaryOperator::Or => {
                let bool_type = Type::Bool;
                self.ensure_compatible(
                    &bool_type,
                    &left,
                    "logical operand",
                    Some(binary.left.span),
                );
                self.ensure_compatible(
                    &bool_type,
                    &right,
                    "logical operand",
                    Some(binary.right.span),
                );
                Type::Bool
            }
        }
    }

    fn type_from_call(&mut self, call: &crate::ast::CallExpression, span: SourceSpan) -> Type {
        if let ExpressionKind::Identifier(identifier) = &call.callee.kind {
            if let Some(error_def) = self.errors.get(&identifier.name) {
                if error_def.variants.len() == 1 {
                    if let Some((variant_name, variant_def)) = error_def
                        .variants
                        .iter()
                        .next()
                        .map(|(name, def)| (name.clone(), def.clone()))
                    {
                        return self.type_from_error_constructor(
                            &identifier.name,
                            &variant_name,
                            &variant_def,
                            call,
                            span,
                        );
                    }
                } else {
                    self.report_error(
                        format!(
                            "error '{}' has multiple variants; call a specific variant like '{}.<variant>'",
                            identifier.name, identifier.name
                        ),
                        Some(identifier.span),
                    );
                    return Type::Unknown;
                }
            }

            if let Some(struct_def) = self.structs.get(&identifier.name).cloned() {
                return self.type_from_struct_call(identifier, struct_def, call, span);
            }
        }

        if let ExpressionKind::Member(member) = &call.callee.kind {
            if let ExpressionKind::Identifier(error_ident) = &member.object.kind {
                if let Some(error_def) = self.errors.get(&error_ident.name) {
                    if let Some(variant_def) = error_def.variants.get(&member.property).cloned() {
                        return self.type_from_error_constructor(
                            &error_ident.name,
                            &member.property,
                            &variant_def,
                            call,
                            span,
                        );
                    } else {
                        self.report_error(
                            format!(
                                "error '{}' has no variant named '{}'",
                                error_ident.name, member.property
                            ),
                            Some(member.property_span),
                        );
                        return Type::Unknown;
                    }
                }
            }
        }

        if let ExpressionKind::Identifier(identifier) = &call.callee.kind {
            if self.enums.contains_key(&identifier.name) {
                self.report_error(
                    format!(
                        "enum '{}' is not callable; use '{}.<variant>' to construct a value",
                        identifier.name, identifier.name
                    ),
                    Some(identifier.span),
                );
                return Type::Unknown;
            }
        }

        if let Some(arg) = call.arguments.iter().find(|arg| arg.name.is_some()) {
            let span = arg.name_span.or(Some(arg.expression.span)).unwrap_or(span);
            self.report_error(
                "named arguments are only supported when constructing structs",
                Some(span),
            );
        }

        let is_non_generic_function_call =
            if let ExpressionKind::Identifier(ident) = &call.callee.kind {
                self.functions
                    .get(&ident.name)
                    .map(|sig| sig.type_parameters.is_empty())
                    .unwrap_or(false)
            } else {
                false
            };

        let arg_types: Vec<Type> = if is_non_generic_function_call {
            Vec::new()
        } else {
            call.arguments
                .iter()
                .map(|arg| self.infer_expression(&arg.expression))
                .collect()
        };
        if let ExpressionKind::Member(member) = &call.callee.kind {
            if let ExpressionKind::Identifier(alias_ident) = &member.object.kind {
                if let Some(binding) = self.module_aliases.get(&alias_ident.name) {
                    if let Some(function) = binding.functions.get(&member.property).cloned() {
                        if !call.type_arguments.is_empty() {
                            self.report_error(
                                format!(
                                    "module function '{}.{}' does not accept type arguments",
                                    alias_ident.name, member.property
                                ),
                                Some(member.property_span),
                            );
                            return Type::Unknown;
                        }
                        let qualified = format!("{}.{}", alias_ident.name, member.property);
                        self.verify_call_arguments(
                            &function.signature.params,
                            &arg_types,
                            &call.arguments,
                            function.signature.arity,
                            Some(&qualified),
                            Some(span),
                        );
                        return function.signature.return_type.clone();
                    } else {
                        self.report_error(
                            format!(
                                "module '{}' has no member '{}'",
                                alias_ident.name, member.property
                            ),
                            Some(member.property_span),
                        );
                        return Type::Unknown;
                    }
                }
            }
        }
        if let ExpressionKind::Identifier(identifier) = &call.callee.kind {
            if let Some(signature) = self.functions.get(&identifier.name).cloned() {
                if signature.type_parameters.is_empty() {
                    let arg_types_for_call: Vec<Type> = call
                        .arguments
                        .iter()
                        .enumerate()
                        .map(|(index, arg)| {
                            let expected = signature.params.get(index);
                            self.infer_argument_with_expected_type(&arg.expression, expected)
                        })
                        .collect();
                    self.verify_call_arguments(
                        &signature.params,
                        &arg_types_for_call,
                        &call.arguments,
                        signature.arity,
                        Some(&identifier.name),
                        Some(span),
                    );
                    return signature.return_type;
                }

                let mut mapping: HashMap<String, Type> = HashMap::new();
                let mut ok = true;

                if !call.type_arguments.is_empty() {
                    let explicit = self.parse_type_argument_expressions(&call.type_arguments);
                    if explicit.len() != signature.type_parameters.len() {
                        let params = signature
                            .type_parameters
                            .iter()
                            .map(|name| format!("<{name}>"))
                            .collect::<Vec<_>>()
                            .join(", ");
                        self.report_error(
                            if signature.type_parameters.is_empty() {
                                format!(
                                    "function '{}' does not accept type arguments ({} provided)",
                                    identifier.name,
                                    explicit.len()
                                )
                            } else {
                                format!(
                                    "function '{}' expects {} type argument{} [{}] but {} provided",
                                    identifier.name,
                                    signature.type_parameters.len(),
                                    if signature.type_parameters.len() == 1 {
                                        ""
                                    } else {
                                        "s"
                                    },
                                    params,
                                    explicit.len()
                                )
                            },
                            Some(span),
                        );
                        ok = false;
                    } else {
                        for (name, ty) in signature
                            .type_parameters
                            .iter()
                            .cloned()
                            .zip(explicit.into_iter())
                        {
                            mapping.insert(name, ty);
                        }
                    }
                }

                for (index, (expected, actual)) in
                    signature.params.iter().zip(arg_types.iter()).enumerate()
                {
                    let arg_span = call.arguments.get(index).map(|arg| arg.expression.span);
                    let context =
                        format!("argument {} to function '{}'", index + 1, identifier.name);
                    if !self.unify_types(expected, actual, &mut mapping, &context, arg_span) {
                        ok = false;
                    }
                }

                for name in &signature.type_parameters {
                    if !mapping.contains_key(name) {
                        let suggestion = format!(
                            "{}[{}]",
                            identifier.name,
                            signature
                                .type_parameters
                                .iter()
                                .map(|param| param.clone())
                                .collect::<Vec<_>>()
                                .join(", ")
                        );
                        self.report_error(
                            format!(
                                "could not infer type for parameter '{}' in call to '{}'; consider specifying type arguments like {}",
                                name, identifier.name, suggestion
                            ),
                            Some(span),
                        );
                        ok = false;
                    }
                }

                if !ok {
                    return Type::Unknown;
                }

                let instantiated_params: Vec<Type> = signature
                    .params
                    .iter()
                    .map(|param| self.substitute_type(param, &mapping))
                    .collect();

                self.verify_call_arguments(
                    &instantiated_params,
                    &arg_types,
                    &call.arguments,
                    signature.arity,
                    Some(&identifier.name),
                    Some(span),
                );

                let instantiated_return = self.substitute_type(&signature.return_type, &mapping);

                let type_arguments = signature
                    .type_parameters
                    .iter()
                    .map(|name| mapping.get(name).cloned().unwrap_or(Type::Unknown))
                    .collect::<Vec<_>>();
                let instance = FunctionInstance {
                    type_arguments,
                    param_types: instantiated_params.clone(),
                    return_type: instantiated_return.clone(),
                };
                self.record_function_instance(span, &identifier.name, instance);

                return instantiated_return;
            }

            if self.builtins.get(&identifier.name).is_some() {
                return Type::Nil;
            }
        }

        let callee_type = self.infer_expression(&call.callee);
        if let Type::Function(params, return_type) = callee_type {
            self.verify_call_arguments(
                &params,
                &arg_types,
                &call.arguments,
                StdArity::Exact(params.len()),
                None,
                Some(span),
            );
            return *return_type;
        }

        if let ExpressionKind::Identifier(identifier) = &call.callee.kind {
            self.report_error(
                format!("call to undefined function '{}'", identifier.name),
                Some(identifier.span),
            );
        }

        Type::Unknown
    }

    fn type_from_struct_call(
        &mut self,
        identifier: &Identifier,
        definition: StructDefinition,
        call: &CallExpression,
        span: SourceSpan,
    ) -> Type {
        let field_count = definition.fields.len();
        let mut assignments: Vec<(&StructFieldType, &CallArgument, Type)> = Vec::new();
        let mut assignment_ok = true;
        if call.arguments.len() != field_count {
            self.report_error(
                format!(
                    "struct '{}' constructor expects {} arguments but got {}",
                    identifier.name,
                    field_count,
                    call.arguments.len()
                ),
                Some(identifier.span),
            );
            assignment_ok = false;
        }

        let all_named = call.arguments.iter().all(|arg| arg.name.is_some());
        let any_named = call.arguments.iter().any(|arg| arg.name.is_some());

        if any_named && !all_named {
            if let Some(arg) = call.arguments.iter().find(|arg| arg.name.is_none()) {
                self.report_error(
                    "cannot mix positional and named arguments when constructing a struct",
                    Some(arg.expression.span),
                );
            }
        }

        if all_named {
            let mut seen = HashSet::new();
            let mut provided = HashSet::new();
            for argument in &call.arguments {
                let arg_name = match &argument.name {
                    Some(name) => name,
                    None => continue,
                };
                if !seen.insert(arg_name.clone()) {
                    let span = argument.name_span.unwrap_or(argument.expression.span);
                    self.report_error(
                        format!(
                            "duplicate field '{}' in struct '{}' constructor",
                            arg_name, identifier.name
                        ),
                        Some(span),
                    );
                    assignment_ok = false;
                    continue;
                }

                match definition.field(arg_name) {
                    Some(field) => {
                        let actual = self.infer_expression(&argument.expression);
                        assignments.push((field, argument, actual));
                        provided.insert(field.name.clone());
                    }
                    None => {
                        let span = argument.name_span.unwrap_or(argument.expression.span);
                        self.report_error(
                            format!(
                                "struct '{}' has no field named '{}'",
                                identifier.name, arg_name
                            ),
                            Some(span),
                        );
                        assignment_ok = false;
                    }
                }
            }

            for field in &definition.fields {
                if !provided.contains(&field.name) {
                    self.report_error(
                        format!(
                            "missing value for field '{}' in struct '{}'",
                            field.name, identifier.name
                        ),
                        Some(identifier.span),
                    );
                    assignment_ok = false;
                }
            }
        } else {
            for (index, (field, argument)) in definition
                .fields
                .iter()
                .zip(call.arguments.iter())
                .enumerate()
            {
                if let Some(name_span) = argument.name_span {
                    self.report_error(
                        format!(
                            "unexpected named argument '{}' in positional struct construction",
                            field.name
                        ),
                        Some(name_span),
                    );
                    assignment_ok = false;
                }

                if index < definition.fields.len() {
                    let actual = self.infer_expression(&argument.expression);
                    assignments.push((field, argument, actual));
                }
            }
        }

        if !assignment_ok {
            return Type::Unknown;
        }

        let mut mapping: HashMap<String, Type> = HashMap::new();
        if !call.type_arguments.is_empty() {
            if definition.type_parameters.is_empty() {
                self.report_error(
                    format!(
                        "struct '{}' does not accept type arguments ({} provided)",
                        identifier.name,
                        call.type_arguments.len()
                    ),
                    Some(identifier.span),
                );
                return Type::Unknown;
            }

            let explicit = self.parse_type_argument_expressions(&call.type_arguments);
            if explicit.len() != definition.type_parameters.len() {
                let params = definition
                    .type_parameters
                    .iter()
                    .map(|param| format!("<{param}>"))
                    .collect::<Vec<_>>()
                    .join(", ");
                self.report_error(
                    format!(
                        "struct '{}' expects {} type argument{} [{}] but {} provided",
                        identifier.name,
                        definition.type_parameters.len(),
                        if definition.type_parameters.len() == 1 {
                            ""
                        } else {
                            "s"
                        },
                        params,
                        explicit.len()
                    ),
                    Some(identifier.span),
                );
                return Type::Unknown;
            }

            for (param, ty) in definition
                .type_parameters
                .iter()
                .cloned()
                .zip(explicit.into_iter())
            {
                mapping.insert(param, ty);
            }
        }

        let mut unified_ok = true;
        for (field, argument, actual) in &assignments {
            let context = format!(
                "value for field '{}' in struct '{}'",
                field.name, identifier.name
            );
            if !self.unify_types(
                &field.ty,
                actual,
                &mut mapping,
                &context,
                Some(argument.expression.span),
            ) {
                unified_ok = false;
            }
        }

        for param in &definition.type_parameters {
            if !mapping.contains_key(param) {
                let suggestion = format!(
                    "{}[{}]",
                    identifier.name,
                    definition
                        .type_parameters
                        .iter()
                        .map(|name| name.clone())
                        .collect::<Vec<_>>()
                        .join(", ")
                );
                self.report_error(
                    format!(
                        "could not infer type for parameter '{}' when constructing '{}'; consider spelling the type arguments explicitly like {}",
                        param, identifier.name, suggestion
                    ),
                    Some(identifier.span),
                );
                unified_ok = false;
            }
        }

        if !unified_ok {
            return Type::Unknown;
        }

        let mut compatible = true;
        for (field, argument, actual) in &assignments {
            let substituted = self.substitute_type(&field.ty, &mapping);
            let context = format!(
                "value for field '{}' in struct '{}'",
                field.name, identifier.name
            );
            if !self.ensure_compatible(
                &substituted,
                actual,
                &context,
                Some(argument.expression.span),
            ) {
                compatible = false;
            }
        }

        if !compatible {
            return Type::Unknown;
        }

        let type_arguments = definition
            .type_parameters
            .iter()
            .map(|name| mapping.get(name).cloned().unwrap_or(Type::Unknown))
            .collect::<Vec<_>>();

        let field_types: Vec<Type> = definition
            .fields
            .iter()
            .map(|field| self.substitute_type(&field.ty, &mapping))
            .collect();
        let instance = StructInstance {
            type_arguments: type_arguments.clone(),
            field_types,
        };
        self.record_struct_instance(span, &identifier.name, instance);

        Type::Struct(StructType {
            name: identifier.name.clone(),
            type_arguments,
        })
    }

    fn type_from_error_constructor(
        &mut self,
        error_name: &str,
        variant_name: &str,
        variant_def: &ErrorVariantDefinition,
        call: &CallExpression,
        span: SourceSpan,
    ) -> Type {
        if call.arguments.iter().any(|arg| arg.name.is_some()) {
            if let Some(argument) = call.arguments.iter().find(|arg| arg.name.is_some()) {
                let arg_span = argument.name_span.unwrap_or(argument.expression.span);
                self.report_error(
                    format!(
                        "error variant '{}.{}' does not accept named arguments",
                        error_name, variant_name
                    ),
                    Some(arg_span),
                );
            }
            return Type::Unknown;
        }

        if call.arguments.len() != variant_def.fields.len() {
            self.report_error(
                format!(
                    "error variant '{}.{}' expects {} argument{}, found {}",
                    error_name,
                    variant_name,
                    variant_def.fields.len(),
                    if variant_def.fields.len() == 1 {
                        ""
                    } else {
                        "s"
                    },
                    call.arguments.len()
                ),
                Some(span),
            );
            return Type::Unknown;
        }

        for (index, field) in variant_def.fields.iter().enumerate() {
            let argument = &call.arguments[index];
            let value_type = self.infer_expression(&argument.expression);
            self.argument_expected_types
                .insert(argument.expression.span, field.ty.clone());
            self.ensure_compatible(
                &field.ty,
                &value_type,
                &format!(
                    "argument {} to error '{}.{}'",
                    index + 1,
                    error_name,
                    variant_name
                ),
                Some(argument.expression.span),
            );
        }

        Type::Error(ErrorType {
            name: error_name.to_string(),
            variant: Some(variant_name.to_string()),
        })
    }

    // Methods for removed json/yaml functionality - kept for potential future use
    #[allow(dead_code)]
    fn type_from_json_decode(
        &mut self,
        call: &CallExpression,
        _span: SourceSpan,
        fallback: Type,
    ) -> Type {
        let default = if matches!(fallback, Type::Unknown) {
            Type::Dict(Box::new(Type::Unknown))
        } else {
            fallback
        };

        if call.arguments.len() != 1 {
            return default;
        }

        let argument = &call.arguments[0].expression;
        if let ExpressionKind::Literal(Literal::String(text)) = &argument.kind {
            match serde_json::from_str::<JsonValue>(text) {
                Ok(value) => json_value_to_type(&value),
                Err(error) => {
                    self.report_error(
                        format!("failed to parse JSON literal passed to json.decode: {error}"),
                        Some(argument.span),
                    );
                    Type::Unknown
                }
            }
        } else {
            default
        }
    }

    #[allow(dead_code)]
    fn type_from_yaml_decode(
        &mut self,
        call: &CallExpression,
        _span: SourceSpan,
        fallback: Type,
    ) -> Type {
        let default = if matches!(fallback, Type::Unknown) {
            Type::Dict(Box::new(Type::Unknown))
        } else {
            fallback
        };

        if call.arguments.len() != 1 {
            return default;
        }

        let argument = &call.arguments[0].expression;
        if let ExpressionKind::Literal(Literal::String(text)) = &argument.kind {
            match serde_yaml::from_str::<YamlValue>(text) {
                Ok(value) => match serde_json::to_value(value) {
                    Ok(json_value) => json_value_to_type(&json_value),
                    Err(error) => {
                        self.report_error(
                            format!(
                                "failed to normalise YAML literal passed to yaml.decode: {error}"
                            ),
                            Some(argument.span),
                        );
                        Type::Unknown
                    }
                },
                Err(error) => {
                    self.report_error(
                        format!("failed to parse YAML literal passed to yaml.decode: {error}"),
                        Some(argument.span),
                    );
                    Type::Unknown
                }
            }
        } else {
            default
        }
    }

    fn verify_call_arguments(
        &mut self,
        expected: &[Type],
        actual: &[Type],
        arguments: &[CallArgument],
        arity: StdArity,
        name: Option<&str>,
        call_span: Option<SourceSpan>,
    ) {
        let context = name
            .map(|function| format!("function '{}'", function))
            .unwrap_or_else(|| "function value".to_string());

        let arg_len = actual.len();
        if !arity.allows(arg_len) {
            let message = match arity {
                StdArity::Exact(count) => format!(
                    "{} expected {} arguments but got {}",
                    context, count, arg_len
                ),
                StdArity::Range { min, max } => {
                    if let Some(limit) = max {
                        if min == limit {
                            format!("{} expected {} arguments but got {}", context, min, arg_len)
                        } else {
                            format!(
                                "{} expected between {} and {} arguments but got {}",
                                context, min, limit, arg_len
                            )
                        }
                    } else {
                        format!(
                            "{} expected at least {} arguments but got {}",
                            context, min, arg_len
                        )
                    }
                }
            };
            let highlight_span = if arg_len > expected.len() {
                arguments
                    .get(expected.len())
                    .map(|arg| arg.expression.span)
                    .or(call_span)
            } else {
                call_span
            };
            self.report_error(message, highlight_span);
            return;
        }

        for (index, actual_ty) in actual.iter().enumerate() {
            if let Some(expected_ty) = expected.get(index) {
                let argument_context = name
                    .map(|function| format!("argument {} to '{}'", index + 1, function))
                    .unwrap_or_else(|| format!("argument {} to function value", index + 1));
                let arg_span = arguments
                    .get(index)
                    .map(|arg| arg.expression.span)
                    .or(call_span);
                if let Some(span) = arg_span {
                    self.argument_expected_types
                        .insert(span, expected_ty.clone());
                }

                if let (Type::List(expected_elem), Some(arg)) = (expected_ty, arguments.get(index))
                {
                    if let ExpressionKind::List(list) = &arg.expression.kind {
                        if matches!(expected_elem.as_ref(), Type::Union(_)) {
                            self.suppress_list_element_errors = true;
                            self.validate_list_elements_against_type(
                                list,
                                expected_elem,
                                &argument_context,
                            );
                            self.suppress_list_element_errors = false;
                            continue;
                        }
                    }
                }

                self.ensure_compatible(expected_ty, actual_ty, &argument_context, arg_span);
            }
        }
    }

    fn type_from_lambda(&mut self, lambda: &LambdaExpression) -> Type {
        let mut param_types = Vec::with_capacity(lambda.parameters.len());
        for param in &lambda.parameters {
            if param.type_annotation.is_none() {
                self.report_error(
                    format!(
                        "parameter '{}' in lambda must have a type annotation",
                        param.name
                    ),
                    Some(param.span),
                );
            }
            let ty = param
                .type_annotation
                .as_ref()
                .and_then(|annotation| self.parse_type(annotation))
                .unwrap_or(Type::Unknown);
            param_types.push(ty);
        }

        self.push_scope();
        for (param, expected_type) in lambda.parameters.iter().zip(param_types.iter()) {
            self.insert(param.name.clone(), expected_type.clone(), true);
            self.binding_types.insert(param.span, expected_type.clone());
            if let Some(default) = &param.default_value {
                let actual = self.infer_expression(default);
                self.ensure_compatible(
                    expected_type,
                    &actual,
                    &format!("default value for parameter '{}' in lambda", param.name),
                    Some(param.span),
                );
            }
        }

        let return_type = match &lambda.body {
            LambdaBody::Expression(expr) => self.infer_expression(expr),
            LambdaBody::Block(block) => {
                self.contexts.push(FunctionContext {
                    return_type: Type::Unknown,
                    allowed_errors: ErrorSet::empty(),
                    saw_explicit_return: false,
                    last_expression_type: None,
                    explicit_return_types: Vec::new(),
                });
                self.check_statements(&block.statements);
                let context = self.contexts.pop().unwrap();

                if !context.explicit_return_types.is_empty() {
                    let mut iter = context.explicit_return_types.into_iter();
                    let mut acc = iter.next().unwrap_or(Type::Void);
                    for ty in iter {
                        acc = self.merge_binding_type(acc, ty, "lambda return type", None);
                    }
                    acc
                } else if let Some(last) = context.last_expression_type {
                    last
                } else if context.saw_explicit_return {
                    Type::Void
                } else {
                    Type::Void
                }
            }
        };

        self.pop_scope();
        let function_type = Type::Function(param_types.clone(), Box::new(return_type.clone()));
        self.lambda_types.insert(lambda.id, function_type.clone());
        function_type
    }

    fn insert(&mut self, name: String, ty: Type, mutable: bool) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.clone(), ty);
        }
        if let Some(scope) = self.const_scopes.last_mut() {
            if mutable {
                scope.remove(&name);
            } else {
                scope.insert(name.clone());
            }
        }
        self.clear_non_nil_fact(&name);
    }

    fn assign(&mut self, name: &str, ty: Type, span: Option<SourceSpan>) {
        for index in (0..self.scopes.len()).rev() {
            if let Some(existing_type) = self.scopes[index].get(name).cloned() {
                if self.const_scopes[index].contains(name) {
                    let message = format!("cannot reassign const '{}'", name);
                    self.report_error(message, span);
                    return;
                }
                let context = format!("assignment to '{}'", name);
                let merged = self.merge_binding_type(existing_type, ty.clone(), &context, span);
                self.scopes[index].insert(name.to_string(), merged);
                self.update_non_nil_fact(name, &ty);
                return;
            }
        }
        self.assign_global(name.to_string(), ty);
    }

    fn assign_global(&mut self, name: String, ty: Type) {
        if let Some(const_scope) = self.const_scopes.first_mut() {
            const_scope.remove(&name);
        }
        if let Some(global) = self.scopes.first_mut() {
            global.insert(name.clone(), ty.clone());
        }
        self.update_non_nil_fact(&name, &ty);
    }

    pub(crate) fn global_binding_types(&self) -> HashMap<String, Type> {
        self.scopes.first().cloned().unwrap_or_else(HashMap::new)
    }

    fn lookup(&self, name: &str) -> Option<Type> {
        for scope in self.scopes.iter().rev() {
            if let Some(ty) = scope.get(name) {
                return Some(ty.clone());
            }
        }
        if self.structs.contains_key(name) {
            return Some(Type::Struct(StructType {
                name: name.to_string(),
                type_arguments: Vec::new(),
            }));
        }
        None
    }

    fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
        self.const_scopes.push(HashSet::new());
        self.non_nil_scopes.push(HashSet::new());
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
        self.const_scopes.pop();
        self.non_nil_scopes.pop();
    }

    fn push_type_parameters(&mut self, params: &[TypeParameter]) {
        if params.is_empty() {
            return;
        }
        let mut scope = HashSet::new();
        for param in params {
            scope.insert(param.name.clone());
        }
        self.type_parameters.push(scope);
    }

    fn pop_type_parameters(&mut self) {
        self.type_parameters.pop();
    }

    fn current_type_parameters(&self) -> HashSet<String> {
        let mut merged = HashSet::new();
        for scope in &self.type_parameters {
            merged.extend(scope.iter().cloned());
        }
        merged
    }
}

struct TypeAnnotationParser<'a> {
    tokens: &'a [Token],
    position: usize,
    type_parameters: &'a HashSet<String>,
    structs: &'a HashMap<String, StructDefinition>,
    unions: &'a HashMap<String, UnionDefinition>,
    enums: &'a HashMap<String, EnumDefinition>,
    errors: &'a HashMap<String, ErrorDefinition>,
}

struct TypeError {
    message: String,
    line: Option<usize>,
    column: Option<usize>,
}

impl TypeError {
    fn at(token: &Token, message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            line: Some(token.line),
            column: Some(token.column),
        }
    }

    fn at_eof(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            line: None,
            column: None,
        }
    }
}

impl<'a> TypeAnnotationParser<'a> {
    fn new(
        tokens: &'a [Token],
        type_parameters: &'a HashSet<String>,
        structs: &'a HashMap<String, StructDefinition>,
        unions: &'a HashMap<String, UnionDefinition>,
        enums: &'a HashMap<String, EnumDefinition>,
        errors: &'a HashMap<String, ErrorDefinition>,
    ) -> Self {
        Self {
            tokens,
            position: 0,
            type_parameters,
            structs,
            unions,
            enums,
            errors,
        }
    }

    fn parse_type(&mut self) -> Result<Type, TypeError> {
        let mut ty = self.parse_primary_type()?;

        while matches!(self.peek_kind(), Some(TokenKind::Question)) {
            self.advance();
            ty = Type::Optional(Box::new(ty));
        }

        Ok(ty)
    }

    fn parse_primary_type(&mut self) -> Result<Type, TypeError> {
        let token = self
            .peek()
            .ok_or_else(|| TypeError::at_eof("expected type annotation"))?;

        match &token.kind {
            TokenKind::Identifier => match token.lexeme.as_str() {
                "Bool" => {
                    self.advance();
                    Ok(Type::Bool)
                }
                "Int" => {
                    self.advance();
                    Ok(Type::Int)
                }
                "Float" => {
                    self.advance();
                    Ok(Type::Float)
                }
                "String" => {
                    self.advance();
                    Ok(Type::String)
                }
                "Nil" => {
                    self.advance();
                    Ok(Type::Nil)
                }
                "Void" => {
                    self.advance();
                    Ok(Type::Void)
                }
                "List" => self.parse_list_type(),
                "Dict" => self.parse_dict_type(),
                "Func" | "Function" | "Fn" => self.parse_function_type(),
                other => {
                    let ident_token = token.clone();
                    self.advance();
                    if self.type_parameters.contains(other) {
                        Ok(Type::GenericParameter(other.to_string()))
                    } else if self.structs.contains_key(other) {
                        let type_arguments =
                            self.parse_struct_type_arguments(other, &ident_token)?;
                        Ok(Type::Struct(StructType {
                            name: other.to_string(),
                            type_arguments,
                        }))
                    } else if self.unions.contains_key(other) {
                        if matches!(self.peek_kind(), Some(TokenKind::LBracket)) {
                            let bracket = self.advance().cloned().unwrap();
                            return Err(TypeError::at(
                                &bracket,
                                format!("union '{}' does not accept type arguments", other),
                            ));
                        }
                        Ok(Type::Union(UnionType {
                            name: other.to_string(),
                        }))
                    } else if self.enums.contains_key(other) {
                        if matches!(self.peek_kind(), Some(TokenKind::LBracket)) {
                            let bracket = self.advance().cloned().unwrap();
                            return Err(TypeError::at(
                                &bracket,
                                format!("enum '{}' does not accept type arguments", other),
                            ));
                        }
                        Ok(Type::Enum(EnumType {
                            name: other.to_string(),
                        }))
                    } else if self.errors.contains_key(other) {
                        if matches!(self.peek_kind(), Some(TokenKind::Dot)) {
                            let _dot = self.advance().cloned().unwrap();
                            let variant_token = self
                                .advance()
                                .cloned()
                                .ok_or_else(|| TypeError::at_eof("expected error variant name"))?;
                            match variant_token.kind {
                                TokenKind::Identifier => {
                                    if let Some(definition) = self.errors.get(other) {
                                        if definition.variants.contains_key(&variant_token.lexeme) {
                                            Ok(Type::Error(ErrorType {
                                                name: other.to_string(),
                                                variant: Some(variant_token.lexeme),
                                            }))
                                        } else {
                                            Err(TypeError::at(
                                                &variant_token,
                                                format!(
                                                    "error '{}' has no variant named '{}'",
                                                    other, variant_token.lexeme
                                                ),
                                            ))
                                        }
                                    } else {
                                        Err(TypeError::at(
                                            &variant_token,
                                            format!("unknown error '{}'", other),
                                        ))
                                    }
                                }
                                _ => Err(TypeError::at(
                                    &variant_token,
                                    format!(
                                        "expected variant name after '{}.', found '{}'",
                                        other, variant_token.lexeme
                                    ),
                                )),
                            }
                        } else {
                            Ok(Type::Error(ErrorType {
                                name: other.to_string(),
                                variant: None,
                            }))
                        }
                    } else {
                        Err(TypeError::at(
                            &ident_token,
                            format!("unknown type '{}'", other),
                        ))
                    }
                }
            },
            TokenKind::Keyword(Keyword::Nil) => {
                self.advance();
                Ok(Type::Nil)
            }
            _ => Err(TypeError::at(
                token,
                format!("unexpected token '{}' in type annotation", token.lexeme),
            )),
        }
    }

    fn parse_list_type(&mut self) -> Result<Type, TypeError> {
        let start = self.advance().unwrap().clone(); // consume 'List'
        self.expect(TokenKind::LBracket, "expected '[' after List", &start)?;
        let element = self.parse_type()?;
        self.expect(
            TokenKind::RBracket,
            "expected ']' after List element type",
            &start,
        )?;
        Ok(Type::List(Box::new(element)))
    }

    fn parse_dict_type(&mut self) -> Result<Type, TypeError> {
        let start = self.advance().unwrap().clone(); // consume 'Dict'
        self.expect(TokenKind::LBracket, "expected '[' after Dict", &start)?;
        let key_type = self.parse_type()?;
        self.expect(
            TokenKind::Comma,
            "expected ',' between Dict key and value types",
            &start,
        )?;
        let value_type = self.parse_type()?;
        self.expect(
            TokenKind::RBracket,
            "expected ']' after Dict value type",
            &start,
        )?;

        match key_type {
            Type::String | Type::Unknown => Ok(Type::Dict(Box::new(value_type))),
            other => Err(TypeError::at(
                &start,
                format!("Dict key type must be String, found {}", other.describe()),
            )),
        }
    }

    fn parse_function_type(&mut self) -> Result<Type, TypeError> {
        let start = self.advance().unwrap().clone(); // consume 'Func'
        self.expect(
            TokenKind::LParen,
            "expected '(' after function type",
            &start,
        )?;
        let mut params = Vec::new();

        if !matches!(self.peek_kind(), Some(TokenKind::RParen)) {
            loop {
                let parameter = self.parse_type()?;
                params.push(parameter);
                if matches!(self.peek_kind(), Some(TokenKind::Comma)) {
                    self.advance();
                } else {
                    break;
                }
            }
        }

        self.expect(
            TokenKind::RParen,
            "expected ')' after function parameter types",
            &start,
        )?;
        self.expect(
            TokenKind::Arrow,
            "expected '->' after function parameter types",
            &start,
        )?;
        let return_type = self.parse_type()?;
        Ok(Type::Function(params, Box::new(return_type)))
    }

    fn parse_struct_type_arguments(
        &mut self,
        name: &str,
        context: &Token,
    ) -> Result<Vec<Type>, TypeError> {
        let expected = self
            .structs
            .get(name)
            .map(|def| def.type_parameters.len())
            .unwrap_or(0);

        if expected == 0 {
            if matches!(self.peek_kind(), Some(TokenKind::LBracket)) {
                let bracket = self.advance().cloned().unwrap();
                return Err(TypeError::at(
                    &bracket,
                    format!("struct '{}' does not accept type arguments", name),
                ));
            }
            return Ok(Vec::new());
        }

        if !matches!(self.peek_kind(), Some(TokenKind::LBracket)) {
            return Err(TypeError::at(
                context,
                format!(
                    "struct '{}' expects {} type argument{}",
                    name,
                    expected,
                    if expected == 1 { "" } else { "s" }
                ),
            ));
        }

        let open_token = self.advance().cloned().unwrap(); // consume '['
        let mut arguments = Vec::new();

        if matches!(self.peek_kind(), Some(TokenKind::RBracket)) {
            return Err(TypeError::at(
                &open_token,
                format!(
                    "struct '{}' expects {} type argument{}",
                    name,
                    expected,
                    if expected == 1 { "" } else { "s" }
                ),
            ));
        }

        loop {
            let argument = self.parse_type()?;
            arguments.push(argument);
            match self.peek_kind() {
                Some(TokenKind::Comma) => {
                    self.advance();
                }
                Some(TokenKind::RBracket) => {
                    self.advance();
                    break;
                }
                Some(_) => {
                    if let Some(token) = self.peek().cloned() {
                        return Err(TypeError::at(
                            &token,
                            format!(
                                "unexpected token '{}' in type arguments for '{}'",
                                token.lexeme, name
                            ),
                        ));
                    } else {
                        return Err(TypeError::at_eof(format!(
                            "expected ']' to close type arguments for '{}'",
                            name
                        )));
                    }
                }
                None => {
                    return Err(TypeError::at_eof(format!(
                        "expected ']' to close type arguments for '{}'",
                        name
                    )));
                }
            }
        }

        if arguments.len() != expected {
            return Err(TypeError::at(
                context,
                format!(
                    "struct '{}' expects {} type argument{}, found {}",
                    name,
                    expected,
                    if expected == 1 { "" } else { "s" },
                    arguments.len()
                ),
            ));
        }

        Ok(arguments)
    }

    fn expect(
        &mut self,
        expected: TokenKind,
        message: &str,
        context: &Token,
    ) -> Result<(), TypeError> {
        match self.peek() {
            Some(token) if token.kind == expected => {
                self.position += 1;
                Ok(())
            }
            Some(token) => Err(TypeError::at(
                token,
                format!("{} (found '{}')", message, token.lexeme),
            )),
            None => Err(TypeError::at(
                context,
                format!("{} (found end of input)", message),
            )),
        }
    }

    fn peek(&self) -> Option<&'a Token> {
        self.tokens.get(self.position)
    }

    fn peek_kind(&self) -> Option<TokenKind> {
        self.peek().map(|token| token.kind.clone())
    }

    fn advance(&mut self) -> Option<&'a Token> {
        let token = self.peek();
        if token.is_some() {
            self.position += 1;
        }
        token
    }

    fn is_at_end(&self) -> bool {
        self.position >= self.tokens.len()
    }
}

// Helper functions for json/yaml type inference - kept for potential future use
#[allow(dead_code)]
fn json_value_to_type(value: &JsonValue) -> Type {
    match value {
        JsonValue::Null => Type::Nil,
        JsonValue::Bool(_) => Type::Bool,
        JsonValue::Number(number) => {
            if number.is_i64() || number.is_u64() {
                Type::Int
            } else {
                Type::Float
            }
        }
        JsonValue::String(_) => Type::String,
        JsonValue::Array(items) => {
            let mut element_type = Type::Unknown;
            for item in items {
                let item_type = json_value_to_type(item);
                element_type = merge_literal_types(element_type, item_type);
            }
            Type::List(Box::new(element_type))
        }
        JsonValue::Object(map) => {
            let mut value_type = Type::Unknown;
            for value in map.values() {
                let field_type = json_value_to_type(value);
                value_type = merge_literal_types(value_type, field_type);
            }
            Type::Dict(Box::new(value_type))
        }
    }
}

#[allow(dead_code)]
fn merge_literal_types(left: Type, right: Type) -> Type {
    match (left, right) {
        (Type::Unknown, other) => other,
        (other, Type::Unknown) => other,
        (Type::Nil, Type::Nil) => Type::Nil,
        (Type::Int, Type::Int) => Type::Int,
        (Type::Float, Type::Float) => Type::Float,
        (Type::Int, Type::Float) | (Type::Float, Type::Int) => Type::Float,
        (Type::Bool, Type::Bool) => Type::Bool,
        (Type::String, Type::String) => Type::String,
        (Type::List(left_el), Type::List(right_el)) => {
            Type::List(Box::new(merge_literal_types(*left_el, *right_el)))
        }
        (Type::Dict(left_val), Type::Dict(right_val)) => {
            Type::Dict(Box::new(merge_literal_types(*left_val, *right_val)))
        }
        (Type::Nil, other) => merge_literal_types(Type::Unknown, other),
        (other, Type::Nil) => merge_literal_types(other, Type::Unknown),
        (left, right) if left == right => left,
        _ => Type::Unknown,
    }
}
