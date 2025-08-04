use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};

use crate::ast::{
    Block, Expression, ExpressionKind, Identifier, LambdaBody, LoopHeader, Module, Statement,
    TypeExpression,
};
use crate::diagnostics::Diagnostics;
use crate::lexer::{Lexer, TokenKind};
use crate::parser::Parser;
use crate::resolver::Resolver;
use crate::runtime::{CodeGenerator, Program, VmSemanticMetadata};
use crate::source::{SourceFile, SourceId};
use crate::typechecker::TypeChecker;

#[derive(Debug, Default)]
pub struct CompileOptions {
    pub dump_tokens: bool,
}

pub struct Compilation {
    pub module: Module,
    pub program: Program,
}

pub struct Compiler {
    diagnostics: Diagnostics,
    options: CompileOptions,
}

impl Compiler {
    pub fn new(options: CompileOptions) -> Self {
        Self {
            diagnostics: Diagnostics::new(),
            options,
        }
    }

    pub fn diagnostics(&self) -> &Diagnostics {
        &self.diagnostics
    }

    pub fn compile(&mut self, source: &SourceFile) -> Result<Compilation> {
        let mut lexer = Lexer::new(source)?;
        let tokens = lexer.tokenize()?;

        if self.options.dump_tokens {
            for token in &tokens {
                println!("{token:?}");
            }
        }

        let mut parser = Parser::new(source, tokens);
        let module = match parser.parse() {
            Ok(module) => {
                let diagnostics = parser.into_diagnostics();
                self.diagnostics.extend(diagnostics);
                Ok(module)
            }
            Err(err) => {
                let diagnostics = parser.into_diagnostics();
                self.diagnostics.extend(diagnostics);
                Err(err)
            }
        }?;

        let entry_path = source.path.canonicalize().unwrap_or(source.path.clone());
        let mut expander = ModuleExpander::new(entry_path.clone());
        let expanded_module = match expander.expand_module(&module, &entry_path) {
            Ok(module) => module,
            Err(err) => {
                let diagnostics = expander.into_diagnostics();
                self.diagnostics.extend(diagnostics);
                return Err(err);
            }
        };
        self.diagnostics.extend(expander.into_diagnostics());

        let mut resolver = Resolver::new();
        resolver.resolve_module(&expanded_module);
        let (resolve_diagnostics, lambda_captures) = resolver.into_parts();
        let resolve_errors = resolve_diagnostics.has_errors();
        self.diagnostics.extend(resolve_diagnostics);
        if resolve_errors {
            bail!("Name resolution failed");
        }

        let mut type_checker = TypeChecker::new();
        type_checker.check_module(&expanded_module);
        let function_instances = type_checker.function_instances().clone();
        let function_call_metadata = type_checker.function_call_metadata().clone();
        let struct_call_metadata = type_checker.struct_call_metadata().clone();
        let struct_definitions = type_checker.struct_definitions();
        let type_diagnostics = type_checker.into_diagnostics();
        let type_errors = type_diagnostics.has_errors();
        self.diagnostics.extend(type_diagnostics);
        if type_errors {
            bail!("Type checking failed");
        }

        let metadata = VmSemanticMetadata {
            function_instances,
            function_call_metadata,
            struct_call_metadata,
            struct_definitions,
        };
        let generator = CodeGenerator::new(lambda_captures, metadata);
        let program = generator.compile_module(&expanded_module)?;

        Ok(Compilation {
            module: expanded_module,
            program,
        })
    }
}

struct ModuleExpander {
    visited: HashSet<PathBuf>,
    next_source_id: u32,
    diagnostics: Diagnostics,
    module_cache: HashMap<PathBuf, Module>,
}

impl ModuleExpander {
    fn new(_entry_path: PathBuf) -> Self {
        let visited = HashSet::new();
        Self {
            visited,
            next_source_id: 1,
            diagnostics: Diagnostics::new(),
            module_cache: HashMap::new(),
        }
    }

    fn expand_module(&mut self, module: &Module, path: &Path) -> Result<Module> {
        if let Some(cached) = self.module_cache.get(path) {
            return Ok(cached.clone());
        }

        if !self.visited.insert(path.to_path_buf()) {
            if let Some(cached) = self.module_cache.get(path) {
                return Ok(cached.clone());
            }
            bail!(format!(
                "cyclic module import detected involving '{}'",
                path.display()
            ));
        }

        let statements = self.expand_statements(&module.statements, path)?;
        let expanded = Module::new(statements);
        self.module_cache
            .insert(path.to_path_buf(), expanded.clone());
        Ok(expanded)
    }

