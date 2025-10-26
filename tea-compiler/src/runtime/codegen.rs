use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use anyhow::{anyhow, bail, Result};

use crate::ast::{
    BinaryExpression, BinaryOperator, Block, ConditionalKind, ConditionalStatement, DictLiteral,
    Expression, ExpressionKind, FunctionParameter, FunctionStatement, IndexExpression,
    InterpolatedStringExpression, InterpolatedStringPart, LambdaBody, LambdaExpression,
    ListLiteral, Literal, LoopHeader, LoopKind, LoopStatement, MatchExpression, MatchPattern,
    MatchStatement, MemberExpression, Module, ReturnStatement, SourceSpan, Statement,
    TestStatement, UnaryExpression, UnaryOperator, UseStatement, VarBinding, VarStatement,
};

use super::bytecode::{Chunk, Function, Instruction, Program, TestCase};
use super::value::{EnumVariantValue, StructTemplate, Value};
use crate::stdlib::{self, StdFunctionKind};
use crate::typechecker::{
    EnumDefinition, EnumVariantMetadata, FunctionInstance, StructDefinition, StructInstance,
    StructType, Type,
};

fn format_type_name(ty: &Type) -> String {
    match ty {
        Type::Bool => "Bool".to_string(),
        Type::Int => "Int".to_string(),
        Type::Float => "Float".to_string(),
        Type::String => "String".to_string(),
        Type::Nil => "Nil".to_string(),
        Type::Optional(inner) => format!("{}?", format_type_name(inner)),
        Type::List(inner) => format!("List[{}]", format_type_name(inner)),
        Type::Dict(inner) => format!("Dict[String, {}]", format_type_name(inner)),
        Type::Function(params, return_type) => {
            let param_str = if params.is_empty() {
                "()".to_string()
            } else {
                let joined = params
                    .iter()
                    .map(|param| format_type_name(param))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("({joined})")
            };
            format!("Func{param_str} -> {}", format_type_name(return_type))
        }
        Type::Struct(struct_type) => format_struct_type_name(struct_type),
        Type::Enum(enum_type) => enum_type.name.clone(),
        Type::GenericParameter(name) => name.clone(),
        Type::Unknown => "Unknown".to_string(),
    }
}

fn format_struct_type_name(struct_type: &StructType) -> String {
    if struct_type.type_arguments.is_empty() {
        struct_type.name.clone()
    } else {
        let args = struct_type
            .type_arguments
            .iter()
            .map(|arg| format_type_name(arg))
            .collect::<Vec<_>>()
            .join(",");
        format!("{}[{}]", struct_type.name, args)
    }
}

fn sanitize_symbol_component(component: &str) -> String {
    component
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

fn mangle_function_name(name: &str, type_arguments: &[Type]) -> String {
    if type_arguments.is_empty() {
        return name.to_string();
    }
    let components: Vec<String> = type_arguments
        .iter()
        .map(|ty| sanitize_symbol_component(&format_type_name(ty)))
        .collect();
    format!("{}$g{}", name, components.join("$"))
}

pub struct VmSemanticMetadata {
    pub function_instances: HashMap<String, Vec<FunctionInstance>>,
    pub function_call_metadata: HashMap<SourceSpan, (String, FunctionInstance)>,
    pub struct_call_metadata: HashMap<SourceSpan, (String, StructInstance)>,
    pub struct_definitions: HashMap<String, StructDefinition>,
    pub enum_variant_metadata: HashMap<SourceSpan, EnumVariantMetadata>,
    pub enum_definitions: HashMap<String, EnumDefinition>,
}

#[derive(Debug)]
pub enum CodegenError {
    Unsupported(&'static str),
}

impl std::fmt::Display for CodegenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CodegenError::Unsupported(feature) => write!(f, "{feature} is not yet supported"),
        }
    }
}

impl std::error::Error for CodegenError {}

pub struct CodeGenerator {
    globals: HashMap<String, usize>,
    const_globals: HashSet<String>,
    functions: Vec<Function>,
    tests: Vec<TestCase>,
    builtins: HashMap<String, StdFunctionKind>,
    module_builtins: HashMap<String, HashMap<String, StdFunctionKind>>,
    struct_indices: HashMap<String, usize>,
    structs: Vec<StructTemplate>,
    lambda_captures: HashMap<usize, Vec<String>>,
    function_instances: HashMap<String, Vec<FunctionInstance>>,
    function_call_metadata: HashMap<SourceSpan, (String, FunctionInstance)>,
    struct_call_metadata: HashMap<SourceSpan, (String, StructInstance)>,
    struct_definitions: HashMap<String, StructDefinition>,
    functions_by_name: HashMap<String, FunctionStatement>,
    base_function_indices: HashMap<String, usize>,
    function_specializations: HashMap<String, HashMap<Vec<Type>, usize>>,
    generic_binding_stack: Vec<HashMap<String, Type>>,
    enum_variant_metadata: HashMap<SourceSpan, EnumVariantMetadata>,
    enum_definitions: HashMap<String, EnumDefinition>,
    temp_name_counter: usize,
    next_global_temp_slot: usize,
}

impl CodeGenerator {
    pub fn new(lambda_captures: HashMap<usize, Vec<String>>, metadata: VmSemanticMetadata) -> Self {
        Self {
            globals: HashMap::new(),
            const_globals: HashSet::new(),
            functions: Vec::new(),
            tests: Vec::new(),
            builtins: HashMap::new(),
            module_builtins: HashMap::new(),
            struct_indices: HashMap::new(),
            structs: Vec::new(),
            lambda_captures,
            function_instances: metadata.function_instances,
            function_call_metadata: metadata.function_call_metadata,
            struct_call_metadata: metadata.struct_call_metadata,
            struct_definitions: metadata.struct_definitions,
            functions_by_name: HashMap::new(),
            base_function_indices: HashMap::new(),
            function_specializations: HashMap::new(),
            generic_binding_stack: Vec::new(),
            enum_variant_metadata: metadata.enum_variant_metadata,
            enum_definitions: metadata.enum_definitions,
            temp_name_counter: 0,
            next_global_temp_slot: 0,
        }
    }

