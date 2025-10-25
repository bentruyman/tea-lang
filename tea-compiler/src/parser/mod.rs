use anyhow::{bail, Result};

use crate::ast::*;
use crate::diagnostics::Diagnostics;
use crate::lexer::{Keyword, Token, TokenKind};
use crate::source::SourceFile;

#[derive(Copy, Clone, PartialEq, PartialOrd)]
enum Precedence {
    Lowest = 0,
    Assignment,
    Or,
    And,
    Equality,
    Comparison,
    Range,
    Term,
    Factor,
    Unary,
}

impl Precedence {
    fn of(kind: &TokenKind) -> Option<Self> {
        match kind {
            TokenKind::Equal => Some(Precedence::Assignment),
            TokenKind::Keyword(Keyword::Or) => Some(Precedence::Or),
            TokenKind::Keyword(Keyword::And) => Some(Precedence::And),
            TokenKind::DoubleEqual | TokenKind::BangEqual => Some(Precedence::Equality),
            TokenKind::Greater
            | TokenKind::GreaterEqual
            | TokenKind::Less
            | TokenKind::LessEqual => Some(Precedence::Comparison),
            TokenKind::DotDot | TokenKind::DotDotDot => Some(Precedence::Range),
            TokenKind::Plus | TokenKind::Minus => Some(Precedence::Term),
            TokenKind::Star | TokenKind::Slash | TokenKind::Percent => Some(Precedence::Factor),
            _ => None,
        }
    }
}

pub struct Parser<'a> {
    _source: &'a SourceFile,
    tokens: Vec<Token>,
    current: usize,
    diagnostics: Diagnostics,
    next_lambda_id: usize,
}

impl<'a> Parser<'a> {
    pub fn new(source: &'a SourceFile, tokens: Vec<Token>) -> Self {
        Self {
            _source: source,
            tokens,
            current: 0,
            diagnostics: Diagnostics::new(),
            next_lambda_id: 0,
        }
    }

    fn span_from_token(token: &Token) -> SourceSpan {
        let len = token.lexeme.chars().count().max(1);
        SourceSpan::new(
            token.line,
            token.column,
            token.line,
            token.column + len.saturating_sub(1),
        )
    }

    fn make_expression(span: SourceSpan, kind: ExpressionKind) -> Expression {
        Expression { span, kind }
    }

    fn union_spans(a: &SourceSpan, b: &SourceSpan) -> SourceSpan {
        SourceSpan::union(a, b)
    }

    pub fn parse(&mut self) -> Result<Module> {
        let mut statements = Vec::new();

        while !self.is_at_end() {
            self.skip_newlines();
            if self.is_at_end() {
                break;
            }
            let statement = self.parse_statement()?;
            statements.push(statement);
        }

        Ok(Module::new(statements))
    }

    pub fn into_diagnostics(self) -> Diagnostics {
        self.diagnostics
    }

    fn allocate_lambda_id(&mut self) -> usize {
        let id = self.next_lambda_id;
        self.next_lambda_id += 1;
        id
    }

    fn parse_statement(&mut self) -> Result<Statement> {
        let docstring = self.consume_doc_comments();
        match self.peek_kind() {
            TokenKind::Keyword(Keyword::Use) => self.parse_use(),
            TokenKind::Keyword(Keyword::Var) => self.parse_binding(Keyword::Var, docstring.clone()),
            TokenKind::Keyword(Keyword::Const) => {
                self.parse_binding(Keyword::Const, docstring.clone())
            }
            TokenKind::Keyword(Keyword::Def) => self.parse_function(docstring.clone(), false),
            TokenKind::Keyword(Keyword::Pub) => self.parse_public_statement(docstring.clone()),
            TokenKind::Keyword(Keyword::Test) => self.parse_test(docstring.clone()),
            TokenKind::Keyword(Keyword::Struct) => self.parse_struct(docstring),
            TokenKind::Keyword(Keyword::If) => self.parse_conditional(ConditionalKind::If),
            TokenKind::Keyword(Keyword::Unless) => self.parse_conditional(ConditionalKind::Unless),
            TokenKind::Keyword(Keyword::For) => self.parse_for_loop(),
            TokenKind::Keyword(Keyword::While) => self.parse_loop(LoopKind::While),
            TokenKind::Keyword(Keyword::Until) => self.parse_loop(LoopKind::Until),
            TokenKind::Keyword(Keyword::Return) => self.parse_return(),
            _ => self.parse_expression_statement(),
        }
    }

    fn parse_public_statement(&mut self, docstring: Option<String>) -> Result<Statement> {
        let pub_token = self.peek().clone();
        self.advance(); // consume 'pub'
        self.skip_newlines();

        match self.peek_kind() {
            TokenKind::Keyword(Keyword::Def) => self.parse_function(docstring, true),
            other => {
                let span = Self::span_from_token(&pub_token);
                self.diagnostics.push_error_with_span(
                    format!(
                        "unexpected {:?} after 'pub', only functions can be declared public",
                        other
                    ),
                    Some(span),
                );
                bail!("invalid public declaration");
            }
        }
    }

    fn parse_use(&mut self) -> Result<Statement> {
        self.advance(); // consume 'use'
        self.skip_newlines();

        let alias_token = self.peek().clone();
        let alias = match alias_token.kind {
            TokenKind::Identifier => {
                self.advance();
                let alias_span = Self::span_from_token(&alias_token);
                let alias_name = alias_token.lexeme;
                if !matches!(self.peek_kind(), TokenKind::Equal) {
                    self.diagnostics.push_with_location(
                        "expected '=' after module alias",
                        alias_token.line,
                        alias_token.column,
                    );
                    bail!(
                        "expected '=' after module alias at line {}, column {}",
                        alias_token.line,
                        alias_token.column
                    );
                }
                self.advance(); // consume '='
                self.skip_newlines();
                UseAlias {
                    name: alias_name,
                    span: alias_span,
                }
            }
            _ => {
                self.diagnostics.push_with_location(
                    "module imports must specify an alias (e.g. `use fs = \"std.fs\"`)",
                    alias_token.line,
                    alias_token.column,
                );
                bail!(
                    "module imports must specify an alias (e.g. `use fs = \"std.fs\"`) at line {}, column {}",
                    alias_token.line,
                    alias_token.column
                );
            }
        };

        let module_token = self.peek().clone();
        let module_span = Self::span_from_token(&module_token);
        let module_path = match &module_token.kind {
            TokenKind::StringLiteral(value) => {
                self.advance();
                value.clone()
            }
            other => {
                let span = Self::span_from_token(&module_token);
                self.diagnostics.push_error_with_span(
                    format!("expected module path string after 'use', found {:?}", other),
                    Some(span),
                );
                bail!("unexpected token after 'use'");
            }
        };

        if matches!(self.peek_kind(), TokenKind::Semicolon) {
            self.advance();
        }
        self.expect_newline("expected newline after use statement")?;

        Ok(Statement::Use(UseStatement {
            alias,
            module_path,
            module_span,
        }))
    }