    fn into_diagnostics(self) -> Diagnostics {
        self.diagnostics
    }

    fn expand_statements(
        &mut self,
        statements: &[Statement],
        base_path: &Path,
    ) -> Result<Vec<Statement>> {
        let mut result = Vec::new();
        let mut alias_maps: HashMap<String, HashMap<String, String>> = HashMap::new();

        for statement in statements {
            match statement {
                Statement::Use(use_stmt) => {
                    result.push(statement.clone());
                    let path = &use_stmt.module_path;
                    if path.starts_with("std.") || path.starts_with("support.") {
                        continue;
                    }

                    let span = use_stmt.module_span;
                    let resolved = match self.resolve_path(base_path, path) {
                        Ok(resolved) => resolved,
                        Err(err) => {
                            self.diagnostics.push_error_with_span(
                                format!("could not resolve module '{}': {err}", path),
                                Some(span),
                            );
                            return Err(err);
                        }
                    };
                    let canonical = match resolved.canonicalize() {
                        Ok(path) => path,
                        Err(err) => {
                            self.diagnostics.push_error_with_span(
                                format!("failed to resolve module '{}': {err}", path),
                                Some(span),
                            );
                            return Err(err.into());
                        }
                    };

                    let module = match self.module_cache.get(&canonical) {
                        Some(module) => module.clone(),
                        None => {
                            let loaded = match self.load_module(&canonical) {
                                Ok(module) => module,
                                Err(err) => {
                                    self.diagnostics.push_error_with_span(
                                        format!("failed to load module '{}': {}", path, err),
                                        Some(span),
                                    );
                                    return Err(err);
                                }
                            };
                            let expanded = self.expand_module(&loaded, &canonical)?;
                            expanded
                        }
                    };

                    let (mut renamed, rename_map) =
                        self.rename_module_statements(module, &use_stmt.alias.name);
                    alias_maps.insert(use_stmt.alias.name.clone(), rename_map);
                    result.append(&mut renamed);
                }
                _ => result.push(statement.clone()),
            }
        }

        self.rewrite_alias_access(&mut result, &alias_maps);

        Ok(result)
    }

    fn rename_module_statements(
        &self,
        module: Module,
        alias: &str,
    ) -> (Vec<Statement>, HashMap<String, String>) {
        let mut rename_map: HashMap<String, String> = HashMap::new();
        for statement in &module.statements {
            match statement {
                Statement::Function(function) => {
                    rename_map.insert(
                        function.name.clone(),
                        format!("__module_{}_{}", alias, function.name),
                    );
                }
                Statement::Var(var_stmt) => {
                    for binding in &var_stmt.bindings {
                        rename_map.insert(
                            binding.name.clone(),
                            format!("__module_{}_{}", alias, binding.name),
                        );
                    }
                }
                Statement::Struct(struct_stmt) => {
                    rename_map.insert(
                        struct_stmt.name.clone(),
                        format!("__module_{}_{}", alias, struct_stmt.name),
                    );
                }
                _ => {}
            }
        }

        let mut renamed = Vec::new();
        for mut statement in module.statements {
            match &mut statement {
                Statement::Function(function) => {
                    if let Some(new_name) = rename_map.get(&function.name).cloned() {
                        function.name = new_name;
                    }
                }
                Statement::Var(var_stmt) => {
                    for binding in &mut var_stmt.bindings {
                        if let Some(new_name) = rename_map.get(&binding.name).cloned() {
                            binding.name = new_name;
                        }
                    }
                }
                Statement::Struct(struct_stmt) => {
                    if let Some(new_name) = rename_map.get(&struct_stmt.name).cloned() {
                        struct_stmt.name = new_name;
                    }
                }
                _ => {}
            }

            self.rewrite_statement_identifiers(&mut statement, &rename_map);
            renamed.push(statement);
        }

        (renamed, rename_map)
    }

    fn rewrite_alias_access(
        &self,
        statements: &mut [Statement],
        alias_maps: &HashMap<String, HashMap<String, String>>,
    ) {
        if alias_maps.is_empty() {
            return;
        }
        for statement in statements {
            self.rewrite_statement_alias(statement, alias_maps);
        }
    }

    fn rewrite_block_identifiers(&self, block: &mut Block, rename_map: &HashMap<String, String>) {
        for statement in &mut block.statements {
            self.rewrite_statement_identifiers(statement, rename_map);
        }
    }

