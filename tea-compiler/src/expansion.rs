use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{bail, Context, Result};

use crate::ast::{
    Block, CatchHandler, CatchKind, Expression, ExpressionKind, Identifier, InterpolatedStringPart,
    LambdaBody, LoopHeader, MatchPattern, Module, Statement, TypeExpression,
};
use crate::diagnostics::Diagnostics;
use crate::lexer::{Lexer, TokenKind};
use crate::loader::ModuleLoader;
use crate::parser::Parser;
use crate::source::{SourceFile, SourceId};

pub struct ExpandedModule {
    pub module: Module,
    alias_exports: HashMap<String, Vec<String>>,
    alias_export_renames: HashMap<String, HashMap<String, String>>,
    alias_export_docstrings: HashMap<String, HashMap<String, String>>,
}

impl ExpandedModule {
    pub fn module(&self) -> &Module {
        &self.module
    }

    pub(crate) fn into_parts(
        self,
    ) -> (
        Module,
        HashMap<String, Vec<String>>,
        HashMap<String, HashMap<String, String>>,
        HashMap<String, HashMap<String, String>>,
    ) {
        (
            self.module,
            self.alias_exports,
            self.alias_export_renames,
            self.alias_export_docstrings,
        )
    }
}

pub(crate) struct ModuleExpander {
    visited: HashSet<PathBuf>,
    next_source_id: u32,
    diagnostics: Diagnostics,
    module_cache: HashMap<PathBuf, Module>,
    alias_exports: HashMap<String, Vec<String>>,
    alias_export_renames: HashMap<String, HashMap<String, String>>,
    alias_export_docstrings: HashMap<String, HashMap<String, String>>,
    module_overrides: HashMap<PathBuf, String>,
    loader: Arc<dyn ModuleLoader>,
}

impl ModuleExpander {
    pub(crate) fn new(
        module_overrides: HashMap<PathBuf, String>,
        loader: Option<Arc<dyn ModuleLoader>>,
    ) -> Result<Self> {
        Ok(Self {
            visited: HashSet::new(),
            next_source_id: 1,
            diagnostics: Diagnostics::new(),
            module_cache: HashMap::new(),
            alias_exports: HashMap::new(),
            alias_export_renames: HashMap::new(),
            alias_export_docstrings: HashMap::new(),
            module_overrides,
            loader: default_loader(loader)?,
        })
    }

    pub(crate) fn expand(&mut self, module: &Module, path: &Path) -> Result<ExpandedModule> {
        let module = self.expand_module(module, path)?;
        Ok(ExpandedModule {
            module,
            alias_exports: self.alias_exports.clone(),
            alias_export_renames: self.alias_export_renames.clone(),
            alias_export_docstrings: self.alias_export_docstrings.clone(),
        })
    }

    pub(crate) fn into_diagnostics(self) -> Diagnostics {
        self.diagnostics
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

                    let resolved_path = match self.loader.resolve_import(base_path, path) {
                        Ok(Some(resolved_path)) => resolved_path,
                        Ok(None) => continue,
                        Err(err) => {
                            let span = use_stmt.module_span;
                            self.diagnostics.push_error_with_span(
                                format!("could not resolve module '{}': {err}", path),
                                Some(span),
                            );
                            return Err(err);
                        }
                    };

                    let span = use_stmt.module_span;
                    let canonical = match self.loader.canonicalize(&resolved_path) {
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
                            self.expand_module(&loaded, &canonical)?
                        }
                    };

