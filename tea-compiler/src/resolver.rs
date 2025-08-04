use std::collections::{HashMap, HashSet};

use crate::ast::{
    AssignmentExpression, BinaryExpression, Block, CallExpression, ConditionalStatement,
    DictLiteral, Expression, ExpressionKind, FunctionParameter, FunctionStatement, Identifier,
    IndexExpression, LambdaBody, LambdaExpression, ListLiteral, LoopHeader, LoopKind,
    LoopStatement, MemberExpression, Module, ReturnStatement, SourceSpan, Statement,
    StructStatement, TestStatement, UnaryExpression, UseStatement, VarStatement,
};
use crate::diagnostics::Diagnostics;
use crate::stdlib;

pub struct Resolver {
    scopes: Vec<HashMap<String, Binding>>,
    builtins: HashSet<String>,
    diagnostics: Diagnostics,
    lambda_stack: Vec<LambdaContext>,
    lambda_captures: HashMap<usize, Vec<String>>,
}

#[derive(Clone, Copy)]
struct Binding {
    kind: BindingKind,
    span: SourceSpan,
    used: bool,
}

#[derive(Clone, Copy)]
enum BindingKind {
    Variable,
    Const,
    Function,
    Parameter,
    Struct,
    Module,
}

impl BindingKind {
    fn describe(self) -> &'static str {
        match self {
            BindingKind::Variable => "variable",
            BindingKind::Const => "const",
            BindingKind::Function => "function",
            BindingKind::Parameter => "parameter",
            BindingKind::Struct => "struct",
            BindingKind::Module => "module alias",
        }
    }
}

struct LambdaContext {
    id: usize,
    scope_index: usize,
}

impl Resolver {
    pub fn new() -> Self {
        Self {
            scopes: vec![HashMap::new()],
            builtins: HashSet::new(),
            diagnostics: Diagnostics::new(),
            lambda_stack: Vec::new(),
            lambda_captures: HashMap::new(),
        }
    }

    pub fn resolve_module(&mut self, module: &Module) {
        self.resolve_statements(&module.statements);
    }

    pub fn into_parts(self) -> (Diagnostics, HashMap<usize, Vec<String>>) {
        (self.diagnostics, self.lambda_captures)
    }

    fn resolve_statements(&mut self, statements: &[Statement]) {
        for statement in statements {
            self.resolve_statement(statement);
        }
    }

    fn resolve_statement(&mut self, statement: &Statement) {
        match statement {
            Statement::Use(use_stmt) => self.resolve_use(use_stmt),
            Statement::Var(var_stmt) => self.resolve_var(var_stmt),
            Statement::Function(function_stmt) => self.resolve_function(function_stmt),
            Statement::Test(test_stmt) => self.resolve_test(test_stmt),
            Statement::Struct(struct_stmt) => self.resolve_struct(struct_stmt),
            Statement::Conditional(cond_stmt) => self.resolve_conditional(cond_stmt),
            Statement::Loop(loop_stmt) => self.resolve_loop(loop_stmt),
            Statement::Return(ret_stmt) => self.resolve_return(ret_stmt),
            Statement::Expression(expr_stmt) => self.resolve_expression(&expr_stmt.expression),
        }
    }

    fn resolve_use(&mut self, use_stmt: &UseStatement) {
        let module_path = use_stmt.module_path.as_str();
        let alias = &use_stmt.alias;
        if stdlib::find_module(module_path).is_some() {
            self.declare_binding(&alias.name, alias.span, BindingKind::Module, true);
            self.mark_binding_used(&alias.name);
        } else if module_path.starts_with("std.") || module_path.starts_with("support.") {
            self.diagnostics.push_error_with_span(
                format!("unknown module '{}'", module_path),
                Some(use_stmt.module_span),
            );
        } else {
            self.declare_binding(&alias.name, alias.span, BindingKind::Module, true);
            self.mark_binding_used(&alias.name);
        }
    }

    fn resolve_var(&mut self, var_stmt: &VarStatement) {
        let binding_kind = if var_stmt.is_const {
            BindingKind::Const
        } else {
            BindingKind::Variable
        };
        for binding in &var_stmt.bindings {
            self.declare_binding(&binding.name, binding.span, binding_kind, true);
        }

        for binding in &var_stmt.bindings {
            if let Some(expr) = &binding.initializer {
                self.resolve_expression(expr);
            }
        }
    }

    fn resolve_function(&mut self, function_stmt: &FunctionStatement) {
        self.declare_binding(
            &function_stmt.name,
            function_stmt.name_span,
            BindingKind::Function,
            true,
        );

        self.push_scope();
        for parameter in &function_stmt.parameters {
            self.resolve_parameter(parameter);
        }
        self.resolve_statements(&function_stmt.body.statements);
        self.pop_scope();
    }