    pub fn compile_module(mut self, module: &Module) -> Result<Program> {
        self.collect_functions(&module.statements);
        self.collect_structs(&module.statements);
        let mut resolver = GlobalResolver;
        let mut chunk = Chunk::new();
        self.compile_statements(&module.statements, &mut chunk, &mut resolver)?;

        if !matches!(chunk.instructions.last(), Some(Instruction::Return)) {
            let nil_index = chunk.add_constant(Value::Nil);
            chunk.emit(Instruction::Constant(nil_index));
            chunk.emit(Instruction::Return);
        }

        let globals = self.order_globals();
        Ok(Program::new(
            chunk,
            self.functions,
            globals,
            self.structs,
            self.tests,
        ))
    }

    fn collect_structs(&mut self, statements: &[Statement]) {
        for statement in statements {
            if let Statement::Struct(struct_stmt) = statement {
                let struct_type = StructType {
                    name: struct_stmt.name.clone(),
                    type_arguments: Vec::new(),
                };
                let _ = self.ensure_struct_template(&struct_type);
            }
        }
    }

    fn collect_functions(&mut self, statements: &[Statement]) {
        for statement in statements {
            if let Statement::Function(function) = statement {
                self.functions_by_name
                    .insert(function.name.clone(), function.clone());
            }
        }
    }

    fn ensure_struct_template_from_instance(
        &mut self,
        name: &str,
        instance: &StructInstance,
    ) -> Result<usize> {
        let mut resolved_args = Vec::with_capacity(instance.type_arguments.len());
        for arg in &instance.type_arguments {
            resolved_args.push(self.resolve_type_with_bindings(arg)?);
        }
        let struct_type = StructType {
            name: name.to_string(),
            type_arguments: resolved_args,
        };
        self.ensure_struct_template(&struct_type)
    }

    fn ensure_struct_template(&mut self, struct_type: &StructType) -> Result<usize> {
        let variant_name = format_struct_type_name(struct_type);
        if let Some(&index) = self.struct_indices.get(&variant_name) {
            return Ok(index);
        }

        let definition = self
            .struct_definitions
            .get(&struct_type.name)
            .ok_or_else(|| anyhow!(format!("unknown struct '{}'", struct_type.name)))?;

        let field_names = definition
            .fields
            .iter()
            .map(|field| field.name.clone())
            .collect::<Vec<_>>();

        let index = self.structs.len();
        self.structs.push(StructTemplate {
            name: variant_name.clone(),
            field_names,
        });
        self.struct_indices.insert(variant_name.clone(), index);
        Ok(index)
    }

    fn resolve_type_with_bindings(&self, ty: &Type) -> Result<Type> {
        match ty {
            Type::GenericParameter(name) => {
                for scope in self.generic_binding_stack.iter().rev() {
                    if let Some(bound) = scope.get(name) {
                        return Ok(bound.clone());
                    }
                }
                Ok(ty.clone())
            }
            Type::List(inner) => Ok(Type::List(Box::new(
                self.resolve_type_with_bindings(inner)?,
            ))),
            Type::Dict(inner) => Ok(Type::Dict(Box::new(
                self.resolve_type_with_bindings(inner)?,
            ))),
            Type::Function(params, return_type) => {
                let mut resolved_params = Vec::with_capacity(params.len());
                for param in params {
                    resolved_params.push(self.resolve_type_with_bindings(param)?);
                }
                let resolved_return = self.resolve_type_with_bindings(return_type)?;
                Ok(Type::Function(resolved_params, Box::new(resolved_return)))
            }
            Type::Struct(struct_type) => {
                let mut resolved_args = Vec::with_capacity(struct_type.type_arguments.len());
                for arg in &struct_type.type_arguments {
                    resolved_args.push(self.resolve_type_with_bindings(arg)?);
                }
                Ok(Type::Struct(StructType {
                    name: struct_type.name.clone(),
                    type_arguments: resolved_args,
                }))
            }
            other => Ok(other.clone()),
        }
    }

    fn ensure_base_function(&mut self, function: &FunctionStatement) -> Result<usize> {
        if let Some(&index) = self.base_function_indices.get(&function.name) {
            return Ok(index);
        }
        let index = self.functions.len();
        self.base_function_indices
            .insert(function.name.clone(), index);
        let _ = self.resolve_or_insert_global(&function.name);
        self.functions.push(Function {
            name: function.name.clone(),
            arity: function.parameters.len(),
            chunk: Chunk::new(),
        });
        let chunk = match self.compile_function_chunk(function) {
            Ok(chunk) => chunk,
            Err(err) => {
                self.base_function_indices.remove(&function.name);
                self.functions.pop();
                return Err(err);
            }
        };
        self.functions[index] = Function {
            name: function.name.clone(),
            arity: function.parameters.len(),
            chunk,
        };
        Ok(index)
    }

    fn compile_function_chunk(&mut self, function: &FunctionStatement) -> Result<Chunk> {
        let mut function_chunk = Chunk::new();
        let mut resolver = FunctionResolver::new(&function.parameters);
        let returns_value =
            self.compile_block(&function.body, &mut function_chunk, &mut resolver)?;

        if !matches!(
            function_chunk.instructions.last(),
            Some(Instruction::Return)
        ) {
            if !returns_value {
                let nil_index = function_chunk.add_constant(Value::Nil);
                function_chunk.emit(Instruction::Constant(nil_index));
            }
            function_chunk.emit(Instruction::Return);
        }

        Ok(function_chunk)
    }

    fn ensure_function_specialization(
        &mut self,
        name: &str,
        instance: &FunctionInstance,
    ) -> Result<usize> {
        if instance.type_arguments.is_empty() {
            let function_stmt = self
                .functions_by_name
                .get(name)
                .ok_or_else(|| anyhow!(format!("unknown function '{}'", name)))?
                .clone();
            return self.ensure_base_function(&function_stmt);
        }

        let mut resolved_args = Vec::with_capacity(instance.type_arguments.len());
        for arg in &instance.type_arguments {
            resolved_args.push(self.resolve_type_with_bindings(arg)?);
        }

        if let Some(index) = self
            .function_specializations
            .get(name)
            .and_then(|map| map.get(&resolved_args))
        {
            return Ok(*index);
        }

        let function_stmt = self
            .functions_by_name
            .get(name)
            .ok_or_else(|| anyhow!(format!("unknown function '{}'", name)))?
            .clone();

        if function_stmt.type_parameters.len() != resolved_args.len() {
            bail!(format!(
                "function '{}' expects {} type arguments, found {}",
                name,
                function_stmt.type_parameters.len(),
                resolved_args.len()
            ));
        }

        let mut bindings = HashMap::new();
        for (param, ty) in function_stmt
            .type_parameters
            .iter()
            .map(|param| param.name.clone())
            .zip(resolved_args.iter().cloned())
        {
            bindings.insert(param, ty);
        }

        self.generic_binding_stack.push(bindings);
        let chunk_result = self.compile_function_chunk(&function_stmt);
        let chunk = match chunk_result {
            Ok(chunk) => {
                self.generic_binding_stack.pop();
                chunk
            }
            Err(err) => {
                self.generic_binding_stack.pop();
                return Err(err);
            }
        };

        let mangled = mangle_function_name(name, &resolved_args);
        let index = self.functions.len();
        self.functions.push(Function {
            name: mangled,
            arity: function_stmt.parameters.len(),
            chunk,
        });
        self.function_specializations
            .entry(name.to_string())
            .or_insert_with(HashMap::new)
            .insert(resolved_args, index);
        Ok(index)
    }

