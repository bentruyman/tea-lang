use std::collections::HashMap;

use crate::analysis::SemanticAnalysis;
use crate::ast::{
    Block, CallExpression, CatchHandler, ConditionalExpression, Expression, ExpressionKind,
    FunctionStatement, LambdaBody, MatchArm, MatchArmBlock, MemberExpression, Module, Statement,
};
use crate::diagnostics::Diagnostics;
use crate::stdlib;

pub(crate) fn validate_browser_target(module: &Module, analysis: &SemanticAnalysis) -> Diagnostics {
    let mut diagnostics = Diagnostics::new();
    let alias_paths: HashMap<String, String> = analysis
        .module_aliases()
        .iter()
        .map(|(alias, binding)| (alias.clone(), binding.module_path.clone()))
        .collect();

    visit_block(
        &mut diagnostics,
        &alias_paths,
        &Block {
            statements: module.statements.clone(),
        },
    );

    diagnostics
}

fn visit_block(
    diagnostics: &mut Diagnostics,
    alias_paths: &HashMap<String, String>,
    block: &Block,
) {
    for statement in &block.statements {
        visit_statement(diagnostics, alias_paths, statement);
    }
}

fn visit_statement(
    diagnostics: &mut Diagnostics,
    alias_paths: &HashMap<String, String>,
    statement: &Statement,
) {
    match statement {
        Statement::Use(use_stmt) => {
            if use_stmt.module_path.starts_with("std.")
                && !stdlib::is_browser_safe_stdlib_module(&use_stmt.module_path)
            {
                diagnostics.push_error_with_span(
                    format!(
                        "module '{}' is not available in the browser target",
                        use_stmt.module_path
                    ),
                    Some(use_stmt.module_span),
                );
            }
        }
        Statement::Var(var_stmt) => {
            for binding in &var_stmt.bindings {
                if let Some(initializer) = &binding.initializer {
                    visit_expression(diagnostics, alias_paths, initializer);
                }
            }
        }
        Statement::Function(function) => visit_function(diagnostics, alias_paths, function),
        Statement::Test(test_stmt) => visit_block(diagnostics, alias_paths, &test_stmt.body),
        Statement::Struct(_) => {}
        Statement::Union(union_stmt) => diagnostics.push_error_with_span(
            format!(
                "union '{}' is not yet supported by the browser runner",
                union_stmt.name
            ),
            Some(union_stmt.name_span),
        ),
        Statement::Enum(enum_stmt) => diagnostics.push_error_with_span(
            format!(
                "enum '{}' is not yet supported by the browser runner",
                enum_stmt.name
            ),
            Some(enum_stmt.name_span),
        ),
        Statement::Error(error_stmt) => diagnostics.push_error_with_span(
            format!(
                "error type '{}' is not yet supported by the browser runner",
                error_stmt.name
            ),
            Some(error_stmt.name_span),
        ),
        Statement::Conditional(conditional) => {
            visit_expression(diagnostics, alias_paths, &conditional.condition);
            visit_block(diagnostics, alias_paths, &conditional.consequent);
            if let Some(alternative) = &conditional.alternative {
                visit_block(diagnostics, alias_paths, alternative);
            }
        }
        Statement::Loop(loop_stmt) => {
            match &loop_stmt.header {
                crate::ast::LoopHeader::For { iterator, .. } => {
                    visit_expression(diagnostics, alias_paths, iterator);
                }
                crate::ast::LoopHeader::Condition(condition) => {
                    visit_expression(diagnostics, alias_paths, condition);
                }
            }
            visit_block(diagnostics, alias_paths, &loop_stmt.body);
        }
        Statement::Break(_) | Statement::Continue(_) => {}
        Statement::Throw(throw_stmt) => diagnostics.push_error_with_span(
            "throw is not yet supported by the browser runner",
            Some(throw_stmt.span),
        ),
        Statement::Return(return_stmt) => {
            if let Some(expression) = &return_stmt.expression {
                visit_expression(diagnostics, alias_paths, expression);
            }
        }
        Statement::Match(match_stmt) => {
            diagnostics.push_error_with_span(
                "match statements are not yet supported by the browser runner",
                Some(match_stmt.span),
            );
            visit_expression(diagnostics, alias_paths, &match_stmt.scrutinee);
            for arm in &match_stmt.arms {
                visit_match_arm_block(diagnostics, alias_paths, arm);
            }
        }
        Statement::Expression(expression) => {
            visit_expression(diagnostics, alias_paths, &expression.expression);
        }
    }
}

fn visit_function(
    diagnostics: &mut Diagnostics,
    alias_paths: &HashMap<String, String>,
    function: &FunctionStatement,
) {
    for parameter in &function.parameters {
        if let Some(default_value) = &parameter.default_value {
            visit_expression(diagnostics, alias_paths, default_value);
        }
    }
    visit_block(diagnostics, alias_paths, &function.body);
}

fn visit_match_arm_block(
    diagnostics: &mut Diagnostics,
    alias_paths: &HashMap<String, String>,
    arm: &MatchArmBlock,
) {
    for pattern in &arm.patterns {
        if let crate::ast::MatchPattern::Expression(expression) = pattern {
            visit_expression(diagnostics, alias_paths, expression);
        }
    }
    visit_block(diagnostics, alias_paths, &arm.block);
}

fn visit_match_arm(
    diagnostics: &mut Diagnostics,
    alias_paths: &HashMap<String, String>,
    arm: &MatchArm,
) {
    for pattern in &arm.patterns {
        if let crate::ast::MatchPattern::Expression(expression) = pattern {
            visit_expression(diagnostics, alias_paths, expression);
        }
    }
    visit_expression(diagnostics, alias_paths, &arm.expression);
}