    fn resolve_test(&mut self, test_stmt: &TestStatement) {
        self.push_scope();
        self.resolve_statements(&test_stmt.body.statements);
        self.pop_scope();
    }

    fn resolve_struct(&mut self, struct_stmt: &StructStatement) {
        self.declare_binding(
            &struct_stmt.name,
            struct_stmt.name_span,
            BindingKind::Struct,
            true,
        );
    }

    fn resolve_conditional(&mut self, cond_stmt: &ConditionalStatement) {
        self.resolve_expression(&cond_stmt.condition);
        self.resolve_block(&cond_stmt.consequent);
        if let Some(alternative) = &cond_stmt.alternative {
            self.resolve_block(alternative);
        }
    }

    fn resolve_loop(&mut self, loop_stmt: &LoopStatement) {
        match loop_stmt.kind {
            LoopKind::For => {
                self.diagnostics.push_error_with_span(
                    "for loops are not supported by the resolver yet",
                    Some(loop_stmt.span),
                );
            }
            LoopKind::While | LoopKind::Until => {
                if let LoopHeader::Condition(condition) = &loop_stmt.header {
                    self.resolve_expression(condition);
                }
            }
        }
        self.resolve_block(&loop_stmt.body);
    }

    fn resolve_return(&mut self, return_stmt: &ReturnStatement) {
        if let Some(expr) = &return_stmt.expression {
            self.resolve_expression(expr);
        }
    }

    fn resolve_block(&mut self, block: &Block) {
        self.push_scope();
        self.resolve_statements(&block.statements);
        self.pop_scope();
    }

    fn resolve_parameter(&mut self, parameter: &FunctionParameter) {
        self.declare_binding(
            &parameter.name,
            parameter.span,
            BindingKind::Parameter,
            true,
        );
        if let Some(default_value) = &parameter.default_value {
            self.resolve_expression(default_value);
        }
    }

    fn resolve_expression(&mut self, expression: &Expression) {
        match &expression.kind {
            ExpressionKind::Identifier(identifier) => self.resolve_identifier(identifier),
            ExpressionKind::Literal(_) => {}
            ExpressionKind::List(list) => self.resolve_list(list),
            ExpressionKind::Dict(dict) => self.resolve_dict(dict),
            ExpressionKind::Unary(unary) => self.resolve_unary(unary),
            ExpressionKind::Binary(binary) => self.resolve_binary(binary),
            ExpressionKind::Call(call) => self.resolve_call(call),
            ExpressionKind::Member(member) => self.resolve_member(member),
            ExpressionKind::Index(index) => self.resolve_index(index),
            ExpressionKind::Range(range) => {
                self.resolve_expression(&range.start);
                self.resolve_expression(&range.end);
            }
            ExpressionKind::Lambda(lambda) => self.resolve_lambda(lambda),
            ExpressionKind::Assignment(assignment) => self.resolve_assignment(assignment),
            ExpressionKind::Grouping(inner) => self.resolve_expression(inner),
        }
    }

    fn resolve_identifier(&mut self, identifier: &Identifier) {
        if self.mark_binding_used(&identifier.name).is_some()
            || self.builtins.contains(&identifier.name)
        {
            return;
        }
        let message = if let Some(module_path) = stdlib::module_for_function(&identifier.name) {
            let suggested_alias = module_path
                .rsplit('.')
                .next()
                .unwrap_or(module_path)
                .to_string();
            format!(
                "use of undefined binding '{}'; add `use {} = \"{}\"` to import it",
                identifier.name, suggested_alias, module_path
            )
        } else {
            format!("use of undefined binding '{}'", identifier.name)
        };
        self.diagnostics
            .push_with_location(message, identifier.span.line, identifier.span.column);
    }

    fn resolve_list(&mut self, list: &ListLiteral) {
        for element in &list.elements {
            self.resolve_expression(element);
        }
    }

    fn resolve_dict(&mut self, dict: &DictLiteral) {
        for entry in &dict.entries {
            self.resolve_expression(&entry.value);
        }
    }

    fn resolve_unary(&mut self, unary: &UnaryExpression) {
        self.resolve_expression(&unary.operand);
    }

    fn resolve_binary(&mut self, binary: &BinaryExpression) {
        self.resolve_expression(&binary.left);
        self.resolve_expression(&binary.right);
    }

    fn resolve_call(&mut self, call: &CallExpression) {
        self.resolve_expression(&call.callee);
        for argument in &call.arguments {
            self.resolve_expression(&argument.expression);
        }
    }

    fn resolve_member(&mut self, member: &MemberExpression) {
        self.resolve_expression(&member.object);
    }