    fn order_globals(&self) -> Vec<String> {
        let mut globals: Vec<(usize, String)> = self
            .globals
            .iter()
            .map(|(name, index)| (*index, name.clone()))
            .collect();
        globals.sort_by_key(|(index, _)| *index);
        globals.into_iter().map(|(_, name)| name).collect()
    }

    fn compile_statements<R: Resolver>(
        &mut self,
        statements: &[Statement],
        chunk: &mut Chunk,
        resolver: &mut R,
    ) -> Result<()> {
        for statement in statements {
            self.compile_statement(statement, chunk, resolver)?;
        }
        Ok(())
    }

    fn compile_statement<R: Resolver>(
        &mut self,
        statement: &Statement,
        chunk: &mut Chunk,
        resolver: &mut R,
    ) -> Result<()> {
        match statement {
            Statement::Var(var) => self.compile_var(var, chunk, resolver),
            Statement::Expression(expr) => {
                self.compile_expression(&expr.expression, chunk, resolver)?;
                chunk.emit(Instruction::Pop);
                Ok(())
            }
            Statement::Conditional(stmt) => self.compile_conditional(stmt, chunk, resolver),
            Statement::Return(stmt) => self.compile_return(stmt, chunk, resolver),
            Statement::Function(func) => self.compile_function(func, chunk),
            Statement::Use(use_stmt) => self.compile_use(use_stmt, resolver),
            Statement::Loop(loop_stmt) => self.compile_loop(loop_stmt, chunk, resolver),
            Statement::Struct(_) => Ok(()),
            Statement::Enum(_) => Ok(()),
            Statement::Test(test_stmt) => self.compile_test(test_stmt),
            Statement::Match(match_stmt) => {
                self.compile_match_statement(match_stmt, chunk, resolver)
            }
        }
    }

    fn compile_var<R: Resolver>(
        &mut self,
        statement: &VarStatement,
        chunk: &mut Chunk,
        resolver: &mut R,
    ) -> Result<()> {
        match resolver.kind() {
            ResolverKind::Global => {
                for VarBinding {
                    name,
                    type_annotation: _,
                    initializer,
                    ..
                } in &statement.bindings
                {
                    if statement.is_const && initializer.is_none() {
                        bail!(format!("const '{}' requires an initializer", name));
                    }
                    if let Some(expr) = initializer {
                        self.compile_expression(expr, chunk, resolver)?;
                    } else {
                        let constant = chunk.add_constant(Value::Nil);
                        chunk.emit(Instruction::Constant(constant));
                    }
                    if statement.is_const {
                        self.store_const_global(chunk, name)?;
                    } else {
                        resolver.store(self, chunk, name)?;
                    }
                    chunk.emit(Instruction::Pop);
                }
                Ok(())
            }
            ResolverKind::Function => {
                for VarBinding {
                    name,
                    type_annotation: _,
                    initializer,
                    ..
                } in &statement.bindings
                {
                    if statement.is_const && initializer.is_none() {
                        bail!(format!("const '{}' requires an initializer", name));
                    }
                    let slot = resolver.declare_local(name, statement.is_const)?;
                    if let Some(expr) = initializer {
                        self.compile_expression(expr, chunk, resolver)?;
                    } else {
                        let constant = chunk.add_constant(Value::Nil);
                        chunk.emit(Instruction::Constant(constant));
                    }
                    chunk.emit(Instruction::SetLocal(slot));
                    chunk.emit(Instruction::Pop);
                }
                Ok(())
            }
        }
    }

    fn compile_return<R: Resolver>(
        &mut self,
        statement: &ReturnStatement,
        chunk: &mut Chunk,
        resolver: &mut R,
    ) -> Result<()> {
        if let Some(expr) = &statement.expression {
            self.compile_expression(expr, chunk, resolver)?;
        } else {
            let constant = chunk.add_constant(Value::Nil);
            chunk.emit(Instruction::Constant(constant));
        }
        chunk.emit(Instruction::Return);
        Ok(())
    }

    fn compile_conditional<R: Resolver>(
        &mut self,
        statement: &ConditionalStatement,
        chunk: &mut Chunk,
        resolver: &mut R,
    ) -> Result<()> {
        self.compile_expression(&statement.condition, chunk, resolver)?;
        if matches!(statement.kind, ConditionalKind::Unless) {
            chunk.emit(Instruction::Not);
        }
        let then_jump = self.emit_jump(chunk, Instruction::JumpIfFalse(usize::MAX));
        let _ = self.compile_block(&statement.consequent, chunk, resolver)?;

        let else_jump = if statement.alternative.is_some() {
            Some(self.emit_jump(chunk, Instruction::Jump(usize::MAX)))
        } else {
            None
        };

        self.patch_jump(chunk, then_jump)?;

        if let Some(alternative) = &statement.alternative {
            let _ = self.compile_block(alternative, chunk, resolver)?;
            if let Some(jump) = else_jump {
                self.patch_jump(chunk, jump)?;
            }
        }

        Ok(())
    }

    fn compile_block<R: Resolver>(
        &mut self,
        block: &Block,
        chunk: &mut Chunk,
        resolver: &mut R,
    ) -> Result<bool> {
        let len = block.statements.len();
        let mut returns_value = false;
        for (index, statement) in block.statements.iter().enumerate() {
            let is_last = index + 1 == len;
            if is_last && resolver.kind() == ResolverKind::Function {
                if let Statement::Expression(expr) = statement {
                    self.compile_expression(&expr.expression, chunk, resolver)?;
                    returns_value = true;
                    continue;
                }
            }
            self.compile_statement(statement, chunk, resolver)?;
            returns_value = false;
        }
        Ok(returns_value)
    }

