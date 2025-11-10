use anyhow::{bail, Result};

use crate::ast::*;
use crate::diagnostics::Diagnostics;
use crate::lexer::{Keyword, Token, TokenKind};
use crate::source::SourceFile;

#[derive(Copy, Clone, PartialEq, PartialOrd)]
enum Precedence {
    Lowest = 0,
    Assignment,
    Coalesce,
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
            TokenKind::Equal
            | TokenKind::PlusEqual
            | TokenKind::MinusEqual
            | TokenKind::StarEqual => Some(Precedence::Assignment),
            TokenKind::QuestionQuestion => Some(Precedence::Coalesce),
            TokenKind::PipePipe => Some(Precedence::Or),
            TokenKind::AmpersandAmpersand => Some(Precedence::And),
            TokenKind::DoubleEqual | TokenKind::BangEqual => Some(Precedence::Equality),
            TokenKind::Greater
            | TokenKind::GreaterEqual
            | TokenKind::Less
            | TokenKind::LessEqual => Some(Precedence::Comparison),
            TokenKind::Keyword(Keyword::Is) => Some(Precedence::Comparison),
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

    fn span_from_tokens(tokens: &[Token]) -> SourceSpan {
        let mut iter = tokens.iter();
        let Some(first) = iter.next() else {
            return SourceSpan::default();
        };
        let mut span = Self::span_from_token(first);
        for token in iter {
            let token_span = Self::span_from_token(token);
            span = Self::union_spans(&span, &token_span);
        }
        span
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
            TokenKind::Keyword(Keyword::Union) => self.parse_union(docstring),
            TokenKind::Keyword(Keyword::Enum) => self.parse_enum(docstring),
            TokenKind::Keyword(Keyword::Error) => self.parse_error(docstring),
            TokenKind::Keyword(Keyword::If) => self.parse_conditional(ConditionalKind::If),
            TokenKind::Keyword(Keyword::For) => self.parse_for_loop(),
            TokenKind::Keyword(Keyword::While) => self.parse_loop(LoopKind::While),
            TokenKind::Keyword(Keyword::Break) => self.parse_break(),
            TokenKind::Keyword(Keyword::Continue) => self.parse_continue(),
            TokenKind::Keyword(Keyword::Return) => self.parse_return(),
            TokenKind::Keyword(Keyword::Match) => self.parse_match_statement(),
            TokenKind::Keyword(Keyword::Throw) => self.parse_throw(),
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

        let error_annotation = if matches!(self.peek_kind(), TokenKind::Bang) {
            self.advance();
            Some(self.parse_error_annotation(&name)?)
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
            error_annotation,
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

        self.skip_newlines();
        self.expect_token(
            TokenKind::LBrace,
            "expected '{' to start struct body after struct name",
        )?;
        self.expect_newline("expected newline after '{' in struct declaration")?;

        let mut fields = Vec::new();
        loop {
            self.skip_newlines();
            if matches!(self.peek_kind(), TokenKind::RBrace) {
                self.advance();
                break;
            }
            if self.is_at_end() {
                self.diagnostics.push_error_with_span(
                    format!("unterminated struct '{}', missing '}}'", name),
                    Some(name_span),
                );
                bail!("unterminated struct");
            }

            let field_docstring = self.consume_doc_comments();
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
                docstring: field_docstring,
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

    fn parse_union(&mut self, docstring: Option<String>) -> Result<Statement> {
        self.advance(); // consume 'union'
        let name_token = self.peek().clone();
        let name_span = Self::span_from_token(&name_token);
        let name = match &name_token.kind {
            TokenKind::Identifier => {
                self.advance();
                name_token.lexeme
            }
            _ => bail!(
                "expected union name at line {}, column {}",
                name_token.line,
                name_token.column
            ),
        };

        self.skip_newlines();
        self.expect_token(
            TokenKind::LBrace,
            "expected '{' to start union body after union name",
        )?;
        self.expect_newline("expected newline after '{' in union declaration")?;

        let mut members = Vec::new();
        loop {
            self.skip_newlines();
            if matches!(self.peek_kind(), TokenKind::RBrace) {
                self.advance();
                break;
            }
            if self.is_at_end() {
                self.diagnostics.push_error_with_span(
                    format!("unterminated union '{}', missing '}}'", name),
                    Some(name_span),
                );
                bail!("unterminated union");
            }

            let tokens = self.collect_type_tokens();
            if tokens.is_empty() {
                let span = Self::span_from_token(&self.peek().clone());
                self.diagnostics
                    .push_error_with_span("union members must specify a type", Some(span));
                bail!("missing union member type");
            }
            let type_span = Self::span_from_tokens(&tokens);
            members.push(UnionMember {
                type_expression: TypeExpression { tokens },
                span: type_span,
            });

            self.expect_newline("expected newline after union member")?;
        }

        if members.is_empty() {
            self.diagnostics.push_error_with_span(
                format!("union '{}' must declare at least one member type", name),
                Some(name_span),
            );
            bail!("union missing members");
        }

        self.expect_newline("expected newline after union declaration")?;

        Ok(Statement::Union(UnionStatement {
            name,
            name_span,
            members,
            docstring,
        }))
    }

    fn parse_enum(&mut self, docstring: Option<String>) -> Result<Statement> {
        self.advance(); // consume 'enum'
        let name_token = self.peek().clone();
        let name_span = Self::span_from_token(&name_token);
        let name = match &name_token.kind {
            TokenKind::Identifier => {
                self.advance();
                name_token.lexeme
            }
            _ => bail!(
                "expected enum name at line {}, column {}",
                name_token.line,
                name_token.column
            ),
        };

        let _type_parameters = if matches!(self.peek_kind(), TokenKind::LBracket) {
            self.advance(); // consume '['
            self.parse_type_parameters("enum", &name, name_span.line)?
        } else {
            Vec::new()
        };

        self.skip_newlines();
        self.expect_token(
            TokenKind::LBrace,
            "expected '{' to start enum body after enum name",
        )?;
        self.expect_newline("expected newline after '{' in enum declaration")?;

        let mut variants = Vec::new();
        loop {
            self.skip_newlines();
            if matches!(self.peek_kind(), TokenKind::RBrace) {
                self.advance();
                break;
            }
            if self.is_at_end() {
                self.diagnostics.push_error_with_span(
                    format!("unterminated enum '{}', missing '}}'", name),
                    Some(name_span),
                );
                bail!("unterminated enum");
            }

            let variant_doc = self.consume_doc_comments();

            let variant_token = self.peek().clone();
            let variant_span = Self::span_from_token(&variant_token);
            let variant_name = match &variant_token.kind {
                TokenKind::Identifier => {
                    self.advance();
                    variant_token.lexeme
                }
                _ => bail!(
                    "expected enum variant name in enum '{}' at line {}, column {}",
                    name,
                    variant_token.line,
                    variant_token.column
                ),
            };

            variants.push(EnumVariant {
                name: variant_name,
                span: variant_span,
                docstring: variant_doc,
            });

            self.expect_newline("expected newline after enum variant")?;
        }

        self.expect_newline("expected newline after enum declaration")?;

        Ok(Statement::Enum(EnumStatement {
            name,
            name_span,
            variants,
            docstring,
        }))
    }

    fn parse_error(&mut self, docstring: Option<String>) -> Result<Statement> {
        let keyword_token = self.advance().clone();
        let keyword_span = Self::span_from_token(&keyword_token);

        let name_token = self.peek().clone();
        let name_span = Self::span_from_token(&name_token);
        let name = match &name_token.kind {
            TokenKind::Identifier => {
                self.advance();
                name_token.lexeme
            }
            _ => bail!(
                "expected error name at line {}, column {}",
                name_token.line,
                name_token.column
            ),
        };

        let mut span = Self::union_spans(&keyword_span, &name_span);
        let variants = if matches!(self.peek_kind(), TokenKind::LBrace) {
            self.advance(); // consume '{'
            self.expect_newline("expected newline after '{' in error declaration")?;
            let mut variants = Vec::new();
            loop {
                self.skip_newlines();
                match self.peek_kind() {
                    TokenKind::Identifier => {
                        let variant = self.parse_error_variant(&name)?;
                        span = Self::union_spans(&span, &variant.span);
                        variants.push(variant);
                    }
                    TokenKind::RBrace => {
                        let closing = self.advance().clone();
                        span = Self::union_spans(&span, &Self::span_from_token(&closing));
                        break;
                    }
                    TokenKind::Eof => {
                        self.diagnostics.push_error_with_span(
                            format!("unterminated error '{}', missing '}}'", name),
                            Some(name_span),
                        );
                        bail!("unterminated error declaration");
                    }
                    other => {
                        let token = self.peek().clone();
                        self.diagnostics.push_error_with_span(
                            format!(
                                "unexpected token {:?} in error declaration '{}'",
                                other, name
                            ),
                            Some(Self::span_from_token(&token)),
                        );
                        bail!("invalid token in error declaration");
                    }
                }
            }

            if variants.is_empty() {
                self.diagnostics.push_error_with_span(
                    format!("error '{}' must declare at least one variant", name),
                    Some(name_span),
                );
                bail!("error must declare variants");
            }

            self.expect_newline("expected newline after error declaration")?;
            variants
        } else {
            self.expect_newline("expected newline after error declaration")?;
            span = Self::union_spans(&span, &name_span);
            vec![ErrorVariant {
                name: name.clone(),
                name_span,
                fields: Vec::new(),
                span: name_span,
            }]
        };

        Ok(Statement::Error(ErrorStatement {
            name,
            name_span,
            variants,
            docstring,
            span,
        }))
    }

    fn parse_error_variant(&mut self, error_name: &str) -> Result<ErrorVariant> {
        let name_token = self.peek().clone();
        let name_span = Self::span_from_token(&name_token);
        let name = match &name_token.kind {
            TokenKind::Identifier => {
                self.advance();
                name_token.lexeme
            }
            _ => bail!(
                "expected variant name in error '{}' at line {}, column {}",
                error_name,
                name_token.line,
                name_token.column
            ),
        };

        let mut span = name_span;
        let mut fields = Vec::new();
        if matches!(self.peek_kind(), TokenKind::LParen) {
            let open_token = self.advance().clone();
            span = Self::union_spans(&span, &Self::span_from_token(&open_token));

            loop {
                self.skip_newlines();
                let field_token = self.peek().clone();
                let field_span = Self::span_from_token(&field_token);
                let field_name = match &field_token.kind {
                    TokenKind::Identifier => {
                        self.advance();
                        field_token.lexeme
                    }
                    _ => bail!(
                        "expected field name in error variant '{}.{}' at line {}, column {}",
                        error_name,
                        name,
                        field_token.line,
                        field_token.column
                    ),
                };

                if !matches!(self.peek_kind(), TokenKind::Colon) {
                    let token = self.peek().clone();
                    self.diagnostics.push_error_with_span(
                        format!(
                            "expected ':' after field '{}' in error variant '{}.{}'",
                            field_name, error_name, name
                        ),
                        Some(Self::span_from_token(&token)),
                    );
                    bail!("missing ':' in error variant field");
                }
                self.advance(); // consume ':'

                let type_tokens = self.collect_type_tokens();
                if type_tokens.is_empty() {
                    self.diagnostics.push_error_with_span(
                        format!(
                            "missing type annotation for field '{}' in error variant '{}.{}'",
                            field_name, error_name, name
                        ),
                        Some(field_span),
                    );
                    bail!("missing error variant field type");
                }

                fields.push(ErrorField {
                    name: field_name,
                    name_span: field_span,
                    type_annotation: TypeExpression {
                        tokens: type_tokens,
                    },
                });

                self.skip_newlines();
                match self.peek_kind() {
                    TokenKind::Comma => {
                        self.advance();
                    }
                    TokenKind::RParen => break,
                    other => {
                        let token = self.peek().clone();
                        self.diagnostics.push_error_with_span(
                            format!(
                                "unexpected token {:?} in fields for error variant '{}.{}'",
                                other, error_name, name
                            ),
                            Some(Self::span_from_token(&token)),
                        );
                        bail!("invalid token in error variant fields");
                    }
                }
            }

            let close_token = self.peek().clone();
            self.expect_token(
                TokenKind::RParen,
                "expected ')' to close error variant fields",
            )?;
            span = Self::union_spans(&span, &Self::span_from_token(&close_token));
        }

        self.expect_newline("expected newline after error variant")?;

        Ok(ErrorVariant {
            name,
            name_span,
            fields,
            span,
        })
    }

    fn parse_error_annotation(&mut self, function_name: &str) -> Result<ErrorAnnotation> {
        if matches!(self.peek_kind(), TokenKind::LBrace) {
            let open_token = self.advance().clone();
            let open_span = Self::span_from_token(&open_token);

            let mut types = Vec::new();
            loop {
                self.skip_newlines();
                match self.peek_kind() {
                    TokenKind::Identifier => {
                        let spec = self.parse_error_type_specifier(function_name)?;
                        types.push(spec);
                        self.skip_newlines();
                        match self.peek_kind() {
                            TokenKind::Comma => {
                                self.advance();
                                self.skip_newlines();
                            }
                            TokenKind::RBrace => {}
                            other => {
                                let token = self.peek().clone();
                                self.diagnostics.push_error_with_span(
                                    format!(
                                        "unexpected token {:?} in error list for function '{}'",
                                        other, function_name
                                    ),
                                    Some(Self::span_from_token(&token)),
                                );
                                bail!("invalid token in error annotation");
                            }
                        }
                    }
                    TokenKind::RBrace => {
                        let closing = self.advance().clone();
                        if types.is_empty() {
                            self.diagnostics.push_error_with_span(
                                format!(
                                    "function '{}' must list at least one error after '!'",
                                    function_name
                                ),
                                Some(Self::span_from_token(&closing)),
                            );
                            bail!("empty error annotation");
                        }
                        let mut span = open_span;
                        for spec in &types {
                            span = Self::union_spans(&span, &spec.span);
                        }
                        span = Self::union_spans(&span, &Self::span_from_token(&closing));
                        return Ok(ErrorAnnotation { types, span });
                    }
                    TokenKind::Eof => {
                        self.diagnostics.push_error_with_span(
                            format!(
                                "unterminated error list in function '{}' after '!'",
                                function_name
                            ),
                            Some(open_span),
                        );
                        bail!("unterminated error list");
                    }
                    other => {
                        let token = self.peek().clone();
                        self.diagnostics.push_error_with_span(
                            format!(
                                "unexpected token {:?} in error list for function '{}'",
                                other, function_name
                            ),
                            Some(Self::span_from_token(&token)),
                        );
                        bail!("invalid token in error annotation");
                    }
                }
            }
        } else {
            let spec = self.parse_error_type_specifier(function_name)?;
            let span = spec.span;
            Ok(ErrorAnnotation {
                types: vec![spec],
                span,
            })
        }
    }

    fn parse_error_type_specifier(&mut self, function_name: &str) -> Result<ErrorTypeSpecifier> {
        let first_token = self.peek().clone();
        let mut span = Self::span_from_token(&first_token);
        let mut path = Vec::new();
        let ident = match &first_token.kind {
            TokenKind::Identifier => {
                self.advance();
                first_token.lexeme
            }
            _ => bail!(
                "expected error type after '!' in function '{}' at line {}, column {}",
                function_name,
                first_token.line,
                first_token.column
            ),
        };
        path.push(ident);

        while matches!(self.peek_kind(), TokenKind::Dot) {
            let dot_token = self.advance().clone();
            let segment_token = self.peek().clone();
            match &segment_token.kind {
                TokenKind::Identifier => {
                    self.advance();
                    span = Self::union_spans(&span, &Self::span_from_token(&segment_token));
                    path.push(segment_token.lexeme);
                }
                other => {
                    self.diagnostics.push_error_with_span(
                        format!(
                            "expected identifier after '.' in error specifier for function '{}', found {:?}",
                            function_name, other
                        ),
                        Some(Self::span_from_token(&segment_token)),
                    );
                    bail!("invalid error specifier");
                }
            }
            span = Self::union_spans(&span, &Self::span_from_token(&dot_token));
        }

        Ok(ErrorTypeSpecifier { path, span })
    }

    fn parse_throw(&mut self) -> Result<Statement> {
        let throw_token = self.advance().clone();
        let throw_span = Self::span_from_token(&throw_token);
        let expression = self.parse_expression_with(default_expression_terminator)?;
        let span = Self::union_spans(&throw_span, &expression.span);
        self.expect_newline("expected newline after throw expression")?;
        Ok(Statement::Throw(ThrowStatement { expression, span }))
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

        // Parse the loop variable(s) - either `item` or `key, value`
        let first_token = self.peek().clone();
        let first_span = Self::span_from_token(&first_token);
        let first_name = match &first_token.kind {
            TokenKind::Identifier => {
                self.advance();
                first_token.lexeme
            }
            _ => {
                self.diagnostics
                    .push_error_with_span("expected identifier after 'for'", Some(first_span));
                bail!("invalid for loop variable");
            }
        };

        let pattern = if matches!(self.peek_kind(), TokenKind::Comma) {
            self.advance(); // consume ','
            let second_token = self.peek().clone();
            let second_span = Self::span_from_token(&second_token);
            let second_name = match &second_token.kind {
                TokenKind::Identifier => {
                    self.advance();
                    second_token.lexeme
                }
                _ => {
                    self.diagnostics.push_error_with_span(
                        "expected second identifier after ',' in for loop",
                        Some(second_span),
                    );
                    bail!("invalid for loop variable");
                }
            };
            ForPattern::Pair(
                Identifier {
                    name: first_name,
                    span: first_span,
                },
                Identifier {
                    name: second_name,
                    span: second_span,
                },
            )
        } else {
            ForPattern::Single(Identifier {
                name: first_name,
                span: first_span,
            })
        };

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

    fn parse_break(&mut self) -> Result<Statement> {
        let break_token = self.peek().clone();
        let span = Self::span_from_token(&break_token);
        self.advance(); // consume 'break'
        self.expect_newline("expected newline after break")?;
        Ok(Statement::Break(BreakStatement { span }))
    }

    fn parse_continue(&mut self) -> Result<Statement> {
        let continue_token = self.peek().clone();
        let span = Self::span_from_token(&continue_token);
        self.advance(); // consume 'continue'
        self.expect_newline("expected newline after continue")?;
        Ok(Statement::Continue(ContinueStatement { span }))
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
                TokenKind::Bang => {
                    let bang_token = self.advance().clone();
                    let bang_span = Self::span_from_token(&bang_token);
                    let span = Self::union_spans(&expr.span, &bang_span);
                    expr = Self::make_expression(span, ExpressionKind::Unwrap(Box::new(expr)));
                    continue;
                }
                TokenKind::Keyword(Keyword::Catch) => {
                    expr = self.finish_catch(expr, terminator)?;
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
            TokenKind::BuiltinIdentifier => {
                // Built-in identifier like @print, @len, @panic
                // Strip the @ prefix for the identifier name
                let name = token.lexeme.strip_prefix('@').unwrap_or(&token.lexeme).to_string();
                Ok(Self::make_expression(
                    token_span,
                    ExpressionKind::Identifier(Identifier {
                        name,
                        span: token_span,
                    }),
                ))
            }
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
            TokenKind::Keyword(Keyword::Def) => self.parse_anonymous_function(token_span),
            TokenKind::Keyword(Keyword::Match) => self.parse_match_expression(token_span),
            TokenKind::Keyword(Keyword::If) => self.parse_if_expression(token_span, terminator),
            TokenKind::Keyword(Keyword::Try) => {
                let expression = self.parse_expression_prec(Precedence::Unary, terminator)?;
                let span = Self::union_spans(&token_span, &expression.span);
                Ok(Self::make_expression(
                    span,
                    ExpressionKind::Try(TryExpression {
                        expression: Box::new(expression),
                        catch: None,
                    }),
                ))
            }
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

    fn parse_match_statement(&mut self) -> Result<Statement> {
        let match_token = self.advance().clone();
        let match_span = Self::span_from_token(&match_token);

        self.skip_newlines();
        let scrutinee =
            self.parse_expression_prec(Precedence::Lowest, default_expression_terminator)?;
        self.expect_newline("expected newline after match scrutinee")?;

        let mut arms = Vec::new();
        loop {
            self.skip_newlines();
            match self.peek_kind() {
                TokenKind::Keyword(Keyword::Case) => {
                    let arm = self.parse_match_arm_block()?;
                    arms.push(arm);
                }
                TokenKind::Keyword(Keyword::End) => break,
                TokenKind::Eof => {
                    self.diagnostics
                        .push_error_with_span("unterminated match statement", Some(match_span));
                    bail!("unterminated match statement");
                }
                other => {
                    let token = self.peek().clone();
                    self.diagnostics.push_error_with_span(
                        format!("unexpected token {:?} inside match statement", other),
                        Some(Self::span_from_token(&token)),
                    );
                    bail!("invalid match statement");
                }
            }
        }

        if arms.is_empty() {
            self.diagnostics.push_error_with_span(
                "match statement requires at least one case",
                Some(match_span),
            );
            bail!("match statement requires cases");
        }

        let end_token = self.peek().clone();
        self.expect_keyword(Keyword::End, "expected 'end' to close match statement")?;
        let end_span = Self::span_from_token(&end_token);
        self.expect_newline("expected newline after match statement")?;

        let mut span = Self::union_spans(&match_span, &scrutinee.span);
        if let Some(last_arm) = arms.last() {
            span = Self::union_spans(&span, &last_arm.span);
        }
        span = Self::union_spans(&span, &end_span);

        Ok(Statement::Match(MatchStatement {
            scrutinee,
            arms,
            span,
        }))
    }

    fn parse_match_expression(&mut self, match_span: SourceSpan) -> Result<Expression> {
        self.skip_newlines();
        let scrutinee =
            self.parse_expression_prec(Precedence::Lowest, default_expression_terminator)?;
        self.expect_newline("expected newline after match scrutinee")?;

        let mut arms = Vec::new();
        loop {
            self.skip_newlines();
            match self.peek_kind() {
                TokenKind::Keyword(Keyword::Case) => {
                    let arm = self.parse_match_arm()?;
                    arms.push(arm);
                }
                TokenKind::Keyword(Keyword::End) => break,
                TokenKind::Eof => {
                    self.diagnostics
                        .push_error_with_span("unterminated match expression", Some(match_span));
                    bail!("unterminated match expression");
                }
                other => {
                    let token = self.peek().clone();
                    self.diagnostics.push_error_with_span(
                        format!("unexpected token {:?} inside match expression", other),
                        Some(Self::span_from_token(&token)),
                    );
                    bail!("invalid match expression");
                }
            }
        }

        if arms.is_empty() {
            self.diagnostics.push_error_with_span(
                "match expression requires at least one case",
                Some(match_span),
            );
            bail!("match expression requires cases");
        }

        let end_token = self.peek().clone();
        self.expect_keyword(Keyword::End, "expected 'end' to close match expression")?;
        let end_span = Self::span_from_token(&end_token);

        let scrutinee_span = scrutinee.span;
        let mut span = Self::union_spans(&match_span, &scrutinee_span);
        if let Some(last_arm) = arms.last() {
            span = Self::union_spans(&span, &last_arm.span);
        }
        span = Self::union_spans(&span, &end_span);

        Ok(Self::make_expression(
            span,
            ExpressionKind::Match(MatchExpression {
                scrutinee: Box::new(scrutinee),
                arms,
            }),
        ))
    }

    fn parse_if_expression(
        &mut self,
        if_span: SourceSpan,
        terminator: fn(&TokenKind) -> bool,
    ) -> Result<Expression> {
        self.skip_newlines();

        // Parse condition (requires parentheses for clarity)
        self.expect_token(
            TokenKind::LParen,
            "expected '(' after 'if' in if-expression",
        )?;
        let condition = self.parse_expression_prec(Precedence::Lowest, terminator_rparen)?;
        self.expect_token(
            TokenKind::RParen,
            "expected ')' after if-expression condition",
        )?;
        self.skip_newlines();

        // Parse consequent (then branch)
        let consequent = self.parse_expression_prec(Precedence::Lowest, terminator)?;
        self.skip_newlines();

        // Require 'else' for expressions (both branches needed to have a value)
        if !matches!(self.peek_kind(), TokenKind::Keyword(Keyword::Else)) {
            let span = Self::span_from_token(&self.peek().clone());
            self.diagnostics.push_error_with_span(
                "if-expression requires 'else' branch (both branches must produce a value)",
                Some(span),
            );
            bail!("if-expression missing else branch");
        }

        self.expect_keyword(Keyword::Else, "expected 'else' in if-expression")?;
        self.skip_newlines();

        // Parse alternative (else branch)
        let alternative = self.parse_expression_prec(Precedence::Lowest, terminator)?;

        let span = Self::union_spans(&if_span, &alternative.span);
        Ok(Self::make_expression(
            span,
            ExpressionKind::Conditional(ConditionalExpression {
                condition: Box::new(condition),
                consequent: Box::new(consequent),
                alternative: Box::new(alternative),
            }),
        ))
    }

    fn parse_match_arm_block(&mut self) -> Result<MatchArmBlock> {
        let case_token = self.advance().clone();
        let case_span = Self::span_from_token(&case_token);

        let mut patterns = Vec::new();
        loop {
            let pattern = self.parse_match_pattern()?;
            patterns.push(pattern);

            if matches!(self.peek_kind(), TokenKind::Pipe) {
                self.advance();
                continue;
            }
            if matches!(self.peek_kind(), TokenKind::Newline) {
                let mut newline_count = 1usize;
                let mut offset = 1usize;
                while matches!(self.peek_kind_at(offset), Some(TokenKind::Newline)) {
                    newline_count += 1;
                    offset += 1;
                }
                match self.peek_kind_at(offset) {
                    Some(TokenKind::Pipe) => {
                        for _ in 0..newline_count {
                            self.advance();
                        }
                        self.advance();
                        continue;
                    }
                    Some(TokenKind::FatArrow) => {
                        for _ in 0..newline_count {
                            self.advance();
                        }
                        break;
                    }
                    _ => {}
                }
            }
            break;
        }

        if patterns.is_empty() {
            self.diagnostics
                .push_error_with_span("match cases require at least one pattern", Some(case_span));
            bail!("match case missing pattern");
        }

        if matches!(self.peek_kind(), TokenKind::FatArrow) {
            self.advance();
            self.skip_newlines();
            let expression = self.parse_expression_with(terminator_match_arm_expression)?;
            let expr_span = expression.span;
            let arm_span = Self::union_spans(&case_span, &expr_span);

            match self.peek_kind() {
                TokenKind::Newline => {
                    self.skip_newlines();
                }
                TokenKind::Keyword(Keyword::Case)
                | TokenKind::Keyword(Keyword::End)
                | TokenKind::Eof => {}
                other => {
                    let span = Self::span_from_token(&self.peek().clone());
                    self.diagnostics.push_error_with_span(
                        format!(
                            "expected newline after match arm expression (found '{:?}')",
                            other
                        ),
                        Some(span),
                    );
                    bail!("expected newline after match arm");
                }
            }

            let block = Block {
                statements: vec![Statement::Expression(ExpressionStatement { expression })],
            };

            return Ok(MatchArmBlock {
                patterns,
                block,
                span: arm_span,
            });
        }

        self.expect_newline("expected newline before match arm block")?;
        let block = self.parse_block_until(&[Keyword::Case, Keyword::End])?;

        Ok(MatchArmBlock {
            patterns,
            block,
            span: case_span,
        })
    }

    fn parse_match_arm(&mut self) -> Result<MatchArm> {
        let case_token = self.advance().clone();
        let case_span = Self::span_from_token(&case_token);

        let mut patterns = Vec::new();
        let mut _consumed_newline_after_patterns = false;
        loop {
            let pattern = self.parse_match_pattern()?;
            patterns.push(pattern);

            let mut saw_newline = false;
            while matches!(self.peek_kind(), TokenKind::Newline) {
                self.advance();
                saw_newline = true;
            }

            if matches!(self.peek_kind(), TokenKind::Pipe) {
                self.advance();
                _consumed_newline_after_patterns = false;
                continue;
            }

            _consumed_newline_after_patterns = saw_newline;
            break;
        }

        if patterns.is_empty() {
            self.diagnostics
                .push_error_with_span("match cases require at least one pattern", Some(case_span));
            bail!("match case missing pattern");
        }

        self.expect_token(TokenKind::FatArrow, "expected '=>' after match pattern")?;
        self.skip_newlines();
        let expression = self.parse_expression_with(terminator_match_arm_expression)?;
        let arm_span = Self::union_spans(&case_span, &expression.span);

        match self.peek_kind() {
            TokenKind::Newline => {
                self.skip_newlines();
            }
            TokenKind::Keyword(Keyword::End) | TokenKind::Eof => {}
            other => {
                let span = Self::span_from_token(&self.peek().clone());
                self.diagnostics.push_error_with_span(
                    format!(
                        "expected newline after match arm expression (found '{:?}')",
                        other
                    ),
                    Some(span),
                );
                bail!("expected newline after match arm");
            }
        }

        Ok(MatchArm {
            patterns,
            expression,
            span: arm_span,
        })
    }

    fn parse_match_pattern(&mut self) -> Result<MatchPattern> {
        self.skip_newlines();
        if matches!(self.peek_kind(), TokenKind::Pipe | TokenKind::FatArrow) {
            let token = self.peek().clone();
            self.diagnostics.push_error_with_span(
                "expected pattern before '|'",
                Some(Self::span_from_token(&token)),
            );
            bail!("missing pattern");
        }

        if matches!(self.peek_kind(), TokenKind::Keyword(Keyword::Is)) {
            let is_token = self.advance().clone();
            let is_span = Self::span_from_token(&is_token);
            let (type_expression, type_span) =
                self.parse_type_test_target(Precedence::Comparison, terminator_match_pattern)?;
            let span = Self::union_spans(&is_span, &type_span);
            return Ok(MatchPattern::Type(type_expression, span));
        }

        let expression =
            self.parse_expression_prec(Precedence::Lowest, terminator_match_pattern)?;
        self.build_match_pattern(expression)
    }

    fn build_match_pattern(&mut self, expression: Expression) -> Result<MatchPattern> {
        if let ExpressionKind::Identifier(identifier) = &expression.kind {
            if identifier.name == "_" {
                return Ok(MatchPattern::Wildcard {
                    span: expression.span,
                });
            }
        }

        let span = expression.span;
        match expression.kind {
            ExpressionKind::Literal(literal) => Ok(MatchPattern::Expression(Expression {
                span,
                kind: ExpressionKind::Literal(literal),
            })),
            ExpressionKind::Identifier(identifier) => Ok(MatchPattern::Expression(Expression {
                span,
                kind: ExpressionKind::Identifier(identifier),
            })),
            ExpressionKind::Member(member) => Ok(MatchPattern::Expression(Expression {
                span,
                kind: ExpressionKind::Member(member),
            })),
            ExpressionKind::Grouping(inner) => self.build_match_pattern(*inner),
            _ => {
                self.diagnostics.push_error_with_span(
                    "match patterns may only contain literals, identifiers, or enum variants",
                    Some(span),
                );
                bail!("invalid match pattern");
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

    fn parse_anonymous_function(&mut self, start_span: SourceSpan) -> Result<Expression> {
        // Parse parameter list: def(x: Int, y: Int)
        self.expect_token(TokenKind::LParen, "expected '(' after 'def'")?;
        let mut parameters = Vec::new();

        loop {
            self.skip_newlines();
            if matches!(self.peek_kind(), TokenKind::RParen) {
                self.advance();
                break;
            }

            let name_token = self.advance().clone();
            let span = Self::span_from_token(&name_token);

            if !matches!(name_token.kind, TokenKind::Identifier) {
                self.diagnostics.push_error_with_span(
                    format!("expected parameter name, found {:?}", name_token.kind),
                    Some(span),
                );
                bail!("invalid parameter name");
            }

            let name = name_token.lexeme;

            // Parse type annotation (required)
            let type_annotation = if matches!(self.peek_kind(), TokenKind::Colon) {
                self.advance();
                let tokens = self.collect_type_tokens();
                Some(TypeExpression { tokens })
            } else {
                self.diagnostics.push_error_with_span(
                    "anonymous function parameters must have type annotations",
                    Some(span),
                );
                bail!("missing type annotation on parameter");
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
                TokenKind::RParen => {
                    self.advance();
                    break;
                }
                other => {
                    let span = Self::span_from_token(&self.peek().clone());
                    self.diagnostics.push_error_with_span(
                        format!("expected ',' or ')' in parameter list, found {:?}", other),
                        Some(span),
                    );
                    bail!("invalid parameter separator");
                }
            }
        }

        // Parse optional return type annotation
        if matches!(self.peek_kind(), TokenKind::Arrow) {
            self.advance();
            // Skip the return type tokens (we don't store them on lambdas)
            self.collect_type_tokens();
        }

        self.skip_newlines();

        // Parse function body (must be a block ending with 'end')
        let mut statements = Vec::new();
        loop {
            self.skip_newlines();
            if matches!(self.peek_kind(), TokenKind::Keyword(Keyword::End)) {
                let end_token = self.advance().clone();
                let body_span = Self::span_from_token(&end_token);
                let span = Self::union_spans(&start_span, &body_span);
                let id = self.allocate_lambda_id();

                return Ok(Self::make_expression(
                    span,
                    ExpressionKind::Lambda(LambdaExpression {
                        id,
                        parameters,
                        body: LambdaBody::Block(Block { statements }),
                    }),
                ));
            }
            if self.is_at_end() {
                self.diagnostics.push_error_with_span(
                    "unterminated anonymous function, expected 'end'",
                    Some(start_span),
                );
                bail!("unterminated anonymous function");
            }
            statements.push(self.parse_statement()?);
        }
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
            TokenKind::PlusEqual | TokenKind::MinusEqual | TokenKind::StarEqual => {
                // Desugar compound assignments: x += y becomes x = x + y
                let binary_op = match operator_token.kind {
                    TokenKind::PlusEqual => BinaryOperator::Add,
                    TokenKind::MinusEqual => BinaryOperator::Subtract,
                    TokenKind::StarEqual => BinaryOperator::Multiply,
                    _ => unreachable!(),
                };

                let right = self.parse_expression_prec(precedence, terminator)?;
                let right_span = right.span.clone();

                // Create binary expression: left op right
                let binary_expr = Self::make_expression(
                    Self::union_spans(&Self::union_spans(&left.span, &operator_span), &right_span),
                    ExpressionKind::Binary(BinaryExpression {
                        left: Box::new(left.clone()),
                        operator: binary_op,
                        right: Box::new(right),
                    }),
                );

                // Create assignment: left = (left op right)
                let span = Self::union_spans(&left.span, &binary_expr.span);
                Ok(Self::make_expression(
                    span,
                    ExpressionKind::Assignment(AssignmentExpression {
                        target: Box::new(left),
                        value: Box::new(binary_expr),
                    }),
                ))
            }
            TokenKind::DotDot | TokenKind::DotDotDot => {
                // .. is exclusive (doesn't include end), ... is inclusive (includes end)
                let inclusive = matches!(operator_token.kind, TokenKind::DotDotDot);
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
            TokenKind::Keyword(Keyword::Is) => {
                let (type_expression, type_span) =
                    self.parse_type_test_target(precedence, terminator)?;
                let span =
                    Self::union_spans(&Self::union_spans(&left.span, &operator_span), &type_span);
                Ok(Self::make_expression(
                    span,
                    ExpressionKind::Is(IsExpression {
                        value: Box::new(left),
                        type_annotation: type_expression,
                        is_span: operator_span,
                        type_span,
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
            | TokenKind::AmpersandAmpersand
            | TokenKind::PipePipe
            | TokenKind::QuestionQuestion => {
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
                | TokenKind::Bang
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

    fn parse_type_test_target(
        &mut self,
        precedence: Precedence,
        terminator: fn(&TokenKind) -> bool,
    ) -> Result<(TypeExpression, SourceSpan)> {
        let (tokens, span) = self.collect_type_tokens_for_type_test(precedence, terminator);
        if tokens.is_empty() {
            let span = if self.is_at_end() {
                None
            } else {
                Some(Self::span_from_token(&self.peek().clone()))
            };
            self.diagnostics
                .push_error_with_span("expected type after 'is' expression", span);
            bail!("missing type test target");
        }
        Ok((TypeExpression { tokens }, span))
    }

    fn collect_type_tokens_for_type_test(
        &mut self,
        precedence: Precedence,
        terminator: fn(&TokenKind) -> bool,
    ) -> (Vec<Token>, SourceSpan) {
        let mut tokens = Vec::new();
        let mut span = SourceSpan::default();
        let mut depth = 0usize;

        loop {
            if self.is_at_end() {
                break;
            }

            let kind = self.peek_kind();

            if depth == 0 {
                if terminator(kind) || matches!(kind, TokenKind::Newline | TokenKind::Semicolon) {
                    break;
                }
                if matches!(
                    kind,
                    TokenKind::Keyword(Keyword::Else)
                        | TokenKind::Keyword(Keyword::End)
                        | TokenKind::Keyword(Keyword::Case)
                ) {
                    break;
                }
                if let Some(next_prec) = Precedence::of(kind) {
                    if precedence >= next_prec {
                        break;
                    }
                }
            }

            let token = self.advance().clone();
            let token_span = Self::span_from_token(&token);
            if tokens.is_empty() {
                span = token_span;
            } else {
                span = Self::union_spans(&span, &token_span);
            }

            match token.kind {
                TokenKind::LBracket | TokenKind::LParen => {
                    depth += 1;
                }
                TokenKind::RBracket | TokenKind::RParen => {
                    if depth > 0 {
                        depth -= 1;
                    } else {
                        break;
                    }
                }
                _ => {}
            }

            tokens.push(token);
        }

        (tokens, span)
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

    fn finish_catch(
        &mut self,
        expression: Expression,
        terminator: fn(&TokenKind) -> bool,
    ) -> Result<Expression> {
        let catch_token = self.advance().clone();
        let clause = self.parse_catch_clause(catch_token, terminator)?;
        let total_span = Self::union_spans(&expression.span, &clause.span);

        match expression.kind {
            ExpressionKind::Try(mut try_expr) => {
                if try_expr.catch.is_some() {
                    self.diagnostics.push_error_with_span(
                        "try expression already has a catch clause",
                        Some(clause.span),
                    );
                    bail!("duplicate catch clause");
                }
                try_expr.catch = Some(clause);
                Ok(Self::make_expression(
                    total_span,
                    ExpressionKind::Try(try_expr),
                ))
            }
            kind => Ok(Self::make_expression(
                total_span,
                ExpressionKind::Try(TryExpression {
                    expression: Box::new(Expression {
                        span: expression.span,
                        kind,
                    }),
                    catch: Some(clause),
                }),
            )),
        }
    }

    fn parse_catch_clause(
        &mut self,
        catch_token: Token,
        terminator: fn(&TokenKind) -> bool,
    ) -> Result<CatchClause> {
        let catch_span = Self::span_from_token(&catch_token);

        if matches!(self.peek_kind(), TokenKind::Identifier) {
            let binding_token = self.peek().clone();
            let mut lookahead = 1usize;
            while matches!(self.peek_kind_at(lookahead), Some(TokenKind::Newline)) {
                lookahead += 1;
            }

            if matches!(
                self.peek_kind_at(lookahead),
                Some(TokenKind::Keyword(Keyword::Case))
            ) {
                let binding_name = binding_token.lexeme.clone();
                let binding_span = Self::span_from_token(&binding_token);
                self.advance();
                let binding = Identifier {
                    name: binding_name,
                    span: binding_span,
                };
                self.expect_newline("expected newline after catch binding")?;
                let arms = self.parse_catch_arms()?;
                if arms.is_empty() {
                    self.diagnostics.push_error_with_span(
                        "catch block requires at least one case",
                        Some(binding.span),
                    );
                    bail!("missing catch cases");
                }
                let end_token = self.peek().clone();
                self.expect_keyword(Keyword::End, "expected 'end' to close catch block")?;
                let end_span = Self::span_from_token(&end_token);
                let mut span = Self::union_spans(&catch_span, &binding.span);
                for arm in &arms {
                    span = Self::union_spans(&span, &arm.span);
                }
                span = Self::union_spans(&span, &end_span);
                return Ok(CatchClause {
                    binding: Some(binding),
                    kind: CatchKind::Arms(arms),
                    span,
                });
            }
        }

        let expression = self.parse_expression_prec(Precedence::Assignment, terminator)?;
        let span = Self::union_spans(&catch_span, &expression.span);
        Ok(CatchClause {
            binding: None,
            kind: CatchKind::Fallback(Box::new(expression)),
            span,
        })
    }

    fn parse_catch_arms(&mut self) -> Result<Vec<CatchArm>> {
        let mut arms = Vec::new();
        loop {
            self.skip_newlines();
            match self.peek_kind() {
                TokenKind::Keyword(Keyword::Case) => {
                    let arm = self.parse_catch_arm()?;
                    arms.push(arm);
                }
                TokenKind::Keyword(Keyword::End) => break,
                TokenKind::Eof => {
                    self.diagnostics
                        .push_error_with_span("unterminated catch block", None);
                    bail!("unterminated catch block");
                }
                other => {
                    let token = self.peek().clone();
                    self.diagnostics.push_error_with_span(
                        format!("unexpected token {:?} in catch block", other),
                        Some(Self::span_from_token(&token)),
                    );
                    bail!("invalid catch block");
                }
            }
        }
        Ok(arms)
    }

    fn parse_catch_arm(&mut self) -> Result<CatchArm> {
        let case_token = self.advance().clone();
        let case_span = Self::span_from_token(&case_token);

        let mut patterns = Vec::new();
        let consumed_newline_after_patterns = loop {
            let pattern = self.parse_match_pattern()?;
            patterns.push(pattern);

            let mut saw_newline = false;
            while matches!(self.peek_kind(), TokenKind::Newline) {
                self.advance();
                saw_newline = true;
            }

            if matches!(self.peek_kind(), TokenKind::Pipe) {
                self.advance();
                continue;
            }

            break saw_newline;
        };

        if patterns.is_empty() {
            self.diagnostics
                .push_error_with_span("catch cases require at least one pattern", Some(case_span));
            bail!("catch case missing pattern");
        }

        if matches!(self.peek_kind(), TokenKind::FatArrow) {
            self.advance();
            self.skip_newlines();
            let expression = self.parse_expression_with(terminator_match_arm_expression)?;
            let arm_span = Self::union_spans(&case_span, &expression.span);

            match self.peek_kind() {
                TokenKind::Newline => {
                    self.skip_newlines();
                }
                TokenKind::Keyword(Keyword::Case)
                | TokenKind::Keyword(Keyword::End)
                | TokenKind::Eof => {}
                other => {
                    let span = Self::span_from_token(&self.peek().clone());
                    self.diagnostics.push_error_with_span(
                        format!(
                            "expected newline after catch arm expression (found '{:?}')",
                            other
                        ),
                        Some(span),
                    );
                    bail!("expected newline after catch arm");
                }
            }

            return Ok(CatchArm {
                patterns,
                handler: CatchHandler::Expression(expression),
                span: arm_span,
            });
        }

        if !consumed_newline_after_patterns {
            self.expect_newline("expected newline before catch arm block")?;
        }
        let block = self.parse_block_until(&[Keyword::Case, Keyword::End])?;

        Ok(CatchArm {
            patterns,
            handler: CatchHandler::Block(block),
            span: case_span,
        })
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

fn terminator_rparen(kind: &TokenKind) -> bool {
    matches!(kind, TokenKind::RParen)
}

fn terminator_match_pattern(kind: &TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Pipe
            | TokenKind::FatArrow
            | TokenKind::Newline
            | TokenKind::Semicolon
            | TokenKind::Eof
    )
}

fn terminator_match_arm_expression(kind: &TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Newline
            | TokenKind::Semicolon
            | TokenKind::Eof
            | TokenKind::Keyword(Keyword::Case)
            | TokenKind::Keyword(Keyword::End)
    )
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
        TokenKind::AmpersandAmpersand => BinaryOperator::And,
        TokenKind::PipePipe => BinaryOperator::Or,
        TokenKind::QuestionQuestion => BinaryOperator::Coalesce,
        other => bail!("unsupported binary operator {:?}", other),
    };
    Ok(operator)
}