    fn parse_binding(&mut self, keyword: Keyword, docstring: Option<String>) -> Result<Statement> {
        let is_const = matches!(keyword, Keyword::Const);
        self.advance(); // consume keyword
        let mut bindings = Vec::new();
        let keyword_lexeme = match keyword {
            Keyword::Var => "var",
            Keyword::Const => "const",
            _ => unreachable!("parse_binding only handles var/const"),
        };

        loop {
            let name_token = self.peek().clone();
            let span = Self::span_from_token(&name_token);
            let name = match &name_token.kind {
                TokenKind::Identifier => {
                    self.advance();
                    name_token.lexeme
                }
                _ => {
                    let span = Self::span_from_token(&name_token);
                    self.diagnostics.push_error_with_span(
                        format!("expected identifier after '{}'", keyword_lexeme),
                        Some(span),
                    );
                    bail!("invalid binding name");
                }
            };

            let type_annotation = if matches!(self.peek_kind(), TokenKind::Colon) {
                let colon_token = self.peek().clone();
                self.advance(); // consume ':'
                let tokens = self.collect_type_tokens();
                if tokens.is_empty() {
                    let colon_span = Self::span_from_token(&colon_token);
                    self.diagnostics.push_error_with_span(
                        "expected type annotation after ':'",
                        Some(colon_span),
                    );
                    bail!("missing type annotation");
                }
                Some(TypeExpression { tokens })
            } else {
                None
            };

            let initializer = if matches!(self.peek_kind(), TokenKind::Equal) {
                self.advance(); // consume '='
                let expression = self.parse_expression_with(terminator_default_or_comma)?;
                Some(expression)
            } else {
                None
            };

            bindings.push(VarBinding {
                name,
                span,
                type_annotation,
                initializer,
            });

            if matches!(self.peek_kind(), TokenKind::Comma) {
                self.advance();
                self.skip_newlines();
                continue;
            }

            break;
        }

        self.expect_newline("expected newline after variable declaration")?;

        Ok(Statement::Var(VarStatement {
            is_const,
            bindings,
            docstring,
        }))
    }

    fn parse_function(&mut self, docstring: Option<String>, is_public: bool) -> Result<Statement> {
        self.advance(); // consume 'def'
        let name_token = self.peek().clone();
        let name_span = Self::span_from_token(&name_token);
        let name = match &name_token.kind {
            TokenKind::Identifier => {
                self.advance();
                name_token.lexeme
            }
            _ => bail!(
                "expected function name after 'def' at line {}, column {}",
                name_token.line,
                name_token.column
            ),
        };

        let type_parameters = if matches!(self.peek_kind(), TokenKind::LBracket) {
            self.advance(); // consume '['
            let params = self.parse_type_parameters("function", &name, name_span.line)?;
            params
        } else {
            Vec::new()
        };

        let lparen_token = self.peek().clone();
        if !matches!(lparen_token.kind, TokenKind::LParen) {
            let span = Self::span_from_token(&lparen_token);
            self.diagnostics
                .push_error_with_span("expected '(' after function name", Some(span));
            bail!("missing function parameter list");
        }
        self.advance(); // consume '('
        let parameters = self.parse_parameters()?;

        let return_type = if matches!(self.peek_kind(), TokenKind::Arrow) {
            self.advance();
            let tokens = self.collect_type_tokens();
            Some(TypeExpression { tokens })
        } else {
            None
        };

        self.expect_newline("expected newline after function signature")?;

        let body = self.parse_block_until(&[Keyword::End])?;
        self.expect_keyword(Keyword::End, "expected 'end' to close function")?;
        self.expect_newline("expected newline after function end")?;

        Ok(Statement::Function(FunctionStatement {
            is_public,
            name,
            name_span,
            type_parameters,
            parameters,
            return_type,
            body,
            docstring,
        }))
    }

    fn parse_test(&mut self, docstring: Option<String>) -> Result<Statement> {
        self.advance(); // consume 'test'
        let name_token = self.peek().clone();
        let name_span = Self::span_from_token(&name_token);
        let name = match &name_token.kind {
            TokenKind::StringLiteral(value) => {
                self.advance();
                value.clone()
            }
            _ => bail!(
                "expected string literal after 'test' at line {}, column {}",
                name_token.line,
                name_token.column
            ),
        };

        self.expect_newline("expected newline after test name")?;
        let body = self.parse_block_until(&[Keyword::End])?;
        self.expect_keyword(Keyword::End, "expected 'end' to close test")?;
        self.expect_newline("expected newline after test end")?;

        Ok(Statement::Test(TestStatement {
            name,
            name_span,
            body,
            docstring,
        }))
    }

    fn parse_parameters(&mut self) -> Result<Vec<FunctionParameter>> {
        let mut parameters = Vec::new();

        if matches!(self.peek_kind(), TokenKind::RParen) {
            self.advance();
            return Ok(parameters);
        }

        loop {
            let name_token = self.peek().clone();
            let span = Self::span_from_token(&name_token);
            let name = match &name_token.kind {
                TokenKind::Identifier => {
                    self.advance();
                    name_token.lexeme
                }
                _ => bail!(
                    "expected parameter name at line {}, column {}",
                    name_token.line,
                    name_token.column
                ),
            };

            let type_annotation = if matches!(self.peek_kind(), TokenKind::Colon) {
                self.advance();
                let tokens = self.collect_type_tokens();
                Some(TypeExpression { tokens })
            } else {
                None
            };

            let default_value = if matches!(self.peek_kind(), TokenKind::Equal) {
                self.advance();
                let expression = self.parse_expression_with(terminator_comma_or_rparen)?;
                Some(expression)
            } else {
                None
            };

            parameters.push(FunctionParameter {
                name,
                span,
                type_annotation,
                default_value,
            });

            match self.peek_kind() {
                TokenKind::Comma => {
                    self.advance();
                }
                TokenKind::RParen => {
                    self.advance();
                    break;
                }
                other => {
                    let fallback_span = span;
                    let span = if self.is_at_end() {
                        fallback_span
                    } else {
                        Self::span_from_token(&self.peek().clone())
                    };
                    self.diagnostics.push_error_with_span(
                        format!("expected ',' or ')' in parameter list, found {:?}", other),
                        Some(span),
                    );
                    bail!("invalid parameter separator");
                }
            }
        }

        Ok(parameters)
    }