    fn compile_expression<R: Resolver>(
        &mut self,
        expression: &Expression,
        chunk: &mut Chunk,
        resolver: &mut R,
    ) -> Result<()> {
        match &expression.kind {
            ExpressionKind::Literal(literal) => self.compile_literal(literal, chunk),
            ExpressionKind::InterpolatedString(template) => {
                self.compile_interpolated_string(template, chunk, resolver)
            }
            ExpressionKind::Identifier(identifier) => resolver.load(self, chunk, &identifier.name),
            ExpressionKind::Binary(binary) => self.compile_binary(binary, chunk, resolver),
            ExpressionKind::Unary(unary) => self.compile_unary(unary, chunk, resolver),
            ExpressionKind::Assignment(assignment) => {
                if let ExpressionKind::Identifier(identifier) = &assignment.target.kind {
                    self.compile_expression(&assignment.value, chunk, resolver)?;
                    resolver.store(self, chunk, &identifier.name)
                } else {
                    Err(CodegenError::Unsupported("complex assignment targets").into())
                }
            }
            ExpressionKind::Call(call) => self.compile_call(call, expression.span, chunk, resolver),
            ExpressionKind::Grouping(expr) => self.compile_expression(expr, chunk, resolver),
            ExpressionKind::List(list) => self.compile_list_literal(list, chunk, resolver),
            ExpressionKind::Dict(dict) => self.compile_dict_literal(dict, chunk, resolver),
            ExpressionKind::Index(index) => self.compile_index_expression(index, chunk, resolver),
            ExpressionKind::Member(member) => {
                if let Some(metadata) = self.enum_variant_metadata.get(&expression.span).cloned() {
                    self.emit_enum_variant(metadata, chunk)
                } else if let Some(metadata) =
                    self.enum_variant_metadata_from_ast(member, expression.span)
                {
                    self.emit_enum_variant(metadata, chunk)
                } else {
                    self.compile_member_expression(member, chunk, resolver)
                }
            }
            ExpressionKind::Lambda(lambda) => {
                self.compile_lambda_expression(lambda, chunk, resolver)
            }
            ExpressionKind::Match(match_expr) => {
                self.compile_match_expression(match_expr, chunk, resolver)
            }
            ExpressionKind::Unwrap(inner) => {
                self.compile_expression(inner, chunk, resolver)?;
                chunk.emit(Instruction::AssertNonNil);
                Ok(())
            }
            ExpressionKind::Range(_) => Err(CodegenError::Unsupported("expression").into()),
        }
    }

    fn compile_literal(&mut self, literal: &Literal, chunk: &mut Chunk) -> Result<()> {
        let value = match literal {
            Literal::Integer(value) => Value::Int(*value),
            Literal::Float(value) => Value::Float(*value),
            Literal::String(value) => Value::String(value.clone()),
            Literal::Boolean(value) => Value::Bool(*value),
            Literal::Nil => Value::Nil,
        };

        let index = chunk.add_constant(value);
        chunk.emit(Instruction::Constant(index));
        Ok(())
    }

    fn compile_interpolated_string<R: Resolver>(
        &mut self,
        template: &InterpolatedStringExpression,
        chunk: &mut Chunk,
        resolver: &mut R,
    ) -> Result<()> {
        if template.parts.is_empty() {
            let constant = chunk.add_constant(Value::String(String::new()));
            chunk.emit(Instruction::Constant(constant));
            return Ok(());
        }

        let mut part_count = 0;
        for part in &template.parts {
            match part {
                InterpolatedStringPart::Literal(text) => {
                    let constant = chunk.add_constant(Value::String(text.clone()));
                    chunk.emit(Instruction::Constant(constant));
                }
                InterpolatedStringPart::Expression(expr) => {
                    self.compile_expression(expr, chunk, resolver)?;
                    chunk.emit(Instruction::BuiltinCall {
                        kind: StdFunctionKind::UtilToString,
                        arg_count: 1,
                    });
                }
            }
            part_count += 1;
        }

        chunk.emit(Instruction::ConcatStrings(part_count));
        Ok(())
    }

    fn compile_binary<R: Resolver>(
        &mut self,
        expression: &BinaryExpression,
        chunk: &mut Chunk,
        resolver: &mut R,
    ) -> Result<()> {
        match expression.operator {
            BinaryOperator::Coalesce => {
                self.compile_expression(&expression.left, chunk, resolver)?;
                let evaluate_right = self.emit_jump(chunk, Instruction::JumpIfNil(usize::MAX));
                let end = self.emit_jump(chunk, Instruction::Jump(usize::MAX));
                self.patch_jump(chunk, evaluate_right)?;
                chunk.emit(Instruction::Pop);
                self.compile_expression(&expression.right, chunk, resolver)?;
                self.patch_jump(chunk, end)?;
                Ok(())
            }
            BinaryOperator::And => {
                self.compile_expression(&expression.left, chunk, resolver)?;
                let short_circuit = self.emit_jump(chunk, Instruction::JumpIfFalse(usize::MAX));
                self.compile_expression(&expression.right, chunk, resolver)?;
                let end = self.emit_jump(chunk, Instruction::Jump(usize::MAX));
                self.patch_jump(chunk, short_circuit)?;
                let false_constant = chunk.add_constant(Value::Bool(false));
                chunk.emit(Instruction::Constant(false_constant));
                self.patch_jump(chunk, end)?;
                Ok(())
            }
            BinaryOperator::Or => {
                self.compile_expression(&expression.left, chunk, resolver)?;
                let evaluate_right = self.emit_jump(chunk, Instruction::JumpIfFalse(usize::MAX));
                let true_constant = chunk.add_constant(Value::Bool(true));
                chunk.emit(Instruction::Constant(true_constant));
                let end = self.emit_jump(chunk, Instruction::Jump(usize::MAX));
                self.patch_jump(chunk, evaluate_right)?;
                self.compile_expression(&expression.right, chunk, resolver)?;
                self.patch_jump(chunk, end)?;
                Ok(())
            }
            BinaryOperator::Add => {
                self.compile_expression(&expression.left, chunk, resolver)?;
                self.compile_expression(&expression.right, chunk, resolver)?;
                chunk.emit(Instruction::Add);
                Ok(())
            }
            BinaryOperator::Subtract => {
                self.compile_expression(&expression.left, chunk, resolver)?;
                self.compile_expression(&expression.right, chunk, resolver)?;
                chunk.emit(Instruction::Subtract);
                Ok(())
            }
            BinaryOperator::Multiply => {
                self.compile_expression(&expression.left, chunk, resolver)?;
                self.compile_expression(&expression.right, chunk, resolver)?;
                chunk.emit(Instruction::Multiply);
                Ok(())
            }
            BinaryOperator::Divide => {
                self.compile_expression(&expression.left, chunk, resolver)?;
                self.compile_expression(&expression.right, chunk, resolver)?;
                chunk.emit(Instruction::Divide);
                Ok(())
            }
            BinaryOperator::Modulo => {
                self.compile_expression(&expression.left, chunk, resolver)?;
                self.compile_expression(&expression.right, chunk, resolver)?;
                chunk.emit(Instruction::Modulo);
                Ok(())
            }
            BinaryOperator::Equal => {
                self.compile_expression(&expression.left, chunk, resolver)?;
                self.compile_expression(&expression.right, chunk, resolver)?;
                chunk.emit(Instruction::Equal);
                Ok(())
            }
            BinaryOperator::NotEqual => {
                self.compile_expression(&expression.left, chunk, resolver)?;
                self.compile_expression(&expression.right, chunk, resolver)?;
                chunk.emit(Instruction::NotEqual);
                Ok(())
            }
            BinaryOperator::Greater => {
                self.compile_expression(&expression.left, chunk, resolver)?;
                self.compile_expression(&expression.right, chunk, resolver)?;
                chunk.emit(Instruction::Greater);
                Ok(())
            }
            BinaryOperator::GreaterEqual => {
                self.compile_expression(&expression.left, chunk, resolver)?;
                self.compile_expression(&expression.right, chunk, resolver)?;
                chunk.emit(Instruction::GreaterEqual);
                Ok(())
            }
            BinaryOperator::Less => {
                self.compile_expression(&expression.left, chunk, resolver)?;
                self.compile_expression(&expression.right, chunk, resolver)?;
                chunk.emit(Instruction::Less);
                Ok(())
            }
            BinaryOperator::LessEqual => {
                self.compile_expression(&expression.left, chunk, resolver)?;
                self.compile_expression(&expression.right, chunk, resolver)?;
                chunk.emit(Instruction::LessEqual);
                Ok(())
            }
        }
    }