    fn rewrite_type_expression_identifiers(
        &self,
        type_expression: &mut TypeExpression,
        rename_map: &HashMap<String, String>,
    ) {
        for token in &mut type_expression.tokens {
            if matches!(token.kind, TokenKind::Identifier) {
                if let Some(new_name) = rename_map.get(&token.lexeme) {
                    token.lexeme = new_name.clone();
                }
            }
        }
    }

    fn rewrite_type_expression_alias(
        &self,
        type_expression: &mut TypeExpression,
        alias_maps: &HashMap<String, HashMap<String, String>>,
    ) {
        if alias_maps.is_empty() {
            return;
        }

        let mut index = 0;
        while index + 2 < type_expression.tokens.len() {
            let first = &type_expression.tokens[index];
            let second = &type_expression.tokens[index + 1];
            let third = &type_expression.tokens[index + 2];

            if matches!(first.kind, TokenKind::Identifier)
                && matches!(second.kind, TokenKind::Dot)
                && matches!(third.kind, TokenKind::Identifier)
            {
                if let Some(map) = alias_maps.get(&first.lexeme) {
                    if let Some(replacement) = map.get(&third.lexeme) {
                        type_expression.tokens[index].lexeme = replacement.clone();
                        type_expression.tokens[index].kind = TokenKind::Identifier;
                        type_expression.tokens.remove(index + 2);
                        type_expression.tokens.remove(index + 1);
                        continue;
                    }
                }
            }

            index += 1;
        }
    }

    fn rewrite_statement_identifiers(
        &self,
        statement: &mut Statement,
        rename_map: &HashMap<String, String>,
    ) {
        match statement {
            Statement::Use(_) => {}
            Statement::Var(var_stmt) => {
                for binding in &mut var_stmt.bindings {
                    if let Some(type_annotation) = &mut binding.type_annotation {
                        self.rewrite_type_expression_identifiers(type_annotation, rename_map);
                    }
                    if let Some(initializer) = &mut binding.initializer {
                        self.rewrite_expression_identifiers(initializer, rename_map);
                    }
                }
            }
            Statement::Function(function_stmt) => {
                if let Some(return_type) = &mut function_stmt.return_type {
                    self.rewrite_type_expression_identifiers(return_type, rename_map);
                }
                for parameter in &mut function_stmt.parameters {
                    if let Some(type_annotation) = &mut parameter.type_annotation {
                        self.rewrite_type_expression_identifiers(type_annotation, rename_map);
                    }
                    if let Some(default_value) = &mut parameter.default_value {
                        self.rewrite_expression_identifiers(default_value, rename_map);
                    }
                }
                self.rewrite_block_identifiers(&mut function_stmt.body, rename_map);
            }
            Statement::Test(test_stmt) => {
                self.rewrite_block_identifiers(&mut test_stmt.body, rename_map);
            }
            Statement::Struct(struct_stmt) => {
                for field in &mut struct_stmt.fields {
                    self.rewrite_type_expression_identifiers(
                        &mut field.type_annotation,
                        rename_map,
                    );
                }
            }
            Statement::Conditional(cond_stmt) => {
                self.rewrite_expression_identifiers(&mut cond_stmt.condition, rename_map);
                self.rewrite_block_identifiers(&mut cond_stmt.consequent, rename_map);
                if let Some(alternative) = &mut cond_stmt.alternative {
                    self.rewrite_block_identifiers(alternative, rename_map);
                }
            }
            Statement::Loop(loop_stmt) => {
                match &mut loop_stmt.header {
                    LoopHeader::For { pattern, iterator } => {
                        self.rewrite_expression_identifiers(pattern, rename_map);
                        self.rewrite_expression_identifiers(iterator, rename_map);
                    }
                    LoopHeader::Condition(expr) => {
                        self.rewrite_expression_identifiers(expr, rename_map);
                    }
                }
                self.rewrite_block_identifiers(&mut loop_stmt.body, rename_map);
            }
            Statement::Return(ret_stmt) => {
                if let Some(expression) = &mut ret_stmt.expression {
                    self.rewrite_expression_identifiers(expression, rename_map);
                }
            }
            Statement::Expression(expr_stmt) => {
                self.rewrite_expression_identifiers(&mut expr_stmt.expression, rename_map);
            }
        }
    }