    fn parse_type_parameters(
        &mut self,
        owner_kind: &str,
        owner_name: &str,
        owner_name_line: usize,
    ) -> Result<Vec<TypeParameter>> {
        let mut params = Vec::new();
        self.skip_newlines();
        if matches!(self.peek_kind(), TokenKind::RBracket) {
            bail!(
                "expected type parameter name in {} '{}' after '['",
                owner_kind,
                owner_name
            );
        }

        loop {
            self.skip_newlines();
            let token = self.peek().clone();
            let span = Self::span_from_token(&token);
            let name = match token.kind {
                TokenKind::Identifier => {
                    self.advance();
                    token.lexeme
                }
                _ => {
                    let span = Self::span_from_token(&token);
                    self.diagnostics.push_error_with_span(
                        format!(
                            "expected type parameter name in {} '{}'",
                            owner_kind, owner_name
                        ),
                        Some(span),
                    );
                    bail!("invalid type parameter name");
                }
            };

            if params
                .iter()
                .any(|param: &TypeParameter| param.name == name)
            {
                self.diagnostics.push_error_with_span(
                    format!(
                        "duplicate type parameter '{}' in {} '{}'",
                        name, owner_kind, owner_name
                    ),
                    Some(span),
                );
                bail!("duplicate type parameter");
            }

            params.push(TypeParameter { name, span });
            self.skip_newlines();

            match self.peek_kind() {
                TokenKind::Comma => {
                    self.advance();
                }
                TokenKind::RBracket => {
                    let closing_token = self.advance().clone();
                    if closing_token.line != owner_name_line {
                        let closing_span = Self::span_from_token(&closing_token);
                        self.diagnostics.push_error_with_span(
                            format!(
                                "newline before closing ']' in {} '{}' type parameters; closing bracket must be on the same line as the name",
                                owner_kind, owner_name
                            ),
                            Some(closing_span),
                        );
                        bail!("type parameter list closed on a new line");
                    }
                    break;
                }
                other => {
                    let span = if self.is_at_end() {
                        span
                    } else {
                        Self::span_from_token(&self.peek().clone())
                    };
                    self.diagnostics.push_error_with_span(
                        format!(
                            "expected ',' or ']' in {} '{}' type parameter list, found {:?}",
                            owner_kind, owner_name, other
                        ),
                        Some(span),
                    );
                    bail!("invalid type parameter separator");
                }
            }
        }

        Ok(params)
    }

    fn parse_struct(&mut self, docstring: Option<String>) -> Result<Statement> {
        self.advance(); // consume 'struct'
        let name_token = self.peek().clone();
        let name_span = Self::span_from_token(&name_token);
        let name = match &name_token.kind {
            TokenKind::Identifier => {
                self.advance();
                name_token.lexeme
            }
            _ => bail!(
                "expected struct name at line {}, column {}",
                name_token.line,
                name_token.column
            ),
        };

        let type_parameters = if matches!(self.peek_kind(), TokenKind::LBracket) {
            self.advance(); // consume '['
            self.parse_type_parameters("struct", &name, name_span.line)?
        } else {
            Vec::new()
        };

        self.expect_newline("expected newline after struct name")?;

        let mut fields = Vec::new();
        loop {
            self.skip_newlines();
            if self.check_keyword(Keyword::End) {
                self.advance();
                break;
            }
            if self.is_at_end() {
                self.diagnostics.push_error_with_span(
                    format!("unterminated struct '{}', missing 'end'", name),
                    Some(name_span),
                );
                bail!("unterminated struct");
            }

            let field_name_token = self.peek().clone();
            let field_span = Self::span_from_token(&field_name_token);
            let field_name = match &field_name_token.kind {
                TokenKind::Identifier => {
                    self.advance();
                    field_name_token.lexeme
                }
                _ => bail!(
                    "expected field name in struct '{}' at line {}, column {}",
                    name,
                    field_name_token.line,
                    field_name_token.column
                ),
            };

            self.expect_token(TokenKind::Colon, "expected ':' after struct field name")?;
            let type_tokens = self.collect_type_tokens();
            if type_tokens.is_empty() {
                self.diagnostics.push_error_with_span(
                    format!("missing type annotation for field '{}'", field_name),
                    Some(field_span),
                );
                bail!("missing field type");
            }
            fields.push(StructField {
                name: field_name,
                span: field_span,
                type_annotation: TypeExpression {
                    tokens: type_tokens,
                },
            });

            self.expect_newline("expected newline after struct field")?;
        }

        self.expect_newline("expected newline after struct declaration")?;

        Ok(Statement::Struct(StructStatement {
            name,
            name_span,
            type_parameters,
            fields,
            docstring,
        }))
    }

    fn consume_doc_comments(&mut self) -> Option<String> {
        let mut parts = Vec::new();
        let mut consumed_any = false;
        loop {
            match self.peek_kind() {
                TokenKind::DocComment(_) => {
                    if let TokenKind::DocComment(text) = self.advance().kind.clone() {
                        parts.push(text);
                        consumed_any = true;
                    }
                }
                TokenKind::Newline if consumed_any => {
                    self.advance();
                }
                _ => break,
            }
        }

        if consumed_any {
            self.skip_newlines();
        }

        if parts.is_empty() {
            None
        } else {
            Some(parts.join("\n"))
        }
    }

    fn parse_conditional(&mut self, kind: ConditionalKind) -> Result<Statement> {
        self.advance(); // consume keyword
        let condition = self.parse_expression_with(default_expression_terminator)?;

        self.expect_newline("expected newline after conditional header")?;
        let consequent = self.parse_block_until(&[Keyword::Else, Keyword::End])?;

        let alternative = if self.check_keyword(Keyword::Else) {
            self.advance(); // consume 'else'
            self.expect_newline("expected newline after else")?;
            let block = self.parse_block_until(&[Keyword::End])?;
            Some(block)
        } else {
            None
        };

        self.expect_keyword(Keyword::End, "expected 'end' to close conditional")?;
        self.expect_newline("expected newline after conditional end")?;

        Ok(Statement::Conditional(ConditionalStatement {
            kind,
            condition,
            consequent,
            alternative,
        }))
    }