    fn compile_unary<R: Resolver>(
        &mut self,
        expression: &UnaryExpression,
        chunk: &mut Chunk,
        resolver: &mut R,
    ) -> Result<()> {
        self.compile_expression(&expression.operand, chunk, resolver)?;
        let instruction = match expression.operator {
            UnaryOperator::Positive => return Ok(()),
            UnaryOperator::Negative => Instruction::Negate,
            UnaryOperator::Not => Instruction::Not,
        };
        chunk.emit(instruction);
        Ok(())
    }

    fn emit_builtin_call<R: Resolver>(
        &mut self,
        kind: StdFunctionKind,
        call: &crate::ast::CallExpression,
        chunk: &mut Chunk,
        resolver: &mut R,
    ) -> Result<()> {
        match kind {
            StdFunctionKind::Print => {
                for argument in &call.arguments {
                    if argument.name.is_some() {
                        bail!("named arguments are not supported for builtins");
                    }
                    self.compile_expression(&argument.expression, chunk, resolver)?;
                }
                chunk.emit(Instruction::Print);
                Ok(())
            }
            _ => {
                for argument in &call.arguments {
                    if argument.name.is_some() {
                        bail!("named arguments are not supported for builtins");
                    }
                    self.compile_expression(&argument.expression, chunk, resolver)?;
                }
                chunk.emit(Instruction::BuiltinCall {
                    kind,
                    arg_count: call.arguments.len(),
                });
                Ok(())
            }
        }
    }

    fn compile_call<R: Resolver>(
        &mut self,
        call: &crate::ast::CallExpression,
        span: SourceSpan,
        chunk: &mut Chunk,
        resolver: &mut R,
    ) -> Result<()> {
        if let ExpressionKind::Member(member) = &call.callee.kind {
            if let ExpressionKind::Identifier(alias_ident) = &member.object.kind {
                if let Some(functions) = self.module_builtins.get(&alias_ident.name) {
                    if let Some(&kind) = functions.get(&member.property) {
                        return self.emit_builtin_call(kind, call, chunk, resolver);
                    }
                }
            }
        }

        if let ExpressionKind::Identifier(identifier) = &call.callee.kind {
            if let Some(kind) = self.builtins.get(identifier.name.as_str()).copied() {
                self.emit_builtin_call(kind, call, chunk, resolver)?;
                return Ok(());
            }
        }

        if let Some((struct_name, instance)) = self.struct_call_metadata.get(&span).cloned() {
            let struct_index =
                self.ensure_struct_template_from_instance(&struct_name, &instance)?;
            return self.compile_struct_constructor_with_index(
                struct_index,
                &struct_name,
                call,
                chunk,
                resolver,
            );
        }

        if let ExpressionKind::Identifier(identifier) = &call.callee.kind {
            if let Some(&struct_index) = self.struct_indices.get(&identifier.name) {
                return self.compile_struct_constructor_with_index(
                    struct_index,
                    &identifier.name,
                    call,
                    chunk,
                    resolver,
                );
            }
        }

        if let Some((function_name, instance)) = self.function_call_metadata.get(&span).cloned() {
            let function_index = self.ensure_function_specialization(&function_name, &instance)?;
            let constant_index = chunk.add_constant(Value::Function(function_index));
            chunk.emit(Instruction::Constant(constant_index));
            for argument in &call.arguments {
                self.compile_expression(&argument.expression, chunk, resolver)?;
            }
            chunk.emit(Instruction::Call(call.arguments.len()));
            return Ok(());
        }

        self.compile_expression(&call.callee, chunk, resolver)?;
        for argument in &call.arguments {
            self.compile_expression(&argument.expression, chunk, resolver)?;
        }
        chunk.emit(Instruction::Call(call.arguments.len()));
        Ok(())
    }

    fn compile_struct_constructor_with_index<R: Resolver>(
        &mut self,
        struct_index: usize,
        struct_name: &str,
        call: &crate::ast::CallExpression,
        chunk: &mut Chunk,
        resolver: &mut R,
    ) -> Result<()> {
        let all_named = call.arguments.iter().all(|arg| arg.name.is_some());
        let any_named = call.arguments.iter().any(|arg| arg.name.is_some());
        if all_named {
            for argument in &call.arguments {
                let field_name = argument
                    .name
                    .as_ref()
                    .expect("named argument expected for struct constructor");
                let constant = chunk.add_constant(Value::String(field_name.clone()));
                chunk.emit(Instruction::Constant(constant));
                self.compile_expression(&argument.expression, chunk, resolver)?;
            }
            chunk.emit(Instruction::MakeStructNamed(struct_index));
            Ok(())
        } else if any_named {
            bail!(format!(
                "cannot mix named and positional arguments when constructing struct '{}'",
                struct_name
            ));
        } else {
            for argument in &call.arguments {
                self.compile_expression(&argument.expression, chunk, resolver)?;
            }
            chunk.emit(Instruction::MakeStructPositional(struct_index));
            Ok(())
        }
    }