    fn rewrite_expression_identifiers(
        &self,
        expression: &mut Expression,
        rename_map: &HashMap<String, String>,
    ) {
        match &mut expression.kind {
            ExpressionKind::Identifier(identifier) => {
                if let Some(new_name) = rename_map.get(&identifier.name) {
                    identifier.name = new_name.clone();
                }
            }
            ExpressionKind::Literal(_) => {}
            ExpressionKind::List(list) => {
                for element in &mut list.elements {
                    self.rewrite_expression_identifiers(element, rename_map);
                }
            }
            ExpressionKind::Dict(dict) => {
                for entry in &mut dict.entries {
                    self.rewrite_expression_identifiers(&mut entry.value, rename_map);
                }
            }
            ExpressionKind::Unary(unary) => {
                self.rewrite_expression_identifiers(&mut unary.operand, rename_map);
            }
            ExpressionKind::Binary(binary) => {
                self.rewrite_expression_identifiers(&mut binary.left, rename_map);
                self.rewrite_expression_identifiers(&mut binary.right, rename_map);
            }
            ExpressionKind::Call(call) => {
                self.rewrite_expression_identifiers(&mut call.callee, rename_map);
                for argument in &mut call.arguments {
                    self.rewrite_expression_identifiers(&mut argument.expression, rename_map);
                }
            }
            ExpressionKind::Member(member) => {
                self.rewrite_expression_identifiers(&mut member.object, rename_map);
            }
            ExpressionKind::Index(index) => {
                self.rewrite_expression_identifiers(&mut index.object, rename_map);
                self.rewrite_expression_identifiers(&mut index.index, rename_map);
            }
            ExpressionKind::Range(range) => {
                self.rewrite_expression_identifiers(&mut range.start, rename_map);
                self.rewrite_expression_identifiers(&mut range.end, rename_map);
            }
            ExpressionKind::Lambda(lambda) => match &mut lambda.body {
                LambdaBody::Expression(expr) => {
                    self.rewrite_expression_identifiers(expr, rename_map);
                }
                LambdaBody::Block(block) => {
                    self.rewrite_block_identifiers(block, rename_map);
                }
            },
            ExpressionKind::Assignment(assignment) => {
                self.rewrite_expression_identifiers(&mut assignment.target, rename_map);
                self.rewrite_expression_identifiers(&mut assignment.value, rename_map);
            }
            ExpressionKind::Grouping(expr) => {
                self.rewrite_expression_identifiers(expr, rename_map);
            }
        }
    }

    fn rewrite_statement_alias(
        &self,
        statement: &mut Statement,
        alias_maps: &HashMap<String, HashMap<String, String>>,
    ) {
        match statement {
            Statement::Use(_) => {}
            Statement::Var(var_stmt) => {
                for binding in &mut var_stmt.bindings {
                    if let Some(type_annotation) = &mut binding.type_annotation {
                        self.rewrite_type_expression_alias(type_annotation, alias_maps);
                    }
                    if let Some(initializer) = &mut binding.initializer {
                        self.rewrite_expression_alias(initializer, alias_maps);
                    }
                }
            }
            Statement::Function(function_stmt) => {
                if let Some(return_type) = &mut function_stmt.return_type {
                    self.rewrite_type_expression_alias(return_type, alias_maps);
                }
                for parameter in &mut function_stmt.parameters {
                    if let Some(type_annotation) = &mut parameter.type_annotation {
                        self.rewrite_type_expression_alias(type_annotation, alias_maps);
                    }
                    if let Some(default_value) = &mut parameter.default_value {
                        self.rewrite_expression_alias(default_value, alias_maps);
                    }
                }
                self.rewrite_block_alias(&mut function_stmt.body, alias_maps);
            }
            Statement::Test(test_stmt) => {
                self.rewrite_block_alias(&mut test_stmt.body, alias_maps);
            }
            Statement::Struct(struct_stmt) => {
                for field in &mut struct_stmt.fields {
                    self.rewrite_type_expression_alias(&mut field.type_annotation, alias_maps);
                }
            }
            Statement::Conditional(cond_stmt) => {
                self.rewrite_expression_alias(&mut cond_stmt.condition, alias_maps);
                self.rewrite_block_alias(&mut cond_stmt.consequent, alias_maps);
                if let Some(alternative) = &mut cond_stmt.alternative {
                    self.rewrite_block_alias(alternative, alias_maps);
                }
            }
            Statement::Loop(loop_stmt) => {
                match &mut loop_stmt.header {
                    LoopHeader::For { pattern, iterator } => {
                        self.rewrite_expression_alias(pattern, alias_maps);
                        self.rewrite_expression_alias(iterator, alias_maps);
                    }
                    LoopHeader::Condition(expr) => {
                        self.rewrite_expression_alias(expr, alias_maps);
                    }
                }
                self.rewrite_block_alias(&mut loop_stmt.body, alias_maps);
            }
            Statement::Return(ret_stmt) => {
                if let Some(expression) = &mut ret_stmt.expression {
                    self.rewrite_expression_alias(expression, alias_maps);
                }
            }
            Statement::Expression(expr_stmt) => {
                self.rewrite_expression_alias(&mut expr_stmt.expression, alias_maps);
            }
        }
    }