    fn parse_for_loop(&mut self) -> Result<Statement> {
        let for_token = self.peek().clone();
        let span = Self::span_from_token(&for_token);
        self.advance(); // consume 'for'

        let pattern = self.parse_expression_with(terminator_keyword_of)?;

        self.expect_keyword(Keyword::Of, "expected 'of' in for loop")?;
        let iterator = self.parse_expression_with(default_expression_terminator)?;

        self.expect_newline("expected newline after for loop header")?;

        let body = self.parse_block_until(&[Keyword::End])?;
        self.expect_keyword(Keyword::End, "expected 'end' to close for loop")?;
        self.expect_newline("expected newline after for loop end")?;

        Ok(Statement::Loop(LoopStatement {
            kind: LoopKind::For,
            header: LoopHeader::For { pattern, iterator },
            body,
            span,
        }))
    }

    fn parse_loop(&mut self, kind: LoopKind) -> Result<Statement> {
        let loop_token = self.peek().clone();
        let span = Self::span_from_token(&loop_token);
        self.advance(); // consume keyword
        let condition = self.parse_expression_with(default_expression_terminator)?;
        self.expect_newline("expected newline after loop header")?;
        let body = self.parse_block_until(&[Keyword::End])?;
        self.expect_keyword(Keyword::End, "expected 'end' to close loop")?;
        self.expect_newline("expected newline after loop end")?;

        Ok(Statement::Loop(LoopStatement {
            kind,
            header: LoopHeader::Condition(condition),
            body,
            span,
        }))
    }

    fn parse_return(&mut self) -> Result<Statement> {
        let return_token = self.peek().clone();
        let span = Self::span_from_token(&return_token);
        self.advance(); // consume 'return'
        if default_expression_terminator(self.peek_kind()) {
            self.expect_newline("expected newline after return")?;
            return Ok(Statement::Return(ReturnStatement {
                span,
                expression: None,
            }));
        }

        let expression = self.parse_expression_with(default_expression_terminator)?;
        self.expect_newline("expected newline after return statement")?;

        Ok(Statement::Return(ReturnStatement {
            span,
            expression: Some(expression),
        }))
    }

    fn parse_expression_statement(&mut self) -> Result<Statement> {
        let expression = self.parse_expression_with(default_expression_terminator)?;
        if let Err(err) = self.expect_newline("expected newline after expression") {
            let span = expression.span;
            self.diagnostics
                .push_error_with_span(err.to_string(), Some(span));
            return Err(err);
        }
        Ok(Statement::Expression(ExpressionStatement { expression }))
    }

    fn parse_block_until(&mut self, terminators: &[Keyword]) -> Result<Block> {
        let mut statements = Vec::new();

        loop {
            self.skip_newlines();
            if self
                .peek_keyword()
                .map(|kw| terminators.contains(&kw))
                .unwrap_or(false)
            {
                break;
            }
            if self.is_at_end() {
                break;
            }
            statements.push(self.parse_statement()?);
        }

        Ok(Block { statements })
    }

    fn parse_expression_with(&mut self, terminator: fn(&TokenKind) -> bool) -> Result<Expression> {
        self.skip_newlines();
        self.parse_expression_prec(Precedence::Lowest, terminator)
    }

    fn parse_expression_prec(
        &mut self,
        precedence: Precedence,
        terminator: fn(&TokenKind) -> bool,
    ) -> Result<Expression> {
        self.skip_newlines();
        let mut expr = self.parse_prefix_expression(terminator)?;

        loop {
            if self.is_at_end() || terminator(self.peek_kind()) {
                break;
            }

            match self.peek_kind() {
                TokenKind::LParen => {
                    expr = self.finish_call(expr, Vec::new())?;
                    continue;
                }
                TokenKind::LBracket => {
                    if let Some(call_expr) = self.try_finish_generic_call(&expr)? {
                        expr = call_expr;
                        continue;
                    }
                    expr = self.finish_index(expr)?;
                    continue;
                }
                TokenKind::Dot => {
                    expr = self.finish_member(expr)?;
                    continue;
                }
                TokenKind::Newline => {
                    // Newlines act as terminators unless explicitly allowed by the caller.
                    if terminator(&TokenKind::Newline) {
                        break;
                    } else {
                        self.advance();
                        continue;
                    }
                }
                _ => {}
            }

            let next_precedence = match Precedence::of(self.peek_kind()) {
                Some(p) => p,
                None => break,
            };

            if precedence >= next_precedence {
                break;
            }

            expr = self.parse_infix_expression(expr, next_precedence, terminator)?;
        }

        Ok(expr)
    }

    fn parse_prefix_expression(
        &mut self,
        terminator: fn(&TokenKind) -> bool,
    ) -> Result<Expression> {
        let token = self.advance().clone();
        let token_span = Self::span_from_token(&token);
        match token.kind {
            TokenKind::Identifier => Ok(Self::make_expression(
                token_span,
                ExpressionKind::Identifier(Identifier {
                    name: token.lexeme,
                    span: token_span,
                }),
            )),
            TokenKind::IntegerLiteral(value) => Ok(Self::make_expression(
                token_span,
                ExpressionKind::Literal(Literal::Integer(value)),
            )),
            TokenKind::FloatLiteral(value) => Ok(Self::make_expression(
                token_span,
                ExpressionKind::Literal(Literal::Float(value)),
            )),
            TokenKind::StringLiteral(ref string) => Ok(Self::make_expression(
                token_span,
                ExpressionKind::Literal(Literal::String(string.clone())),
            )),
            TokenKind::InterpolatedStringStart => self.parse_interpolated_string(token_span),
            TokenKind::BooleanLiteral(value) => Ok(Self::make_expression(
                token_span,
                ExpressionKind::Literal(Literal::Boolean(value)),
            )),
            TokenKind::Keyword(Keyword::Nil) => Ok(Self::make_expression(
                token_span,
                ExpressionKind::Literal(Literal::Nil),
            )),
            TokenKind::Minus => {
                let operand = self.parse_expression_prec(Precedence::Unary, terminator)?;
                let span = Self::union_spans(&token_span, &operand.span);
                Ok(Self::make_expression(
                    span,
                    ExpressionKind::Unary(UnaryExpression {
                        operator: UnaryOperator::Negative,
                        operand: Box::new(operand),
                    }),
                ))
            }
            TokenKind::Plus => {
                let operand = self.parse_expression_prec(Precedence::Unary, terminator)?;
                let span = Self::union_spans(&token_span, &operand.span);
                Ok(Self::make_expression(
                    span,
                    ExpressionKind::Unary(UnaryExpression {
                        operator: UnaryOperator::Positive,
                        operand: Box::new(operand),
                    }),
                ))
            }
            TokenKind::Bang | TokenKind::Keyword(Keyword::Not) => {
                let operand = self.parse_expression_prec(Precedence::Unary, terminator)?;
                let span = Self::union_spans(&token_span, &operand.span);
                Ok(Self::make_expression(
                    span,
                    ExpressionKind::Unary(UnaryExpression {
                        operator: UnaryOperator::Not,
                        operand: Box::new(operand),
                    }),
                ))
            }
            TokenKind::LParen => {
                let expr = self.parse_expression_prec(Precedence::Lowest, terminator_rparen)?;
                let closing_token = self.peek().clone();
                self.expect_token(TokenKind::RParen, "expected ')' after expression")?;
                let closing_span = Self::span_from_token(&closing_token);
                let span =
                    Self::union_spans(&token_span, &Self::union_spans(&expr.span, &closing_span));
                Ok(Self::make_expression(
                    span,
                    ExpressionKind::Grouping(Box::new(expr)),
                ))
            }
            TokenKind::LBrace => self.parse_dict_literal(token_span),
            TokenKind::LBracket => self.parse_list_literal(token_span),
            TokenKind::Pipe => self.parse_lambda_expression(token_span, terminator),
            TokenKind::PipePipe => self.parse_zero_arg_lambda(token_span, terminator),
            other => bail!(
                "unexpected token {:?} at line {}, column {}",
                other,
                token.line,
                token.column
            ),
        }
    }