    fn emit_jump(&mut self, chunk: &mut Chunk, instruction: Instruction) -> usize {
        chunk.emit(instruction)
    }

    fn patch_jump(&mut self, chunk: &mut Chunk, index: usize) -> Result<()> {
        let target = chunk.len();
        match chunk.instructions.get_mut(index) {
            Some(Instruction::Jump(slot))
            | Some(Instruction::JumpIfFalse(slot))
            | Some(Instruction::JumpIfNil(slot)) => {
                *slot = target;
                Ok(())
            }
            _ => bail!("invalid jump patch location"),
        }
    }

    fn allocate_temp_slot<R: Resolver>(&mut self, resolver: &mut R) -> Result<TempSlot> {
        match resolver.kind() {
            ResolverKind::Function => {
                let name = format!("__match_tmp{}", self.temp_name_counter);
                self.temp_name_counter += 1;
                let _ = resolver.declare_local(&name, false)?;
                Ok(TempSlot::ResolverLocal { name })
            }
            ResolverKind::Global => {
                let index = self.next_global_temp_slot;
                self.next_global_temp_slot += 1;
                Ok(TempSlot::DirectLocal { index })
            }
        }
    }

    fn store_temp_slot<R: Resolver>(
        &mut self,
        slot: &TempSlot,
        chunk: &mut Chunk,
        resolver: &mut R,
    ) -> Result<()> {
        match slot {
            TempSlot::ResolverLocal { name } => resolver.store(self, chunk, name),
            TempSlot::DirectLocal { index } => {
                chunk.emit(Instruction::SetLocal(*index));
                Ok(())
            }
        }
    }

    fn load_temp_slot<R: Resolver>(
        &mut self,
        slot: &TempSlot,
        chunk: &mut Chunk,
        resolver: &mut R,
    ) -> Result<()> {
        match slot {
            TempSlot::ResolverLocal { name } => resolver.load(self, chunk, name),
            TempSlot::DirectLocal { index } => {
                chunk.emit(Instruction::GetLocal(*index));
                Ok(())
            }
        }
    }

    fn resolve_or_insert_global(&mut self, name: &str) -> usize {
        if let Some(index) = self.globals.get(name) {
            *index
        } else {
            let index = self.globals.len();
            self.globals.insert(name.to_string(), index);
            index
        }
    }

    fn load_global(&self, chunk: &mut Chunk, name: &str) -> Result<()> {
        if let Some(index) = self.globals.get(name).cloned() {
            chunk.emit(Instruction::GetGlobal(index));
            Ok(())
        } else {
            bail!(format!("undefined global '{}'", name))
        }
    }

    fn store_global(&mut self, chunk: &mut Chunk, name: &str) -> Result<()> {
        if self.const_globals.contains(name) {
            bail!(format!("cannot assign to const '{name}'"));
        }
        let index = self.resolve_or_insert_global(name);
        chunk.emit(Instruction::SetGlobal(index));
        Ok(())
    }

    fn store_const_global(&mut self, chunk: &mut Chunk, name: &str) -> Result<()> {
        let index = self.resolve_or_insert_global(name);
        chunk.emit(Instruction::SetGlobal(index));
        self.const_globals.insert(name.to_string());
        Ok(())
    }

    fn compile_function(&mut self, function: &FunctionStatement, chunk: &mut Chunk) -> Result<()> {
        let base_index = self.ensure_base_function(function)?;
        let global_index = self.resolve_or_insert_global(&function.name);
        let constant_index = chunk.add_constant(Value::Function(base_index));
        chunk.emit(Instruction::Constant(constant_index));
        chunk.emit(Instruction::SetGlobal(global_index));
        chunk.emit(Instruction::Pop);

        if let Some(instances) = self.function_instances.get(&function.name).cloned() {
            for instance in instances {
                self.ensure_function_specialization(&function.name, &instance)?;
            }
        }
        Ok(())
    }

    fn compile_test(&mut self, test: &TestStatement) -> Result<()> {
        let mut function_chunk = Chunk::new();
        let mut resolver = FunctionResolver::new(&[]);
        let returns_value = self.compile_block(&test.body, &mut function_chunk, &mut resolver)?;

        if !matches!(
            function_chunk.instructions.last(),
            Some(Instruction::Return)
        ) {
            if !returns_value {
                let nil_index = function_chunk.add_constant(Value::Nil);
                function_chunk.emit(Instruction::Constant(nil_index));
            }
            function_chunk.emit(Instruction::Return);
        }

        let function_index = self.functions.len();
        self.functions.push(Function {
            name: format!("test {}", test.name),
            arity: 0,
            chunk: function_chunk,
        });

        self.tests.push(TestCase {
            name: test.name.clone(),
            name_span: test.name_span,
            function_index,
        });

        Ok(())
    }

    fn compile_loop<R: Resolver>(
        &mut self,
        statement: &LoopStatement,
        chunk: &mut Chunk,
        resolver: &mut R,
    ) -> Result<()> {
        match statement.kind {
            LoopKind::While | LoopKind::Until => {
                let LoopHeader::Condition(condition) = &statement.header else {
                    return Err(CodegenError::Unsupported("loop header").into());
                };

                let loop_start = chunk.len();
                self.compile_expression(condition, chunk, resolver)?;
                if matches!(statement.kind, LoopKind::Until) {
                    chunk.emit(Instruction::Not);
                }

                let exit_jump = self.emit_jump(chunk, Instruction::JumpIfFalse(usize::MAX));
                let _ = self.compile_block(&statement.body, chunk, resolver)?;
                chunk.emit(Instruction::Jump(loop_start));
                self.patch_jump(chunk, exit_jump)?;
                Ok(())
            }
            LoopKind::For => Err(CodegenError::Unsupported("for loop").into()),
        }
    }