    fn rewrite_block_alias(
        &self,
        block: &mut Block,
        alias_maps: &HashMap<String, HashMap<String, String>>,
    ) {
        for statement in &mut block.statements {
            self.rewrite_statement_alias(statement, alias_maps);
        }
    }

    fn rewrite_expression_alias(
        &self,
        expression: &mut Expression,
        alias_maps: &HashMap<String, HashMap<String, String>>,
    ) {
        match &mut expression.kind {
            ExpressionKind::Member(member) => {
                self.rewrite_expression_alias(&mut member.object, alias_maps);
                if let ExpressionKind::Identifier(identifier) = &member.object.kind {
                    if let Some(map) = alias_maps.get(&identifier.name) {
                        if let Some(replacement) = map.get(&member.property) {
                            expression.kind = ExpressionKind::Identifier(Identifier {
                                name: replacement.clone(),
                                span: member.property_span,
                            });
                            return;
                        }
                    }
                }
            }
            ExpressionKind::Call(call) => {
                self.rewrite_expression_alias(&mut call.callee, alias_maps);
                for argument in &mut call.arguments {
                    self.rewrite_expression_alias(&mut argument.expression, alias_maps);
                }
            }
            ExpressionKind::Assignment(assignment) => {
                self.rewrite_expression_alias(&mut assignment.target, alias_maps);
                self.rewrite_expression_alias(&mut assignment.value, alias_maps);
            }
            ExpressionKind::Binary(binary) => {
                self.rewrite_expression_alias(&mut binary.left, alias_maps);
                self.rewrite_expression_alias(&mut binary.right, alias_maps);
            }
            ExpressionKind::Unary(unary) => {
                self.rewrite_expression_alias(&mut unary.operand, alias_maps);
            }
            ExpressionKind::List(list) => {
                for element in &mut list.elements {
                    self.rewrite_expression_alias(element, alias_maps);
                }
            }
            ExpressionKind::Dict(dict) => {
                for entry in &mut dict.entries {
                    self.rewrite_expression_alias(&mut entry.value, alias_maps);
                }
            }
            ExpressionKind::Range(range) => {
                self.rewrite_expression_alias(&mut range.start, alias_maps);
                self.rewrite_expression_alias(&mut range.end, alias_maps);
            }
            ExpressionKind::Lambda(lambda) => match &mut lambda.body {
                LambdaBody::Expression(expr) => {
                    self.rewrite_expression_alias(expr, alias_maps);
                }
                LambdaBody::Block(block) => {
                    self.rewrite_block_alias(block, alias_maps);
                }
            },
            ExpressionKind::Grouping(expr) => {
                self.rewrite_expression_alias(expr, alias_maps);
            }
            ExpressionKind::Identifier(_) | ExpressionKind::Literal(_) => {}
            ExpressionKind::Index(index) => {
                self.rewrite_expression_alias(&mut index.object, alias_maps);
                self.rewrite_expression_alias(&mut index.index, alias_maps);
            }
        }
    }

    fn resolve_path(&self, base_path: &Path, import: &str) -> Result<PathBuf> {
        let base_dir = base_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        let mut path = if Path::new(import).is_absolute() {
            PathBuf::from(import)
        } else {
            base_dir.join(import)
        };
        if path.extension().is_none() {
            path.set_extension("tea");
        }
        Ok(path)
    }

    fn load_module(&mut self, path: &Path) -> Result<Module> {
        let contents = fs::read_to_string(path)
            .with_context(|| format!("failed to read module at '{}'", path.display()))?;
        let source = SourceFile::new(SourceId(self.next_source_id), path.to_path_buf(), contents);
        self.next_source_id += 1;

        let mut lexer = Lexer::new(&source)?;
        let tokens = lexer.tokenize()?;
        let mut parser = Parser::new(&source, tokens);
        let module = parser.parse()?;
        let diagnostics = parser.into_diagnostics();
        if !diagnostics.is_empty() {
            let messages = diagnostics
                .entries()
                .iter()
                .map(|d| d.message.clone())
                .collect::<Vec<_>>()
                .join("; ");
            bail!(messages);
        }
        Ok(module)
    }
}