    fn parse_interpolated_string(&mut self, start_span: SourceSpan) -> Result<Expression> {
        let mut parts = Vec::new();

        loop {
            match self.peek_kind() {
                TokenKind::InterpolatedStringSegment(_) => {
                    let segment_token = self.advance().clone();
                    if let TokenKind::InterpolatedStringSegment(value) = segment_token.kind {
                        parts.push(InterpolatedStringPart::Literal(value));
                    }
                }
                TokenKind::InterpolatedStringExprStart => {
                    self.advance();
                    let expression = self.parse_expression_with(|kind| {
                        matches!(kind, TokenKind::InterpolatedStringExprEnd)
                    })?;
                    self.expect_token(
                        TokenKind::InterpolatedStringExprEnd,
                        "expected '}' to close interpolation",
                    )?;
                    parts.push(InterpolatedStringPart::Expression(expression));
                }
                TokenKind::InterpolatedStringEnd => {
                    let end_token = self.advance().clone();
                    let end_span = Self::span_from_token(&end_token);
                    let span = Self::union_spans(&start_span, &end_span);
                    return Ok(Self::make_expression(
                        span,
                        ExpressionKind::InterpolatedString(InterpolatedStringExpression { parts }),
                    ));
                }
                TokenKind::Eof => {
                    self.diagnostics
                        .push_error_with_span("unterminated interpolated string", Some(start_span));
                    bail!("unterminated interpolated string");
                }
                other => {
                    let token = self.peek().clone();
                    let span = Self::span_from_token(&token);
                    self.diagnostics.push_error_with_span(
                        format!("unexpected token {:?} inside interpolated string", other),
                        Some(span),
                    );
                    bail!("invalid interpolated string");
                }
            }
        }
    }

    fn parse_list_literal(&mut self, opening_span: SourceSpan) -> Result<Expression> {
        let mut elements = Vec::new();
        self.skip_newlines();
        if matches!(self.peek_kind(), TokenKind::RBracket) {
            let closing_token = self.advance().clone();
            let closing_span = Self::span_from_token(&closing_token);
            let span = Self::union_spans(&opening_span, &closing_span);
            return Ok(Self::make_expression(
                span,
                ExpressionKind::List(ListLiteral { elements }),
            ));
        }

        loop {
            let element =
                self.parse_expression_prec(Precedence::Lowest, terminator_comma_or_rbracket)?;
            elements.push(element);
            match self.peek_kind() {
                TokenKind::Comma => {
                    self.advance();
                    self.skip_newlines();
                }
                TokenKind::RBracket => {
                    let closing_token = self.advance().clone();
                    let closing_span = Self::span_from_token(&closing_token);
                    let mut span = Self::union_spans(&opening_span, &closing_span);
                    if let Some(last) = elements.last() {
                        span = Self::union_spans(&span, &last.span);
                    }
                    return Ok(Self::make_expression(
                        span,
                        ExpressionKind::List(ListLiteral { elements }),
                    ));
                }
                other => {
                    bail!("expected ',' or ']' in list literal, found {:?}", other);
                }
            }
        }
    }

    fn parse_dict_literal(&mut self, opening_span: SourceSpan) -> Result<Expression> {
        let mut entries = Vec::new();
        self.skip_newlines();
        if matches!(self.peek_kind(), TokenKind::RBrace) {
            let closing_token = self.advance().clone();
            let closing_span = Self::span_from_token(&closing_token);
            let span = Self::union_spans(&opening_span, &closing_span);
            return Ok(Self::make_expression(
                span,
                ExpressionKind::Dict(DictLiteral { entries }),
            ));
        }

        loop {
            let key_token = self.peek().clone();
            let key = match &key_token.kind {
                TokenKind::Identifier => {
                    self.advance();
                    key_token.lexeme
                }
                TokenKind::StringLiteral(value) => {
                    self.advance();
                    value.clone()
                }
                _ => bail!(
                    "expected identifier or string literal as dictionary key at line {}, column {}",
                    key_token.line,
                    key_token.column
                ),
            };

            self.expect_token(TokenKind::Colon, "expected ':' after dictionary key")?;
            let value =
                self.parse_expression_prec(Precedence::Lowest, terminator_comma_or_rbrace)?;
            entries.push(DictEntry { key, value });

            match self.peek_kind() {
                TokenKind::Comma => {
                    self.advance();
                    self.skip_newlines();
                }
                TokenKind::RBrace => {
                    let closing_token = self.advance().clone();
                    let closing_span = Self::span_from_token(&closing_token);
                    let mut span = Self::union_spans(&opening_span, &closing_span);
                    if let Some(last) = entries.last() {
                        span = Self::union_spans(&span, &last.value.span);
                    }
                    return Ok(Self::make_expression(
                        span,
                        ExpressionKind::Dict(DictLiteral { entries }),
                    ));
                }
                other => {
                    bail!(
                        "expected ',' or a closing brace in dictionary literal, found {:?}",
                        other
                    )
                }
            }
        }
    }