fn visit_member_call(
    diagnostics: &mut Diagnostics,
    alias_paths: &HashMap<String, String>,
    member: &MemberExpression,
) {
    if let ExpressionKind::Identifier(identifier) = &member.object.kind {
        if let Some(module_path) = alias_paths.get(&identifier.name) {
            if let Some(kind) =
                stdlib::module_function_kind(module_path.as_str(), member.property.as_str())
            {
                if !stdlib::is_browser_safe_function(kind) {
                    diagnostics.push_error_with_span(
                        format!(
                            "function '{}.{}' is not available in the browser target",
                            module_path, member.property
                        ),
                        Some(member.property_span),
                    );
                }
            }
        }
    }
}

fn visit_call(
    diagnostics: &mut Diagnostics,
    alias_paths: &HashMap<String, String>,
    call: &CallExpression,
) {
    if let ExpressionKind::Identifier(identifier) = &call.callee.kind {
        if let Some(kind) = stdlib::builtin_kind(identifier.name.as_str()) {
            if !stdlib::is_browser_safe_function(kind) {
                diagnostics.push_error_with_span(
                    format!(
                        "built-in '@{}' is not available in the browser target",
                        identifier.name
                    ),
                    Some(identifier.span),
                );
            }
        }
    }

    if let ExpressionKind::Member(member) = &call.callee.kind {
        visit_member_call(diagnostics, alias_paths, member);
    }

    visit_expression(diagnostics, alias_paths, &call.callee);
    for argument in &call.arguments {
        visit_expression(diagnostics, alias_paths, &argument.expression);
    }
}

fn visit_expression(
    diagnostics: &mut Diagnostics,
    alias_paths: &HashMap<String, String>,
    expression: &Expression,
) {
    match &expression.kind {
        ExpressionKind::Identifier(_) | ExpressionKind::Literal(_) => {}
        ExpressionKind::InterpolatedString(interpolated) => {
            for part in &interpolated.parts {
                if let crate::ast::InterpolatedStringPart::Expression(expression) = part {
                    visit_expression(diagnostics, alias_paths, expression);
                }
            }
        }
        ExpressionKind::List(list) => {
            for element in &list.elements {
                visit_expression(diagnostics, alias_paths, element);
            }
        }
        ExpressionKind::Dict(dict) => {
            for entry in &dict.entries {
                visit_expression(diagnostics, alias_paths, &entry.value);
            }
        }
        ExpressionKind::Unary(unary) => {
            visit_expression(diagnostics, alias_paths, &unary.operand);
        }
        ExpressionKind::Binary(binary) => {
            visit_expression(diagnostics, alias_paths, &binary.left);
            visit_expression(diagnostics, alias_paths, &binary.right);
        }
        ExpressionKind::Is(is_expr) => {
            visit_expression(diagnostics, alias_paths, &is_expr.value);
        }
        ExpressionKind::Call(call) => visit_call(diagnostics, alias_paths, call),
        ExpressionKind::Member(member) => {
            visit_expression(diagnostics, alias_paths, &member.object)
        }
        ExpressionKind::Index(index) => {
            visit_expression(diagnostics, alias_paths, &index.object);
            visit_expression(diagnostics, alias_paths, &index.index);
        }
        ExpressionKind::Range(range) => {
            visit_expression(diagnostics, alias_paths, &range.start);
            visit_expression(diagnostics, alias_paths, &range.end);
        }
        ExpressionKind::Lambda(lambda) => match &lambda.body {
            LambdaBody::Expression(expression) => {
                visit_expression(diagnostics, alias_paths, expression)
            }
            LambdaBody::Block(block) => visit_block(diagnostics, alias_paths, block),
        },
        ExpressionKind::Assignment(assignment) => {
            visit_expression(diagnostics, alias_paths, &assignment.target);
            visit_expression(diagnostics, alias_paths, &assignment.value);
        }
        ExpressionKind::Match(match_expr) => {
            diagnostics.push_error_with_span(
                "match expressions are not yet supported by the browser runner",
                Some(expression.span),
            );
            visit_expression(diagnostics, alias_paths, &match_expr.scrutinee);
            for arm in &match_expr.arms {
                visit_match_arm(diagnostics, alias_paths, arm);
            }
        }
        ExpressionKind::Conditional(ConditionalExpression {
            condition,
            consequent,
            alternative,
        }) => {
            visit_expression(diagnostics, alias_paths, condition);
            visit_expression(diagnostics, alias_paths, consequent);
            visit_expression(diagnostics, alias_paths, alternative);
        }
        ExpressionKind::Unwrap(inner) | ExpressionKind::Grouping(inner) => {
            visit_expression(diagnostics, alias_paths, inner);
        }
        ExpressionKind::Try(try_expr) => {
            diagnostics.push_error_with_span(
                "try expressions are not yet supported by the browser runner",
                Some(expression.span),
            );
            visit_expression(diagnostics, alias_paths, &try_expr.expression);
            if let Some(catch_clause) = &try_expr.catch {
                match &catch_clause.kind {
                    crate::ast::CatchKind::Fallback(expression) => {
                        visit_expression(diagnostics, alias_paths, expression);
                    }
                    crate::ast::CatchKind::Arms(arms) => {
                        for arm in arms {
                            for pattern in &arm.patterns {
                                if let crate::ast::MatchPattern::Expression(expression) = pattern {
                                    visit_expression(diagnostics, alias_paths, expression);
                                }
                            }
                            match &arm.handler {
                                CatchHandler::Expression(expression) => {
                                    visit_expression(diagnostics, alias_paths, expression);
                                }
                                CatchHandler::Block(block) => {
                                    visit_block(diagnostics, alias_paths, block);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