    fn compile_list_literal<R: Resolver>(
        &mut self,
        list: &ListLiteral,
        chunk: &mut Chunk,
        resolver: &mut R,
    ) -> Result<()> {
        for element in &list.elements {
            self.compile_expression(element, chunk, resolver)?;
        }
        chunk.emit(Instruction::MakeList(list.elements.len()));
        Ok(())
    }

    fn compile_dict_literal<R: Resolver>(
        &mut self,
        dict: &DictLiteral,
        chunk: &mut Chunk,
        resolver: &mut R,
    ) -> Result<()> {
        for entry in &dict.entries {
            let key_constant = chunk.add_constant(Value::String(entry.key.clone()));
            chunk.emit(Instruction::Constant(key_constant));
            self.compile_expression(&entry.value, chunk, resolver)?;
        }
        chunk.emit(Instruction::MakeDict(dict.entries.len()));
        Ok(())
    }

    fn compile_index_expression<R: Resolver>(
        &mut self,
        index: &IndexExpression,
        chunk: &mut Chunk,
        resolver: &mut R,
    ) -> Result<()> {
        self.compile_expression(&index.object, chunk, resolver)?;
        self.compile_expression(&index.index, chunk, resolver)?;
        chunk.emit(Instruction::Index);
        Ok(())
    }

    fn compile_member_expression<R: Resolver>(
        &mut self,
        member: &crate::ast::MemberExpression,
        chunk: &mut Chunk,
        resolver: &mut R,
    ) -> Result<()> {
        self.compile_expression(&member.object, chunk, resolver)?;
        let constant = chunk.add_constant(Value::String(member.property.clone()));
        chunk.emit(Instruction::Constant(constant));
        chunk.emit(Instruction::GetField);
        Ok(())
    }

    fn compile_match_expression<R: Resolver>(
        &mut self,
        expression: &MatchExpression,
        chunk: &mut Chunk,
        resolver: &mut R,
    ) -> Result<()> {
        if expression.arms.is_empty() {
            let nil_index = chunk.add_constant(Value::Nil);
            chunk.emit(Instruction::Constant(nil_index));
            return Ok(());
        }

        self.compile_expression(&expression.scrutinee, chunk, resolver)?;
        let temp_slot = self.allocate_temp_slot(resolver)?;
        self.store_temp_slot(&temp_slot, chunk, resolver)?;
        chunk.emit(Instruction::Pop);

        let mut end_jumps = Vec::new();

        for arm in &expression.arms {
            let (matched_jumps, fallthroughs) =
                self.emit_match_arm_conditions(&temp_slot, &arm.patterns, chunk, resolver)?;

            for jump in matched_jumps {
                self.patch_jump(chunk, jump)?;
            }

            self.compile_expression(&arm.expression, chunk, resolver)?;
            let exit_jump = self.emit_jump(chunk, Instruction::Jump(usize::MAX));
            end_jumps.push(exit_jump);

            for jump in fallthroughs {
                self.patch_jump(chunk, jump)?;
            }
        }

        let nil_index = chunk.add_constant(Value::Nil);
        chunk.emit(Instruction::Constant(nil_index));

        for jump in end_jumps {
            self.patch_jump(chunk, jump)?;
        }

        Ok(())
    }

    fn compile_match_statement<R: Resolver>(
        &mut self,
        statement: &MatchStatement,
        chunk: &mut Chunk,
        resolver: &mut R,
    ) -> Result<()> {
        if statement.arms.is_empty() {
            return Ok(());
        }

        self.compile_expression(&statement.scrutinee, chunk, resolver)?;
        let temp_slot = self.allocate_temp_slot(resolver)?;
        self.store_temp_slot(&temp_slot, chunk, resolver)?;
        chunk.emit(Instruction::Pop);

        let mut end_jumps = Vec::new();

        for arm in &statement.arms {
            let (matched_jumps, fallthroughs) =
                self.emit_match_arm_conditions(&temp_slot, &arm.patterns, chunk, resolver)?;

            for jump in matched_jumps {
                self.patch_jump(chunk, jump)?;
            }

            let _ = self.compile_block(&arm.block, chunk, resolver)?;
            let exit_jump = self.emit_jump(chunk, Instruction::Jump(usize::MAX));
            end_jumps.push(exit_jump);

            for jump in fallthroughs {
                self.patch_jump(chunk, jump)?;
            }
        }

        for jump in end_jumps {
            self.patch_jump(chunk, jump)?;
        }

        Ok(())
    }

    fn emit_match_arm_conditions<R: Resolver>(
        &mut self,
        temp_slot: &TempSlot,
        patterns: &[MatchPattern],
        chunk: &mut Chunk,
        resolver: &mut R,
    ) -> Result<(Vec<usize>, Vec<usize>)> {
        let mut matched_jumps = Vec::new();
        let mut fallthrough_jumps = Vec::new();

        for pattern in patterns {
            for jump in fallthrough_jumps.drain(..) {
                self.patch_jump(chunk, jump)?;
            }

            match pattern {
                MatchPattern::Wildcard { .. } => {
                    let jump = self.emit_jump(chunk, Instruction::Jump(usize::MAX));
                    matched_jumps.push(jump);
                    return Ok((matched_jumps, Vec::new()));
                }
                MatchPattern::Expression(expr) => {
                    self.load_temp_slot(temp_slot, chunk, resolver)?;
                    self.compile_expression(expr, chunk, resolver)?;
                    chunk.emit(Instruction::Equal);
                    let skip = self.emit_jump(chunk, Instruction::JumpIfFalse(usize::MAX));
                    let matched = self.emit_jump(chunk, Instruction::Jump(usize::MAX));
                    matched_jumps.push(matched);
                    fallthrough_jumps.push(skip);
                }
            }
        }

        Ok((matched_jumps, fallthrough_jumps))
    }

    fn emit_enum_variant(
        &mut self,
        metadata: EnumVariantMetadata,
        chunk: &mut Chunk,
    ) -> Result<()> {
        let value = Value::EnumVariant(Rc::new(EnumVariantValue {
            enum_name: metadata.enum_name.clone(),
            variant_name: metadata.variant_name.clone(),
            discriminant: metadata.discriminant,
        }));
        let index = chunk.add_constant(value);
        chunk.emit(Instruction::Constant(index));
        Ok(())
    }