    fn parse_lambda_expression(
        &mut self,
        start_span: SourceSpan,
        terminator: fn(&TokenKind) -> bool,
    ) -> Result<Expression> {
        let mut parameters = Vec::new();
        if matches!(self.peek_kind(), TokenKind::Pipe) {
            self.advance();
        } else {
            loop {
                let name_token = self.peek().clone();
                let span = Self::span_from_token(&name_token);
                let name = match &name_token.kind {
                    TokenKind::Identifier => {
                        self.advance();
                        name_token.lexeme
                    }
                    _ => bail!(
                        "expected parameter name in lambda at line {}, column {}",
                        name_token.line,
                        name_token.column
                    ),
                };

                let type_annotation = if matches!(self.peek_kind(), TokenKind::Colon) {
                    self.advance();
                    let tokens = self.collect_type_tokens();
                    Some(TypeExpression { tokens })
                } else {
                    None
                };

                parameters.push(FunctionParameter {
                    name,
                    span,
                    type_annotation,
                    default_value: None,
                });

                match self.peek_kind() {
                    TokenKind::Comma => {
                        self.advance();
                    }
                    TokenKind::Pipe => {
                        self.advance();
                        break;
                    }
                    other => {
                        let span = if self.is_at_end() {
                            span
                        } else {
                            Self::span_from_token(&self.peek().clone())
                        };
                        self.diagnostics.push_error_with_span(
                            format!(
                                "expected ',' or '|' in lambda parameters, found {:?}",
                                other
                            ),
                            Some(span),
                        );
                        bail!("invalid lambda parameter separator");
                    }
                }
            }
        }

        self.expect_token(TokenKind::FatArrow, "expected '=>' after lambda parameters")?;
        self.skip_newlines();

        let (body, body_span) = if matches!(self.peek_kind(), TokenKind::LBrace) {
            let (block, closing_span) = self.parse_braced_block()?;
            (LambdaBody::Block(block), closing_span)
        } else {
            let expression = self.parse_expression_prec(Precedence::Lowest, terminator)?;
            let span = expression.span;
            (LambdaBody::Expression(Box::new(expression)), span)
        };

        let span = Self::union_spans(&start_span, &body_span);
        let id = self.allocate_lambda_id();
        Ok(Self::make_expression(
            span,
            ExpressionKind::Lambda(LambdaExpression {
                id,
                parameters,
                body,
            }),
        ))
    }

    fn parse_zero_arg_lambda(
        &mut self,
        start_span: SourceSpan,
        terminator: fn(&TokenKind) -> bool,
    ) -> Result<Expression> {
        self.expect_token(TokenKind::FatArrow, "expected '=>' after '||'")?;
        self.skip_newlines();

        let (body, body_span) = if matches!(self.peek_kind(), TokenKind::LBrace) {
            let (block, closing_span) = self.parse_braced_block()?;
            (LambdaBody::Block(block), closing_span)
        } else {
            let expression = self.parse_expression_prec(Precedence::Lowest, terminator)?;
            let span = expression.span;
            (LambdaBody::Expression(Box::new(expression)), span)
        };

        let span = Self::union_spans(&start_span, &body_span);
        let id = self.allocate_lambda_id();
        Ok(Self::make_expression(
            span,
            ExpressionKind::Lambda(LambdaExpression {
                id,
                parameters: Vec::new(),
                body,
            }),
        ))
    }

    fn parse_braced_block(&mut self) -> Result<(Block, SourceSpan)> {
        self.expect_token(TokenKind::LBrace, "expected '{' to start block")?;
        let mut statements = Vec::new();
        let closing_span;

        loop {
            self.skip_newlines();
            if matches!(self.peek_kind(), TokenKind::RBrace) {
                let closing_token = self.advance().clone();
                closing_span = Self::span_from_token(&closing_token);
                break;
            }
            if self.is_at_end() {
                bail!("unterminated block, expected '}}'");
            }
            statements.push(self.parse_statement()?);
        }

        Ok((Block { statements }, closing_span))
    }

    fn parse_infix_expression(
        &mut self,
        left: Expression,
        precedence: Precedence,
        terminator: fn(&TokenKind) -> bool,
    ) -> Result<Expression> {
        let operator_token = self.advance().clone();
        let operator_span = Self::span_from_token(&operator_token);
        match operator_token.kind {
            TokenKind::Equal => {
                let value = self.parse_expression_prec(precedence, terminator)?;
                let span =
                    Self::union_spans(&Self::union_spans(&left.span, &operator_span), &value.span);
                Ok(Self::make_expression(
                    span,
                    ExpressionKind::Assignment(AssignmentExpression {
                        target: Box::new(left),
                        value: Box::new(value),
                    }),
                ))
            }
            TokenKind::DotDot | TokenKind::DotDotDot => {
                let inclusive = matches!(operator_token.kind, TokenKind::DotDot);
                let right = self.parse_expression_prec(precedence, terminator)?;
                let span =
                    Self::union_spans(&Self::union_spans(&left.span, &operator_span), &right.span);
                Ok(Self::make_expression(
                    span,
                    ExpressionKind::Range(RangeExpression {
                        start: Box::new(left),
                        end: Box::new(right),
                        inclusive,
                    }),
                ))
            }
            TokenKind::DoubleEqual
            | TokenKind::BangEqual
            | TokenKind::Greater
            | TokenKind::GreaterEqual
            | TokenKind::Less
            | TokenKind::LessEqual
            | TokenKind::Plus
            | TokenKind::Minus
            | TokenKind::Star
            | TokenKind::Slash
            | TokenKind::Percent
            | TokenKind::Keyword(Keyword::And)
            | TokenKind::Keyword(Keyword::Or) => {
                let operator = binary_operator_from_token(&operator_token.kind)?;
                let right = self.parse_expression_prec(precedence, terminator)?;
                let span =
                    Self::union_spans(&Self::union_spans(&left.span, &operator_span), &right.span);
                Ok(Self::make_expression(
                    span,
                    ExpressionKind::Binary(BinaryExpression {
                        operator,
                        left: Box::new(left),
                        right: Box::new(right),
                    }),
                ))
            }
            ref other => bail!(
                "unexpected infix operator {:?} at line {}, column {}",
                other,
                operator_token.line,
                operator_token.column
            ),
        }
    }

