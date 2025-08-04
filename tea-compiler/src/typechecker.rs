use std::collections::{HashMap, HashSet};

use crate::ast::{
    BinaryExpression, BinaryOperator, CallArgument, CallExpression, ConditionalStatement,
    DictLiteral, Expression, ExpressionKind, FunctionStatement, Identifier, IndexExpression,
    LambdaBody, LambdaExpression, ListLiteral, Literal, LoopHeader, LoopKind, LoopStatement,
    Module, ReturnStatement, SourceSpan, Statement, StructStatement, TestStatement, TypeExpression,
    TypeParameter, UnaryExpression, UnaryOperator, VarStatement,
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
    List(Box<Type>),
    Dict(Box<Type>),
    Function(Vec<Type>, Box<Type>),
    Struct(StructType),
    GenericParameter(String),
    Unknown,
}

impl Type {
    fn describe(&self) -> String {
        match self {
            Type::Bool => "Bool".to_string(),
            Type::Int => "Int".to_string(),
            Type::Float => "Float".to_string(),
            Type::String => "String".to_string(),
            Type::Nil => "Nil".to_string(),
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

#[cfg_attr(not(feature = "llvm-aot"), allow(dead_code))]
#[derive(Debug, Clone)]
pub struct FunctionInstance {
    pub type_arguments: Vec<Type>,
    pub param_types: Vec<Type>,
    pub return_type: Type,
}

#[cfg_attr(not(feature = "llvm-aot"), allow(dead_code))]
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
struct ModuleFunctionInfo {
    signature: FunctionSignature,
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

#[derive(Debug, Clone)]
struct FunctionContext {
    return_type: Type,
    saw_explicit_return: bool,
    last_expression_type: Option<Type>,
    explicit_return_types: Vec<Type>,
}

pub struct TypeChecker {
    scopes: Vec<HashMap<String, Type>>,
    const_scopes: Vec<HashSet<String>>,
    functions: HashMap<String, FunctionSignature>,
    structs: HashMap<String, StructDefinition>,
    type_parameters: Vec<HashSet<String>>,
    builtins: HashMap<String, StdFunctionKind>,
    module_aliases: HashMap<String, ModuleBinding>,
    contexts: Vec<FunctionContext>,
    diagnostics: Diagnostics,
    #[cfg(feature = "llvm-aot")]
    lambda_types: HashMap<usize, Type>,
    function_instances: HashMap<String, Vec<FunctionInstance>>,
    struct_instances: HashMap<String, Vec<StructInstance>>,
    function_call_metadata: HashMap<SourceSpan, (String, FunctionInstance)>,
    struct_call_metadata: HashMap<SourceSpan, (String, StructInstance)>,
}

impl TypeChecker {
    pub fn new() -> Self {
        let mut checker = Self {
            scopes: vec![HashMap::new()],
            const_scopes: vec![HashSet::new()],
            functions: HashMap::new(),
            structs: HashMap::new(),
            type_parameters: Vec::new(),
            builtins: HashMap::new(),
            module_aliases: HashMap::new(),
            contexts: Vec::new(),
            diagnostics: Diagnostics::new(),
            #[cfg(feature = "llvm-aot")]
            lambda_types: HashMap::new(),
            function_instances: HashMap::new(),
            struct_instances: HashMap::new(),
            function_call_metadata: HashMap::new(),
            struct_call_metadata: HashMap::new(),
        };
        checker.register_builtin_structs();
        checker
    }

    pub fn check_module(&mut self, module: &Module) {
        self.collect_structs(&module.statements);
        self.check_statements(&module.statements);
    }

    pub fn into_diagnostics(self) -> Diagnostics {
        self.diagnostics
    }

    #[cfg(feature = "llvm-aot")]
    pub(crate) fn lambda_types(&self) -> &HashMap<usize, Type> {
        &self.lambda_types
    }

    pub(crate) fn function_instances(&self) -> &HashMap<String, Vec<FunctionInstance>> {
        &self.function_instances
    }

    #[cfg_attr(not(feature = "llvm-aot"), allow(dead_code))]
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

    pub(crate) fn struct_definitions(&self) -> HashMap<String, StructDefinition> {
        self.structs.clone()
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
            Statement::Return(ret) => self.check_return(ret),
            Statement::Function(func) => self.check_function(func),
            Statement::Test(test_stmt) => self.check_test(test_stmt),
            Statement::Use(use_stmt) => self.register_use(use_stmt),
            Statement::Struct(_) => {}
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
            let inferred = binding
                .initializer
                .as_ref()
                .map(|expr| self.infer_expression(expr))
                .unwrap_or(Type::Unknown);

            let target_type = if let Some(expected) = annotated.clone() {
                self.ensure_compatible(
                    &expected,
                    &inferred,
                    &format!("variable '{}'", name),
                    Some(binding.span),
                );
                expected
            } else {
                inferred.clone()
            };

            self.insert(name.clone(), target_type, !statement.is_const);
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
        self.check_statements(&statement.consequent.statements);
        if let Some(alt) = &statement.alternative {
            self.check_statements(&alt.statements);
        }
    }

    fn check_loop(&mut self, statement: &LoopStatement) {
        match statement.kind {
            LoopKind::While | LoopKind::Until => {
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
            }
            LoopKind::For => {
                self.report_error(
                    "for loops are not supported by the type checker yet",
                    Some(statement.span),
                );
            }
        }
        self.check_statements(&statement.body.statements);
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
                    ctx.explicit_return_types.push(Type::Nil);
                }
            }
            (None, Type::Nil) => {
                if let Some(ctx) = self.contexts.last_mut() {
                    ctx.explicit_return_types.push(Type::Nil);
                }
            }
            (None, expected_type) => {
                self.report_error(
                    format!(
                        "return type mismatch: expected {}, found Nil",
                        expected_type.describe()
                    ),
                    Some(statement.span),
                );
                if let Some(ctx) = self.contexts.last_mut() {
                    ctx.explicit_return_types.push(Type::Nil);
                }
            }
        }
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
        self.assign_global(
            function.name.clone(),
            Type::Function(
                signature.params.clone(),
                Box::new(signature.return_type.clone()),
            ),
        );

        self.push_scope();
        self.contexts.push(FunctionContext {
            return_type: declared_return_type.clone(),
            saw_explicit_return: false,
            last_expression_type: None,
            explicit_return_types: Vec::new(),
        });

        for (param, expected_type) in function.parameters.iter().zip(param_types.iter()) {
            self.insert(param.name.clone(), expected_type.clone(), true);
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
                    if declared_return_type != Type::Unknown && declared_return_type != Type::Nil {
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
            (Type::List(expected_inner), Type::List(actual_inner)) => {
                self.ensure_compatible(expected_inner, actual_inner, context, span)
            }
            (Type::Dict(expected_inner), Type::Dict(actual_inner)) => {
                self.ensure_compatible(expected_inner, actual_inner, context, span)
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

    fn substitute_type(&self, ty: &Type, mapping: &HashMap<String, Type>) -> Type {
        match ty {
            Type::GenericParameter(name) => mapping
                .get(name)
                .cloned()
                .unwrap_or_else(|| Type::GenericParameter(name.clone())),
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
                self.report_error(
                    format!(
                        "{}: expected {}, found {}",
                        context,
                        left.describe(),
                        right.describe()
                    ),
                    span,
                );
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
            StdFunctionKind::CliCapture => {
                if let Some(first) = params.get_mut(0) {
                    *first = Type::List(Box::new(Type::String));
                }
            }
            StdFunctionKind::CliParse => {
                if let Some(first) = params.get_mut(0) {
                    *first = Type::Dict(Box::new(Type::Unknown));
                }
                if let Some(second) = params.get_mut(1) {
                    *second = Type::List(Box::new(Type::String));
                }
            }
            StdFunctionKind::ProcessRun => {
                if let Some(second) = params.get_mut(1) {
                    *second = Type::List(Box::new(Type::String));
                }
                if let Some(third) = params.get_mut(2) {
                    *third = Type::Dict(Box::new(Type::String));
                }
                if let Some(fourth) = params.get_mut(3) {
                    *fourth = Type::String;
                }
                if let Some(fifth) = params.get_mut(4) {
                    *fifth = Type::String;
                }
            }
            StdFunctionKind::ProcessSpawn => {
                if let Some(second) = params.get_mut(1) {
                    *second = Type::List(Box::new(Type::String));
                }
                if let Some(third) = params.get_mut(2) {
                    *third = Type::Dict(Box::new(Type::String));
                }
                if let Some(fourth) = params.get_mut(3) {
                    *fourth = Type::String;
                }
            }
            StdFunctionKind::ProcessReadStdout | StdFunctionKind::ProcessReadStderr => {
                if let Some(second) = params.get_mut(1) {
                    *second = Type::Int;
                }
            }
            _ => {}
        }

        let mut return_type = self.std_type_to_type(function.return_type);
        match function.kind {
            StdFunctionKind::PathComponents => {
                return_type = Type::List(Box::new(Type::String));
            }
            StdFunctionKind::CliCapture => {
                return_type = Type::Struct(StructType {
                    name: "CliResult".to_string(),
                    type_arguments: Vec::new(),
                });
            }
            StdFunctionKind::CliArgs => {
                return_type = Type::List(Box::new(Type::String));
            }
            StdFunctionKind::CliParse => {
                return_type = Type::Struct(StructType {
                    name: "CliParseResult".to_string(),
                    type_arguments: Vec::new(),
                });
            }
            StdFunctionKind::ProcessRun | StdFunctionKind::ProcessWait => {
                return_type = Type::Struct(StructType {
                    name: "ProcessResult".to_string(),
                    type_arguments: Vec::new(),
                });
            }
            StdFunctionKind::FsListDir | StdFunctionKind::FsWalk | StdFunctionKind::FsGlob => {
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
            self.report_error(
                format!("unknown module '{}'", module_path),
                Some(use_stmt.module_span),
            );
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
            StdType::Void => Type::Nil,
        }
    }

    fn parse_type(&mut self, type_expr: &TypeExpression) -> Option<Type> {
        if type_expr.tokens.is_empty() {
            self.report_error("missing type annotation after ':'", None);
            return None;
        }

        let type_parameters = self.current_type_parameters();
        let mut parser =
            TypeAnnotationParser::new(&type_expr.tokens, &type_parameters, &self.structs);
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
            ExpressionKind::Index(index) => self.type_from_index(index, expression.span),
            ExpressionKind::Range(_) => Type::Unknown,
        }
    }

    fn type_from_list(&mut self, list: &ListLiteral, _span: SourceSpan) -> Type {
        let mut element_type = Type::Unknown;
        for (index, element) in list.elements.iter().enumerate() {
            let value_type = self.infer_expression(element);
            element_type = self.merge_binding_type(
                element_type,
                value_type,
                &format!("list element {}", index + 1),
                Some(element.span),
            );
        }
        Type::List(Box::new(element_type))
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
                let int_type = Type::Int;
                self.ensure_compatible(&int_type, &index_type, "list index", Some(span));
                (*element_type).clone()
            }
            Type::Dict(value_type) => {
                let string_type = Type::String;
                self.ensure_compatible(&string_type, &index_type, "dict index", Some(span));
                (*value_type).clone()
            }
            Type::Unknown => Type::Unknown,
            other => {
                self.report_error(
                    format!(
                        "indexing requires a list or dict value, found {}",
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

    fn type_from_binary(&mut self, binary: &BinaryExpression, span: SourceSpan) -> Type {
        let left = self.infer_expression(&binary.left);
        let right = self.infer_expression(&binary.right);
        match binary.operator {
            BinaryOperator::Add
            | BinaryOperator::Subtract
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
            if let Some(struct_def) = self.structs.get(&identifier.name).cloned() {
                return self.type_from_struct_call(identifier, struct_def, call, span);
            }
        }

        if let Some(arg) = call.arguments.iter().find(|arg| arg.name.is_some()) {
            let span = arg.name_span.or(Some(arg.expression.span)).unwrap_or(span);
            self.report_error(
                "named arguments are only supported when constructing structs",
                Some(span),
            );
        }

        let arg_types: Vec<Type> = call
            .arguments
            .iter()
            .map(|arg| self.infer_expression(&arg.expression))
            .collect();
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
                        return match function.kind {
                            StdFunctionKind::JsonDecode => self.type_from_json_decode(
                                call,
                                span,
                                function.signature.return_type.clone(),
                            ),
                            StdFunctionKind::YamlDecode => self.type_from_yaml_decode(
                                call,
                                span,
                                function.signature.return_type.clone(),
                            ),
                            _ => function.signature.return_type.clone(),
                        };
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
                    self.verify_call_arguments(
                        &signature.params,
                        &arg_types,
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

                if let Some(kind) = self.builtins.get(&identifier.name).copied() {
                    match kind {
                        StdFunctionKind::JsonDecode => {
                            return self.type_from_json_decode(call, span, instantiated_return);
                        }
                        StdFunctionKind::YamlDecode => {
                            return self.type_from_yaml_decode(call, span, instantiated_return);
                        }
                        _ => {}
                    }
                }

                return instantiated_return;
            }

            if let Some(kind) = self.builtins.get(&identifier.name).copied() {
                return match kind {
                    StdFunctionKind::JsonDecode => {
                        self.type_from_json_decode(call, span, Type::Unknown)
                    }
                    StdFunctionKind::YamlDecode => {
                        self.type_from_yaml_decode(call, span, Type::Unknown)
                    }
                    _ => Type::Nil,
                };
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
                    saw_explicit_return: false,
                    last_expression_type: None,
                    explicit_return_types: Vec::new(),
                });
                self.check_statements(&block.statements);
                let context = self.contexts.pop().unwrap();

                if !context.explicit_return_types.is_empty() {
                    let mut iter = context.explicit_return_types.into_iter();
                    let mut acc = iter.next().unwrap_or(Type::Nil);
                    for ty in iter {
                        acc = self.merge_binding_type(acc, ty, "lambda return type", None);
                    }
                    acc
                } else if let Some(last) = context.last_expression_type {
                    last
                } else if context.saw_explicit_return {
                    Type::Nil
                } else {
                    Type::Nil
                }
            }
        };

        self.pop_scope();
        let function_type = Type::Function(param_types.clone(), Box::new(return_type.clone()));
        #[cfg(feature = "llvm-aot")]
        {
            self.lambda_types.insert(lambda.id, function_type.clone());
        }
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
                scope.insert(name);
            }
        }
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
            global.insert(name, ty);
        }
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
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
        self.const_scopes.pop();
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
    ) -> Self {
        Self {
            tokens,
            position: 0,
            type_parameters,
            structs,
        }
    }

    fn parse_type(&mut self) -> Result<Type, TypeError> {
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
                "List" => self.parse_list_type(),
                "Dict" => self.parse_dict_type(),
                "Func" | "Function" | "Fn" => self.parse_function_type(),
                other => {
                    let ident_token = token.clone();
                    self.advance();
                    if self.type_parameters.contains(other) {
                        Ok(Type::GenericParameter(other.to_string()))
                    } else {
                        let type_arguments =
                            self.parse_struct_type_arguments(other, &ident_token)?;
                        Ok(Type::Struct(StructType {
                            name: other.to_string(),
                            type_arguments,
                        }))
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