    fn resolve_index(&mut self, index: &IndexExpression) {
        self.resolve_expression(&index.object);
        self.resolve_expression(&index.index);
    }

    fn resolve_lambda(&mut self, lambda: &LambdaExpression) {
        self.push_scope();
        let scope_index = self.scopes.len() - 1;
        self.lambda_stack.push(LambdaContext {
            id: lambda.id,
            scope_index,
        });
        for parameter in &lambda.parameters {
            self.resolve_parameter(parameter);
        }
        match &lambda.body {
            LambdaBody::Expression(expr) => self.resolve_expression(expr),
            LambdaBody::Block(block) => self.resolve_statements(&block.statements),
        }
        self.lambda_stack.pop();
        self.pop_scope();
    }

    fn resolve_assignment(&mut self, assignment: &AssignmentExpression) {
        self.resolve_expression(&assignment.target);
        if let ExpressionKind::Identifier(identifier) = &assignment.target.kind {
            if let Some(kind) = self.mark_binding_used(&identifier.name) {
                if matches!(kind, BindingKind::Const) {
                    self.diagnostics.push_with_location(
                        format!("cannot reassign const '{}'", identifier.name),
                        identifier.span.line,
                        identifier.span.column,
                    );
                }
            }
        }
        self.resolve_expression(&assignment.value);
    }

    fn declare_binding(
        &mut self,
        name: &str,
        span: SourceSpan,
        kind: BindingKind,
        check_shadow: bool,
    ) {
        if let Some(existing) = self
            .scopes
            .last()
            .and_then(|scope| scope.get(name).copied())
        {
            let new_kind_desc = kind.describe();
            let existing_kind_desc = existing.kind.describe();
            self.diagnostics.push_with_location(
                format!(
                    "duplicate declaration of {new_kind_desc} '{}' (first declared as {existing_kind_desc} at line {}, column {})",
                    name, existing.span.line, existing.span.column
                ),
                span.line,
                span.column,
            );
            return;
        }

        if check_shadow {
            if let Some(existing) = self.find_in_outer_scopes(name) {
                let existing_kind_desc = existing.kind.describe();
                let new_kind_desc = kind.describe();
                self.diagnostics.push_with_location(
                    format!(
                        "redeclaration of {new_kind_desc} '{}' shadows existing {existing_kind_desc} declared at line {}, column {}",
                        name, existing.span.line, existing.span.column
                    ),
                    span.line,
                    span.column,
                );
            }
        }

        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(
                name.to_string(),
                Binding {
                    kind,
                    span,
                    used: false,
                },
            );
        }
    }

    fn mark_binding_used(&mut self, name: &str) -> Option<BindingKind> {
        let scope_len = self.scopes.len();
        let mut found = None;
        let mut capture_scope_index = None;

        for (depth, scope) in self.scopes.iter_mut().rev().enumerate() {
            if let Some(binding) = scope.get_mut(name) {
                binding.used = true;
                found = Some(binding.kind);
                capture_scope_index = Some(scope_len.saturating_sub(depth + 1));
                break;
            }
        }

        if let Some(scope_index) = capture_scope_index {
            self.note_capture(name, scope_index);
        }

        found
    }

    fn find_in_outer_scopes(&self, name: &str) -> Option<Binding> {
        if self.scopes.len() <= 1 {
            return None;
        }

        self.scopes
            .iter()
            .rev()
            .skip(1)
            .find_map(|scope| scope.get(name).copied())
    }

    fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn pop_scope(&mut self) {
        if self.scopes.len() <= 1 {
            return;
        }
        if let Some(scope) = self.scopes.pop() {
            for (name, binding) in scope {
                if !binding.used {
                    let message = match binding.kind {
                        BindingKind::Variable => format!("unused variable '{}'", name),
                        BindingKind::Const => format!("unused const '{}'", name),
                        BindingKind::Parameter => format!("unused parameter '{}'", name),
                        BindingKind::Function | BindingKind::Struct => continue,
                        BindingKind::Module => {
                            format!("unused module alias '{}'", name)
                        }
                    };
                    self.diagnostics.push_with_location(
                        message,
                        binding.span.line,
                        binding.span.column,
                    );
                }
            }
        }
    }

    fn note_capture(&mut self, name: &str, scope_index: usize) {
        for ctx in self.lambda_stack.iter().rev() {
            if scope_index < ctx.scope_index {
                if scope_index > 0 {
                    let captures = self.lambda_captures.entry(ctx.id).or_default();
                    if !captures.iter().any(|existing| existing == name) {
                        captures.push(name.to_string());
                    }
                }
            } else {
                break;
            }
        }
    }
}