    fn collect_type_tokens(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();
        let mut depth = 0usize;

        while !self.is_at_end() {
            let should_break = match self.peek_kind() {
                TokenKind::Equal
                | TokenKind::Newline
                | TokenKind::RBrace
                | TokenKind::Semicolon
                | TokenKind::Pipe
                | TokenKind::FatArrow => depth == 0,
                TokenKind::Comma | TokenKind::RParen => depth == 0,
                _ => false,
            };

            if should_break {
                break;
            }

            let token = self.advance().clone();
            match token.kind {
                TokenKind::LBracket | TokenKind::LParen => {
                    depth += 1;
                }
                TokenKind::RBracket | TokenKind::RParen => {
                    if depth > 0 {
                        depth -= 1;
                    }
                }
                _ => {}
            }
            tokens.push(token);
        }

        tokens
    }

    fn expect_newline(&mut self, message: &str) -> Result<()> {
        match self.peek_kind() {
            TokenKind::Newline => {
                self.skip_newlines();
                Ok(())
            }
            TokenKind::Eof | TokenKind::RBrace => Ok(()),
            other => {
                let span = Self::span_from_token(&self.peek().clone());
                self.diagnostics
                    .push_error_with_span(format!("{} (found '{:?}')", message, other), Some(span));
                bail!("unexpected token");
            }
        }
    }

    fn expect_keyword(&mut self, keyword: Keyword, message: &str) -> Result<()> {
        if self.check_keyword(keyword) {
            self.advance();
            Ok(())
        } else {
            let token = self.peek().clone();
            bail!(
                "{} at line {}, column {} (found '{}')",
                message,
                token.line,
                token.column,
                token.lexeme
            );
        }
    }

    fn try_finish_generic_call(&mut self, callee: &Expression) -> Result<Option<Expression>> {
        if !matches!(
            callee.kind,
            ExpressionKind::Identifier(_) | ExpressionKind::Member(_)
        ) {
            return Ok(None);
        }
        if !matches!(self.peek_kind(), TokenKind::LBracket) {
            return Ok(None);
        }

        let saved_index = self.current;
        self.advance(); // consume '['
        let Some(type_arguments) = self.parse_type_argument_list()? else {
            self.current = saved_index;
            return Ok(None);
        };

        self.skip_newlines();
        let closing_token = self.peek().clone();
        if !matches!(closing_token.kind, TokenKind::RBracket) {
            self.current = saved_index;
            return Ok(None);
        }
        let closing_span = Self::span_from_token(&closing_token);
        self.advance(); // consume ']'

        if !matches!(self.peek_kind(), TokenKind::LParen) {
            self.current = saved_index;
            return Ok(None);
        }

        let mut callee_with_generics = callee.clone();
        callee_with_generics.span = Self::union_spans(&callee_with_generics.span, &closing_span);
        let expression = self.finish_call(callee_with_generics, type_arguments)?;
        Ok(Some(expression))
    }

    fn parse_type_argument_list(&mut self) -> Result<Option<Vec<TypeExpression>>> {
        self.skip_newlines();
        if matches!(self.peek_kind(), TokenKind::RBracket) {
            return Ok(None);
        }

        let mut arguments = Vec::new();
        loop {
            let Some(tokens) = self.collect_type_argument_tokens() else {
                return Ok(None);
            };
            arguments.push(TypeExpression { tokens });

            self.skip_newlines();
            match self.peek_kind() {
                TokenKind::Comma => {
                    self.advance();
                    self.skip_newlines();
                }
                TokenKind::RBracket => break,
                _ => return Ok(None),
            }
        }

        Ok(Some(arguments))
    }

    fn collect_type_argument_tokens(&mut self) -> Option<Vec<Token>> {
        self.skip_newlines();
        let mut tokens = Vec::new();
        let mut depth = 0usize;

        while !self.is_at_end() {
            let token = self.peek().clone();
            match token.kind {
                TokenKind::Comma if depth == 0 => break,
                TokenKind::RBracket if depth == 0 => break,
                TokenKind::LBracket | TokenKind::LParen => {
                    depth += 1;
                    tokens.push(token);
                    self.advance();
                }
                TokenKind::RBracket | TokenKind::RParen => {
                    if depth == 0 {
                        break;
                    }
                    depth -= 1;
                    tokens.push(token);
                    self.advance();
                }
                TokenKind::Newline if depth == 0 => return None,
                _ => {
                    tokens.push(token);
                    self.advance();
                }
            }
        }

        if tokens.is_empty() {
            return None;
        }

        let valid_start = match tokens.first().map(|token| &token.kind) {
            Some(TokenKind::Identifier) => tokens[0]
                .lexeme
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_uppercase()),
            Some(TokenKind::Keyword(Keyword::Nil)) => true,
            _ => false,
        };

        if !valid_start {
            return None;
        }