                    let (mut renamed, export_renames, docstrings) =
                        self.rename_module_statements(module, &use_stmt.alias.name);
                    let exports = export_renames.keys().cloned().collect();
                    self.alias_exports
                        .insert(use_stmt.alias.name.clone(), exports);
                    self.alias_export_renames
                        .insert(use_stmt.alias.name.clone(), export_renames.clone());
                    if !docstrings.is_empty() {
                        self.alias_export_docstrings
                            .insert(use_stmt.alias.name.clone(), docstrings);
                    }
                    alias_maps.insert(use_stmt.alias.name.clone(), export_renames);
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
    ) -> (
        Vec<Statement>,
        HashMap<String, String>,
        HashMap<String, String>,
    ) {
        let mut all_renames: HashMap<String, String> = HashMap::new();
        let mut export_renames: HashMap<String, String> = HashMap::new();
        let mut docstrings: HashMap<String, String> = HashMap::new();

        for statement in &module.statements {
            if let Statement::Use(use_stmt) = statement {
                let original_alias = use_stmt.alias.name.clone();
                let new_alias = format!("__module_{}__{}", alias, original_alias);
                all_renames.insert(original_alias, new_alias);
            }
        }

        for statement in &module.statements {
            match statement {
                Statement::Function(function) => {
                    let renamed = format!("__module_{}_{}", alias, function.name);
                    all_renames.insert(function.name.clone(), renamed.clone());
                    if function.is_public {
                        export_renames.insert(function.name.clone(), renamed);
                        if let Some(doc) = function.docstring.as_ref() {
                            if !doc.is_empty() {
                                docstrings.insert(function.name.clone(), doc.clone());
                            }
                        }
                    }
                }
                Statement::Var(var_stmt) => {
                    for binding in &var_stmt.bindings {
                        let renamed = format!("__module_{}_{}", alias, binding.name);
                        all_renames.insert(binding.name.clone(), renamed.clone());
                        export_renames.insert(binding.name.clone(), renamed);
                        if let Some(doc) = var_stmt.docstring.as_ref() {
                            if !doc.is_empty() {
                                docstrings.insert(binding.name.clone(), doc.clone());
                            }
                        }
                    }
                }
                Statement::Struct(struct_stmt) => {
                    let renamed = format!("__module_{}_{}", alias, struct_stmt.name);
                    all_renames.insert(struct_stmt.name.clone(), renamed.clone());
                    export_renames.insert(struct_stmt.name.clone(), renamed);
                    if let Some(doc) = struct_stmt.docstring.as_ref() {
                        if !doc.is_empty() {
                            docstrings.insert(struct_stmt.name.clone(), doc.clone());
                        }
                    }
                }
                Statement::Union(union_stmt) => {
                    let renamed = format!("__module_{}_{}", alias, union_stmt.name);
                    all_renames.insert(union_stmt.name.clone(), renamed.clone());
                    export_renames.insert(union_stmt.name.clone(), renamed);
                    if let Some(doc) = union_stmt.docstring.as_ref() {
                        if !doc.is_empty() {
                            docstrings.insert(union_stmt.name.clone(), doc.clone());
                        }
                    }
                }
                Statement::Enum(enum_stmt) => {
                    let renamed = format!("__module_{}_{}", alias, enum_stmt.name);
                    all_renames.insert(enum_stmt.name.clone(), renamed.clone());
                    export_renames.insert(enum_stmt.name.clone(), renamed);
                    if let Some(doc) = enum_stmt.docstring.clone() {
                        if !doc.is_empty() {
                            docstrings.insert(enum_stmt.name.clone(), doc);
                        }
                    }
                }
                _ => {}
            }
        }

        let mut renamed = Vec::new();
        for mut statement in module.statements {
            match &mut statement {
                Statement::Function(function) => {
                    if let Some(new_name) = all_renames.get(&function.name).cloned() {
                        function.name = new_name;
                    }
                }
                Statement::Var(var_stmt) => {
                    for binding in &mut var_stmt.bindings {
                        if let Some(new_name) = all_renames.get(&binding.name).cloned() {
                            binding.name = new_name;
                        }
                    }
                }
                Statement::Struct(struct_stmt) => {
                    if let Some(new_name) = all_renames.get(&struct_stmt.name).cloned() {
                        struct_stmt.name = new_name;
                    }
                }
                Statement::Union(union_stmt) => {
                    if let Some(new_name) = all_renames.get(&union_stmt.name).cloned() {
                        union_stmt.name = new_name;
                    }
                }
                Statement::Enum(enum_stmt) => {
                    if let Some(new_name) = all_renames.get(&enum_stmt.name).cloned() {
                        enum_stmt.name = new_name;
                    }
                }
                _ => {}
            }

            if let Statement::Use(use_stmt) = &mut statement {
                if let Some(new_alias) = all_renames.get(&use_stmt.alias.name).cloned() {
                    use_stmt.alias.name = new_alias;
                }
            }

            self.rewrite_statement_identifiers(&mut statement, &all_renames);
            renamed.push(statement);
        }

        (renamed, export_renames, docstrings)
    }

    fn rewrite_alias_access(
        &mut self,
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
            Statement::Union(union_stmt) => {
                for member in &mut union_stmt.members {
                    self.rewrite_type_expression_identifiers(
                        &mut member.type_expression,
                        rename_map,
                    );
                }
            }
            Statement::Enum(_) => {}
            Statement::Error(_) => {}
            Statement::Conditional(cond_stmt) => {
                self.rewrite_expression_identifiers(&mut cond_stmt.condition, rename_map);
                self.rewrite_block_identifiers(&mut cond_stmt.consequent, rename_map);
                if let Some(alternative) = &mut cond_stmt.alternative {
                    self.rewrite_block_identifiers(alternative, rename_map);
                }
            }
            Statement::Loop(loop_stmt) => {
                match &mut loop_stmt.header {
                    LoopHeader::For { iterator, .. } => {
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
            Statement::Break(_) | Statement::Continue(_) => {}
            Statement::Throw(throw_stmt) => {
                self.rewrite_expression_identifiers(&mut throw_stmt.expression, rename_map);
            }
            Statement::Match(match_stmt) => {
                self.rewrite_expression_identifiers(&mut match_stmt.scrutinee, rename_map);
                for arm in &mut match_stmt.arms {
                    for pattern in &mut arm.patterns {
                        if let MatchPattern::Expression(pattern_expr) = pattern {
                            self.rewrite_expression_identifiers(pattern_expr, rename_map);
                        }
                    }
                    self.rewrite_block_identifiers(&mut arm.block, rename_map);
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
            ExpressionKind::InterpolatedString(template) => {
                for part in &mut template.parts {
                    if let InterpolatedStringPart::Expression(expr) = part {
                        self.rewrite_expression_identifiers(expr, rename_map);
                    }
                }
            }
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
            ExpressionKind::Is(is_expr) => {
                self.rewrite_expression_identifiers(&mut is_expr.value, rename_map);
                self.rewrite_type_expression_identifiers(&mut is_expr.type_annotation, rename_map);
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
            ExpressionKind::Conditional(cond) => {
                self.rewrite_expression_identifiers(&mut cond.condition, rename_map);
                self.rewrite_expression_identifiers(&mut cond.consequent, rename_map);
                self.rewrite_expression_identifiers(&mut cond.alternative, rename_map);
            }
            ExpressionKind::Match(match_expr) => {
                self.rewrite_expression_identifiers(&mut match_expr.scrutinee, rename_map);
                for arm in &mut match_expr.arms {
                    for pattern in &mut arm.patterns {
                        match pattern {
                            MatchPattern::Expression(pattern_expr) => {
                                self.rewrite_expression_identifiers(pattern_expr, rename_map);
                            }
                            MatchPattern::Type(type_expr, _) => {
                                self.rewrite_type_expression_identifiers(type_expr, rename_map);
                            }
                            MatchPattern::Wildcard { .. } => {}
                        }
                    }
                    self.rewrite_expression_identifiers(&mut arm.expression, rename_map);
                }
            }
            ExpressionKind::Try(try_expr) => {
                self.rewrite_expression_identifiers(&mut try_expr.expression, rename_map);
                if let Some(clause) = &mut try_expr.catch {
                    match &mut clause.kind {
                        CatchKind::Fallback(expr) => {
                            self.rewrite_expression_identifiers(expr, rename_map);
                        }
                        CatchKind::Arms(arms) => {
                            for arm in arms {
                                for pattern in &mut arm.patterns {
                                    if let MatchPattern::Expression(pattern_expr) = pattern {
                                        self.rewrite_expression_identifiers(
                                            pattern_expr,
                                            rename_map,
                                        );
                                    }
                                }
                                match &mut arm.handler {
                                    CatchHandler::Expression(expr) => {
                                        self.rewrite_expression_identifiers(expr, rename_map);
                                    }
                                    CatchHandler::Block(block) => {
                                        self.rewrite_block_identifiers(block, rename_map);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            ExpressionKind::Unwrap(inner) => {
                self.rewrite_expression_identifiers(inner, rename_map);
            }
            ExpressionKind::Grouping(expr) => {
                self.rewrite_expression_identifiers(expr, rename_map);
            }
        }
    }

    fn rewrite_statement_alias(
        &mut self,
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
            Statement::Union(union_stmt) => {
                for member in &mut union_stmt.members {
                    self.rewrite_type_expression_alias(&mut member.type_expression, alias_maps);
                }
            }
            Statement::Enum(_) => {}
            Statement::Error(_) => {}
            Statement::Conditional(cond_stmt) => {
                self.rewrite_expression_alias(&mut cond_stmt.condition, alias_maps);
                self.rewrite_block_alias(&mut cond_stmt.consequent, alias_maps);
                if let Some(alternative) = &mut cond_stmt.alternative {
                    self.rewrite_block_alias(alternative, alias_maps);
                }
            }
            Statement::Loop(loop_stmt) => {
                match &mut loop_stmt.header {
                    LoopHeader::For { iterator, .. } => {
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
            Statement::Break(_) | Statement::Continue(_) => {}
            Statement::Throw(throw_stmt) => {
                self.rewrite_expression_alias(&mut throw_stmt.expression, alias_maps);
            }
            Statement::Match(match_stmt) => {
                self.rewrite_expression_alias(&mut match_stmt.scrutinee, alias_maps);
                for arm in &mut match_stmt.arms {
                    for pattern in &mut arm.patterns {
                        if let MatchPattern::Expression(pattern_expr) = pattern {
                            self.rewrite_expression_alias(pattern_expr, alias_maps);
                        }
                    }
                    self.rewrite_block_alias(&mut arm.block, alias_maps);
                }
            }
            Statement::Expression(expr_stmt) => {
                self.rewrite_expression_alias(&mut expr_stmt.expression, alias_maps);
            }
        }
    }

    fn rewrite_block_alias(
        &mut self,
        block: &mut Block,
        alias_maps: &HashMap<String, HashMap<String, String>>,
    ) {
        for statement in &mut block.statements {
            self.rewrite_statement_alias(statement, alias_maps);
        }
    }

    fn rewrite_expression_alias(
        &mut self,
        expression: &mut Expression,
        alias_maps: &HashMap<String, HashMap<String, String>>,
    ) {
        match &mut expression.kind {
            ExpressionKind::Member(member) => {
                self.rewrite_expression_alias(&mut member.object, alias_maps);
                if let ExpressionKind::Identifier(identifier) = &member.object.kind {
                    if let Some(map) = alias_maps.get(&identifier.name) {
                        match map.get(&member.property) {
                            Some(replacement) => {
                                expression.kind = ExpressionKind::Identifier(Identifier {
                                    name: replacement.clone(),
                                    span: member.property_span,
                                });
                                return;
                            }
                            None => {
                                self.diagnostics.push_error_with_span(
                                    format!(
                                        "module '{}' has no export named '{}'",
                                        identifier.name, member.property
                                    ),
                                    Some(member.property_span),
                                );
                            }
                        }
                    }
                }
            }
            ExpressionKind::InterpolatedString(template) => {
                for part in &mut template.parts {
                    if let InterpolatedStringPart::Expression(expr) = part {
                        self.rewrite_expression_alias(expr, alias_maps);
                    }
                }
            }
            ExpressionKind::Is(is_expr) => {
                self.rewrite_expression_alias(&mut is_expr.value, alias_maps);
                self.rewrite_type_expression_alias(&mut is_expr.type_annotation, alias_maps);
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
            ExpressionKind::Conditional(cond) => {
                self.rewrite_expression_alias(&mut cond.condition, alias_maps);
                self.rewrite_expression_alias(&mut cond.consequent, alias_maps);
                self.rewrite_expression_alias(&mut cond.alternative, alias_maps);
            }
            ExpressionKind::Match(match_expr) => {
                self.rewrite_expression_alias(&mut match_expr.scrutinee, alias_maps);
                for arm in &mut match_expr.arms {
                    for pattern in &mut arm.patterns {
                        match pattern {
                            MatchPattern::Expression(pattern_expr) => {
                                self.rewrite_expression_alias(pattern_expr, alias_maps);
                            }
                            MatchPattern::Type(type_expr, _) => {
                                self.rewrite_type_expression_alias(type_expr, alias_maps);
                            }
                            MatchPattern::Wildcard { .. } => {}
                        }
                    }
                    self.rewrite_expression_alias(&mut arm.expression, alias_maps);
                }
            }
            ExpressionKind::Try(try_expr) => {
                self.rewrite_expression_alias(&mut try_expr.expression, alias_maps);
                if let Some(clause) = &mut try_expr.catch {
                    match &mut clause.kind {
                        CatchKind::Fallback(expr) => {
                            self.rewrite_expression_alias(expr, alias_maps);
                        }
                        CatchKind::Arms(arms) => {
                            for arm in arms {
                                for pattern in &mut arm.patterns {
                                    if let MatchPattern::Expression(pattern_expr) = pattern {
                                        self.rewrite_expression_alias(pattern_expr, alias_maps);
                                    }
                                }
                                match &mut arm.handler {
                                    CatchHandler::Expression(expr) => {
                                        self.rewrite_expression_alias(expr, alias_maps);
                                    }
                                    CatchHandler::Block(block) => {
                                        self.rewrite_block_alias(block, alias_maps);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            ExpressionKind::Grouping(expr) => {
                self.rewrite_expression_alias(expr, alias_maps);
            }
            ExpressionKind::Identifier(_) | ExpressionKind::Literal(_) => {}
            ExpressionKind::Index(index) => {
                self.rewrite_expression_alias(&mut index.object, alias_maps);
                self.rewrite_expression_alias(&mut index.index, alias_maps);
            }
            ExpressionKind::Unwrap(inner) => {
                self.rewrite_expression_alias(inner, alias_maps);
            }
        }
    }

    fn load_module(&mut self, path: &Path) -> Result<Module> {
        if let Some(contents) = self.module_overrides.get(path).cloned() {
            return self.load_module_from_contents(path, contents);
        }

        let contents = self
            .loader
            .load_module(path)
            .with_context(|| format!("failed to read module at '{}'", path.display()))?;
        self.load_module_from_contents(path, contents)
    }

    fn load_module_from_contents(&mut self, path: &Path, contents: String) -> Result<Module> {
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

fn default_loader(loader: Option<Arc<dyn ModuleLoader>>) -> Result<Arc<dyn ModuleLoader>> {
    if let Some(loader) = loader {
        return Ok(loader);
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        Ok(Arc::new(crate::loader::NativeModuleLoader))
    }

    #[cfg(target_arch = "wasm32")]
    {
        bail!("an explicit module loader is required on wasm32 targets")
    }
}