    fn enum_variant_metadata_from_ast(
        &mut self,
        member: &MemberExpression,
        span: SourceSpan,
    ) -> Option<EnumVariantMetadata> {
        if let ExpressionKind::Identifier(identifier) = &member.object.kind {
            if let Some(enum_definition) = self.enum_definitions.get(&identifier.name) {
                if let Some(variant) = enum_definition
                    .variants
                    .iter()
                    .find(|variant| variant.name == member.property)
                {
                    let metadata = EnumVariantMetadata {
                        enum_name: identifier.name.clone(),
                        variant_name: variant.name.clone(),
                        discriminant: variant.discriminant,
                    };
                    self.enum_variant_metadata.insert(span, metadata.clone());
                    return Some(metadata);
                }
            }
        }
        None
    }

    fn compile_lambda_expression<R: Resolver>(
        &mut self,
        lambda: &LambdaExpression,
        chunk: &mut Chunk,
        resolver: &mut R,
    ) -> Result<()> {
        let captures = self
            .lambda_captures
            .get(&lambda.id)
            .cloned()
            .unwrap_or_else(Vec::new);

        let function_index = self.compile_lambda_function(lambda, &captures)?;

        for capture in &captures {
            resolver.load(self, chunk, capture)?;
        }

        chunk.emit(Instruction::MakeClosure {
            function_index,
            capture_count: captures.len(),
        });
        Ok(())
    }

    fn compile_lambda_function(
        &mut self,
        lambda: &LambdaExpression,
        captures: &[String],
    ) -> Result<usize> {
        let mut function_chunk = Chunk::new();
        let mut resolver = FunctionResolver::with_captures(captures, &lambda.parameters);
        let returns_value = match &lambda.body {
            LambdaBody::Expression(expr) => {
                self.compile_expression(expr, &mut function_chunk, &mut resolver)?;
                true
            }
            LambdaBody::Block(block) => {
                self.compile_block(block, &mut function_chunk, &mut resolver)?
            }
        };

        if !matches!(
            function_chunk.instructions.last(),
            Some(Instruction::Return)
        ) {
            if !returns_value {
                let nil_index = function_chunk.add_constant(Value::Nil);
                function_chunk.emit(Instruction::Constant(nil_index));
            }
            function_chunk.emit(Instruction::Return);
        }

        let function_index = self.functions.len();
        self.functions.push(Function {
            name: format!("<lambda:{}>", lambda.id),
            arity: lambda.parameters.len(),
            chunk: function_chunk,
        });
        Ok(function_index)
    }

    fn compile_use<R: Resolver>(
        &mut self,
        statement: &UseStatement,
        resolver: &mut R,
    ) -> Result<()> {
        if resolver.kind() != ResolverKind::Global {
            return Err(CodegenError::Unsupported("use in function").into());
        }

        let module_path = statement.module_path.as_str();
        if let Some(module) = stdlib::find_module(module_path) {
            let entry = self
                .module_builtins
                .entry(statement.alias.name.clone())
                .or_insert_with(HashMap::new);
            for function in module.functions {
                self.builtins
                    .insert(function.name.to_string(), function.kind);
                entry.insert(function.name.to_string(), function.kind);
            }
            Ok(())
        } else if module_path.starts_with("std.") || module_path.starts_with("support.") {
            bail!(format!("unknown module '{module_path}'"));
        } else {
            Ok(())
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ResolverKind {
    Global,
    Function,
}

enum TempSlot {
    ResolverLocal { name: String },
    DirectLocal { index: usize },
}

trait Resolver {
    fn kind(&self) -> ResolverKind;
    fn declare_local(&mut self, name: &str, is_const: bool) -> Result<usize>;
    fn load(&mut self, generator: &mut CodeGenerator, chunk: &mut Chunk, name: &str) -> Result<()>;
    fn store(&mut self, generator: &mut CodeGenerator, chunk: &mut Chunk, name: &str)
        -> Result<()>;
}

struct GlobalResolver;

impl Resolver for GlobalResolver {
    fn kind(&self) -> ResolverKind {
        ResolverKind::Global
    }

    fn declare_local(&mut self, _name: &str, _is_const: bool) -> Result<usize> {
        Err(CodegenError::Unsupported("locals").into())
    }

    fn load(&mut self, generator: &mut CodeGenerator, chunk: &mut Chunk, name: &str) -> Result<()> {
        generator.load_global(chunk, name)
    }

    fn store(
        &mut self,
        generator: &mut CodeGenerator,
        chunk: &mut Chunk,
        name: &str,
    ) -> Result<()> {
        generator.store_global(chunk, name)
    }
}

struct FunctionResolver {
    locals: HashMap<String, usize>,
    const_locals: HashSet<String>,
    next_local_index: usize,
}

impl FunctionResolver {
    fn new(parameters: &[FunctionParameter]) -> Self {
        Self::with_captures(&[], parameters)
    }

    fn with_captures(captures: &[String], parameters: &[FunctionParameter]) -> Self {
        let mut locals = HashMap::new();
        for (index, name) in captures.iter().enumerate() {
            locals.insert(name.clone(), index);
        }
        let mut next_local_index = captures.len();
        for param in parameters {
            locals.insert(param.name.clone(), next_local_index);
            next_local_index += 1;
        }
        Self {
            locals,
            const_locals: HashSet::new(),
            next_local_index,
        }
    }
}

impl Resolver for FunctionResolver {
    fn kind(&self) -> ResolverKind {
        ResolverKind::Function
    }

    fn declare_local(&mut self, name: &str, is_const: bool) -> Result<usize> {
        if let Some(index) = self.locals.get(name).cloned() {
            if is_const {
                self.const_locals.insert(name.to_string());
            } else {
                self.const_locals.remove(name);
            }
            Ok(index)
        } else {
            let index = self.next_local_index;
            self.locals.insert(name.to_string(), index);
            if is_const {
                self.const_locals.insert(name.to_string());
            }
            self.next_local_index += 1;
            Ok(index)
        }
    }

    fn load(&mut self, generator: &mut CodeGenerator, chunk: &mut Chunk, name: &str) -> Result<()> {
        if let Some(index) = self.locals.get(name).cloned() {
            chunk.emit(Instruction::GetLocal(index));
            Ok(())
        } else {
            generator.load_global(chunk, name)
        }
    }

    fn store(
        &mut self,
        generator: &mut CodeGenerator,
        chunk: &mut Chunk,
        name: &str,
    ) -> Result<()> {
        if let Some(index) = self.locals.get(name).cloned() {
            if self.const_locals.contains(name) {
                bail!(format!("cannot assign to const '{name}'"));
            }
            chunk.emit(Instruction::SetLocal(index));
            Ok(())
        } else {
            generator.store_global(chunk, name)
        }
    }
}