        Some(tokens)
    }

    fn expect_token(&mut self, expected: TokenKind, message: &str) -> Result<()> {
        if std::mem::discriminant(self.peek_kind()) == std::mem::discriminant(&expected) {
            self.advance();
            Ok(())
        } else {
            let token = self.peek().clone();
            bail!(
                "{} at line {}, column {} (found '{}')",
                message,
                token.line,
                token.column,
                token.lexeme
            );
        }
    }

    fn finish_call(
        &mut self,
        callee: Expression,
        type_arguments: Vec<TypeExpression>,
    ) -> Result<Expression> {
        self.expect_token(TokenKind::LParen, "expected '(' to start argument list")?;
        let open_token = self.tokens.get(self.current - 1).cloned().unwrap();
        let open_span = Self::span_from_token(&open_token);

        let mut arguments: Vec<CallArgument> = Vec::new();
        self.skip_newlines();
        if matches!(self.peek_kind(), TokenKind::RParen) {
            let closing_token = self.advance().clone();
            let closing_span = Self::span_from_token(&closing_token);
            let span =
                Self::union_spans(&Self::union_spans(&callee.span, &open_span), &closing_span);
            return Ok(Self::make_expression(
                span,
                ExpressionKind::Call(CallExpression {
                    callee: Box::new(callee),
                    type_arguments,
                    arguments,
                }),
            ));
        }

        loop {
            self.skip_newlines();

            let (name, name_span) = if matches!(self.peek_kind(), TokenKind::Identifier)
                && matches!(self.peek_kind_at(1), Some(TokenKind::Colon))
            {
                let name_token = self.advance().clone();
                let span = Some(Self::span_from_token(&name_token));
                self.advance(); // consume ':'
                self.skip_newlines();
                (Some(name_token.lexeme), span)
            } else {
                (None, None)
            };

            let expression =
                self.parse_expression_prec(Precedence::Lowest, terminator_comma_or_rparen)?;
            let expr_span = expression.span;
            arguments.push(CallArgument {
                name,
                name_span,
                expression,
            });
            match self.peek_kind() {
                TokenKind::Comma => {
                    self.advance();
                }
                TokenKind::RParen => {
                    let closing_token = self.advance().clone();
                    let closing_span = Self::span_from_token(&closing_token);
                    let mut span = Self::union_spans(&callee.span, &open_span);
                    if let Some(last) = arguments.last() {
                        span = Self::union_spans(&span, &last.expression.span);
                    }
                    span = Self::union_spans(&span, &closing_span);
                    return Ok(Self::make_expression(
                        span,
                        ExpressionKind::Call(CallExpression {
                            callee: Box::new(callee),
                            type_arguments,
                            arguments,
                        }),
                    ));
                }
                other => {
                    let span = if self.is_at_end() {
                        expr_span
                    } else {
                        Self::span_from_token(&self.peek().clone())
                    };
                    self.diagnostics.push_error_with_span(
                        format!("expected ',' or ')' in argument list, found {:?}", other),
                        Some(span),
                    );
                    bail!("invalid argument separator");
                }
            }
        }
    }

    fn finish_index(&mut self, object: Expression) -> Result<Expression> {
        self.expect_token(TokenKind::LBracket, "expected '[' for index expression")?;
        let open_token = self.tokens.get(self.current - 1).cloned().unwrap();
        let open_span = Self::span_from_token(&open_token);
        let index = self.parse_expression_prec(Precedence::Lowest, terminator_rbracket)?;
        let closing_token = self.peek().clone();
        self.expect_token(TokenKind::RBracket, "expected ']' after index expression")?;
        let closing_span = Self::span_from_token(&closing_token);

        let span = Self::union_spans(
            &Self::union_spans(&object.span, &open_span),
            &Self::union_spans(&index.span, &closing_span),
        );

        Ok(Self::make_expression(
            span,
            ExpressionKind::Index(IndexExpression {
                object: Box::new(object),
                index: Box::new(index),
            }),
        ))
    }

    fn finish_member(&mut self, object: Expression) -> Result<Expression> {
        self.expect_token(TokenKind::Dot, "expected '.' for member access")?;
        let dot_token = self.tokens.get(self.current - 1).cloned().unwrap();
        let dot_span = Self::span_from_token(&dot_token);
        let name_token = self.peek().clone();
        let property_span = Self::span_from_token(&name_token);
        let property = match name_token.kind {
            TokenKind::Identifier | TokenKind::Keyword(_) => {
                self.advance();
                name_token.lexeme
            }
            ref other => {
                bail!(
                    "expected identifier after '.', found {:?} at line {}, column {}",
                    other,
                    name_token.line,
                    name_token.column
                );
            }
        };

        let span = Self::union_spans(&object.span, &Self::union_spans(&dot_span, &property_span));

        Ok(Self::make_expression(
            span,
            ExpressionKind::Member(MemberExpression {
                object: Box::new(object),
                property,
                property_span,
            }),
        ))
    }

    fn skip_newlines(&mut self) {
        while matches!(self.peek_kind(), TokenKind::Newline) {
            self.advance();
        }
    }

    fn check_keyword(&self, keyword: Keyword) -> bool {
        matches!(self.peek_keyword(), Some(kw) if kw == keyword)
    }

    fn peek_keyword(&self) -> Option<Keyword> {
        match self.peek_kind() {
            TokenKind::Keyword(kw) => Some(*kw),
            _ => None,
        }
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.current]
    }

    fn peek_kind(&self) -> &TokenKind {
        &self.tokens[self.current].kind
    }

    fn peek_kind_at(&self, offset: usize) -> Option<TokenKind> {
        self.tokens
            .get(self.current + offset)
            .map(|token| token.kind.clone())
    }

    fn advance(&mut self) -> &Token {
        if !self.is_at_end() {
            self.current += 1;
        }
        &self.tokens[self.current - 1]
    }

    fn is_at_end(&self) -> bool {
        matches!(self.peek_kind(), TokenKind::Eof)
    }
}

fn default_expression_terminator(kind: &TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Newline
            | TokenKind::Semicolon
            | TokenKind::Eof
            | TokenKind::Keyword(Keyword::End)
            | TokenKind::Keyword(Keyword::Else)
    )
}

fn terminator_default_or_comma(kind: &TokenKind) -> bool {
    default_expression_terminator(kind) || matches!(kind, TokenKind::Comma)
}

fn terminator_comma_or_rparen(kind: &TokenKind) -> bool {
    matches!(kind, TokenKind::Comma | TokenKind::RParen)
}

fn terminator_comma_or_rbracket(kind: &TokenKind) -> bool {
    matches!(kind, TokenKind::Comma | TokenKind::RBracket)
}

fn terminator_comma_or_rbrace(kind: &TokenKind) -> bool {
    matches!(kind, TokenKind::Comma | TokenKind::RBrace)
}

fn terminator_keyword_of(kind: &TokenKind) -> bool {
    matches!(kind, TokenKind::Keyword(Keyword::Of))
}

fn terminator_rparen(kind: &TokenKind) -> bool {
    matches!(kind, TokenKind::RParen)
}

fn terminator_rbracket(kind: &TokenKind) -> bool {
    matches!(kind, TokenKind::RBracket)
}

fn binary_operator_from_token(kind: &TokenKind) -> Result<BinaryOperator> {
    let operator = match kind {
        TokenKind::Plus => BinaryOperator::Add,
        TokenKind::Minus => BinaryOperator::Subtract,
        TokenKind::Star => BinaryOperator::Multiply,
        TokenKind::Slash => BinaryOperator::Divide,
        TokenKind::Percent => BinaryOperator::Modulo,
        TokenKind::DoubleEqual => BinaryOperator::Equal,
        TokenKind::BangEqual => BinaryOperator::NotEqual,
        TokenKind::Greater => BinaryOperator::Greater,
        TokenKind::GreaterEqual => BinaryOperator::GreaterEqual,
        TokenKind::Less => BinaryOperator::Less,
        TokenKind::LessEqual => BinaryOperator::LessEqual,
        TokenKind::Keyword(Keyword::And) => BinaryOperator::And,
        TokenKind::Keyword(Keyword::Or) => BinaryOperator::Or,
        other => bail!("unsupported binary operator {:?}", other),
    };
    Ok(operator)
}
