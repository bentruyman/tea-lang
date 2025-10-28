use std::collections::VecDeque;
use std::num::{ParseFloatError, ParseIntError};

use anyhow::Result;
use thiserror::Error;

use crate::source::SourceFile;

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub lexeme: String,
    pub line: usize,
    pub column: usize,
}

impl Token {
    fn new(kind: TokenKind, lexeme: String, line: usize, column: usize) -> Self {
        Self {
            kind,
            lexeme,
            line,
            column,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Identifier,
    IntegerLiteral(i64),
    FloatLiteral(f64),
    StringLiteral(String),
    InterpolatedStringStart,
    InterpolatedStringSegment(String),
    InterpolatedStringExprStart,
    InterpolatedStringExprEnd,
    InterpolatedStringEnd,
    DocComment(String),
    BooleanLiteral(bool),
    Keyword(Keyword),
    Newline,
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Comma,
    Dot,
    DotDot,
    DotDotDot,
    Colon,
    Semicolon,
    Equal,
    PlusEqual,
    MinusEqual,
    StarEqual,
    DoubleEqual,
    Bang,
    BangEqual,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,
    Plus,
    PlusPlus,
    Minus,
    MinusMinus,
    Star,
    Slash,
    Percent,
    Pipe,
    PipePipe,
    Arrow,    // ->
    FatArrow, // =>
    Ampersand,
    AmpersandAmpersand,
    Question,
    QuestionQuestion,
    Eof,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Keyword {
    Var,
    Const,
    Def,
    Pub,
    Test,
    Match,
    Case,
    If,
    End,
    Error,
    Try,
    Catch,
    Throw,
    For,
    Of,
    While,
    Break,
    Continue,
    Return,
    Use,
    Struct,
    Union,
    Enum,
    Else,
    Not,
    In,
    Is,
    Nil,
}

pub struct Lexer<'a> {
    input: &'a str,
    position: usize,
    line: usize,
    column: usize,
    pending_tokens: VecDeque<Token>,
    template_stack: Vec<TemplateContext>,
}

#[derive(Debug, Clone)]
enum TemplateState {
    Literal,
    Expression { brace_depth: usize },
}

#[derive(Debug, Clone)]
struct TemplateContext {
    start_position: usize,
    start_line: usize,
    start_column: usize,
    segment_start_position: usize,
    segment_start_line: usize,
    segment_start_column: usize,
    state: TemplateState,
}

#[derive(Debug, Error)]
pub enum LexerError {
    #[error("Unexpected character '{ch}' at line {line}, column {column}")]
    UnexpectedCharacter {
        ch: char,
        line: usize,
        column: usize,
    },
    #[error("Unterminated string literal starting at line {line}, column {column}")]
    UnterminatedStringLiteral { line: usize, column: usize },
    #[error("Unterminated escape sequence in string literal at line {line}, column {column}")]
    UnterminatedEscapeSequence { line: usize, column: usize },
    #[error(
        "Failed to parse integer literal '{lexeme}' at line {line}, column {column}: {source}"
    )]
    IntegerParse {
        lexeme: String,
        line: usize,
        column: usize,
        #[source]
        source: ParseIntError,
    },
    #[error("Failed to parse float literal '{lexeme}' at line {line}, column {column}: {source}")]
    FloatParse {
        lexeme: String,
        line: usize,
        column: usize,
        #[source]
        source: ParseFloatError,
    },
}

impl LexerError {
    pub fn line(&self) -> usize {
        match self {
            LexerError::UnexpectedCharacter { line, .. }
            | LexerError::UnterminatedStringLiteral { line, .. }
            | LexerError::UnterminatedEscapeSequence { line, .. }
            | LexerError::IntegerParse { line, .. }
            | LexerError::FloatParse { line, .. } => *line,
        }
    }

    pub fn column(&self) -> usize {
        match self {
            LexerError::UnexpectedCharacter { column, .. }
            | LexerError::UnterminatedStringLiteral { column, .. }
            | LexerError::UnterminatedEscapeSequence { column, .. }
            | LexerError::IntegerParse { column, .. }
            | LexerError::FloatParse { column, .. } => *column,
        }
    }
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a SourceFile) -> Result<Self> {
        Ok(Self {
            input: &source.contents,
            position: 0,
            line: 1,
            column: 1,
            pending_tokens: VecDeque::new(),
            template_stack: Vec::new(),
        })
    }

    pub fn tokenize(&mut self) -> Result<Vec<Token>> {
        let mut tokens = Vec::new();

        while self.peek_char().is_some() || !self.pending_tokens.is_empty() {
            if let Some(token) = self.pending_tokens.pop_front() {
                tokens.push(token);
                continue;
            }

            if self
                .template_stack
                .last()
                .map(|ctx| matches!(ctx.state, TemplateState::Literal))
                .unwrap_or(false)
            {
                self.lex_interpolated_string_segment()?;
                continue;
            }

            let ch = match self.peek_char() {
                Some(ch) => ch,
                None => break,
            };

            if self.try_handle_interpolated_expression_char(ch)? {
                continue;
            }

            match ch {
                ' ' | '\t' => {
                    self.advance_char();
                }
                '\r' => {
                    self.advance_char();
                    if self.peek_char() == Some('\n') {
                        self.advance_char();
                    }
                    tokens.push(self.make_newline_token());
                }
                '\n' => {
                    self.advance_char();
                    tokens.push(self.make_newline_token());
                }
                '#' => {
                    if matches!(self.peek_next_char(), Some('#')) {
                        let token = self.lex_doc_comment();
                        tokens.push(token);
                    } else {
                        self.skip_comment();
                    }
                }
                '"' => {
                    let token = self.lex_string()?;
                    tokens.push(token);
                }
                '`' => {
                    self.begin_interpolated_string();
                }
                '0'..='9' => {
                    let token = self.lex_number()?;
                    tokens.push(token);
                }
                'a'..='z' | 'A'..='Z' | '_' => {
                    let token = self.lex_identifier_or_keyword()?;
                    tokens.push(token);
                }
                '(' => tokens.push(self.simple_token(TokenKind::LParen)),
                ')' => tokens.push(self.simple_token(TokenKind::RParen)),
                '{' => tokens.push(self.simple_token(TokenKind::LBrace)),
                '}' => tokens.push(self.simple_token(TokenKind::RBrace)),
                '[' => tokens.push(self.simple_token(TokenKind::LBracket)),
                ']' => tokens.push(self.simple_token(TokenKind::RBracket)),
                ',' => tokens.push(self.simple_token(TokenKind::Comma)),
                ';' => tokens.push(self.simple_token(TokenKind::Semicolon)),
                ':' => tokens.push(self.simple_token(TokenKind::Colon)),
                '.' => {
                    let token = self.lex_dot_variants();
                    tokens.push(token);
                }
                '=' => {
                    let token = self.lex_equals_variants();
                    tokens.push(token);
                }
                '!' => {
                    let token = self.lex_bang_variants();
                    tokens.push(token);
                }
                '>' => {
                    let token = self.lex_greater_variants();
                    tokens.push(token);
                }
                '<' => {
                    let token = self.lex_less_variants();
                    tokens.push(token);
                }
                '+' => {
                    let token = self.lex_plus_variants();
                    tokens.push(token);
                }
                '-' => {
                    let token = self.lex_minus_variants();
                    tokens.push(token);
                }
                '*' => {
                    let token = self.lex_star_variants();
                    tokens.push(token);
                }
                '/' => tokens.push(self.simple_token(TokenKind::Slash)),
                '%' => tokens.push(self.simple_token(TokenKind::Percent)),
                '|' => {
                    let token = self.lex_pipe_variants();
                    tokens.push(token);
                }
                '&' => {
                    let token = self.lex_ampersand_variants();
                    tokens.push(token);
                }
                '?' => {
                    let token = self.lex_question_variants();
                    tokens.push(token);
                }
                other => {
                    return Err(LexerError::UnexpectedCharacter {
                        ch: other,
                        line: self.line,
                        column: self.column,
                    }
                    .into());
                }
            }
        }

        if let Some(context) = self.template_stack.last() {
            return Err(LexerError::UnterminatedStringLiteral {
                line: context.start_line,
                column: context.start_column,
            }
            .into());
        }

        tokens.push(Token::new(
            TokenKind::Eof,
            String::new(),
            self.line,
            self.column,
        ));

        Ok(tokens)
    }

    fn make_newline_token(&self) -> Token {
        Token::new(
            TokenKind::Newline,
            "\n".to_string(),
            self.line.saturating_sub(1),
            1,
        )
    }

    fn skip_comment(&mut self) {
        while let Some(ch) = self.peek_char() {
            if ch == '\n' {
                break;
            }
            self.advance_char();
        }
    }

    fn lex_doc_comment(&mut self) -> Token {
        let start = self.position;
        let start_line = self.line;
        let start_column = self.column;

        // consume the leading ##
        self.advance_char();
        self.advance_char();

        if let Some(peeked) = self.peek_char() {
            if peeked == ' ' || peeked == '\t' {
                self.advance_char();
            }
        }

        let content_start = self.position;

        while let Some(ch) = self.peek_char() {
            if ch == '\n' || ch == '\r' {
                break;
            }
            self.advance_char();
        }

        let raw_lexeme = self.slice(start, self.position).to_string();
        let content = self
            .slice(content_start, self.position)
            .trim_end()
            .to_string();

        Token::new(
            TokenKind::DocComment(content),
            raw_lexeme,
            start_line,
            start_column,
        )
    }

    fn begin_interpolated_string(&mut self) {
        let start_position = self.position;
        let start_line = self.line;
        let start_column = self.column;
        self.advance_char();
        let lexeme = self.slice(start_position, self.position).to_string();
        self.pending_tokens.push_back(Token::new(
            TokenKind::InterpolatedStringStart,
            lexeme,
            start_line,
            start_column,
        ));
        self.template_stack.push(TemplateContext {
            start_position,
            start_line,
            start_column,
            segment_start_position: self.position,
            segment_start_line: self.line,
            segment_start_column: self.column,
            state: TemplateState::Literal,
        });
    }

    fn lex_interpolated_string_segment(&mut self) -> Result<()> {
        if self.template_stack.is_empty() {
            return Ok(());
        }
        let idx = self.template_stack.len() - 1;
        let (
            _start_position,
            start_line,
            start_column,
            segment_start_position,
            segment_start_line,
            segment_start_column,
        ) = {
            let ctx = &self.template_stack[idx];
            (
                ctx.start_position,
                ctx.start_line,
                ctx.start_column,
                ctx.segment_start_position,
                ctx.segment_start_line,
                ctx.segment_start_column,
            )
        };
        let mut value = String::new();

        loop {
            let ch = match self.peek_char() {
                Some(ch) => ch,
                None => {
                    return Err(LexerError::UnterminatedStringLiteral {
                        line: start_line,
                        column: start_column,
                    }
                    .into())
                }
            };

            match ch {
                '`' => {
                    if !value.is_empty() {
                        let raw_lexeme = self.slice(segment_start_position, self.position);
                        self.pending_tokens.push_back(Token::new(
                            TokenKind::InterpolatedStringSegment(std::mem::take(&mut value)),
                            raw_lexeme.to_string(),
                            segment_start_line,
                            segment_start_column,
                        ));
                    }
                    let end_line = self.line;
                    let end_column = self.column;
                    let closing_start = self.position;
                    self.advance_char();
                    let lexeme = self.slice(closing_start, self.position).to_string();
                    self.pending_tokens.push_back(Token::new(
                        TokenKind::InterpolatedStringEnd,
                        lexeme,
                        end_line,
                        end_column,
                    ));
                    self.template_stack.pop();
                    return Ok(());
                }
                '$' if matches!(self.peek_next_char(), Some('{')) => {
                    if !value.is_empty() {
                        let raw_lexeme = self.slice(segment_start_position, self.position);
                        self.pending_tokens.push_back(Token::new(
                            TokenKind::InterpolatedStringSegment(std::mem::take(&mut value)),
                            raw_lexeme.to_string(),
                            segment_start_line,
                            segment_start_column,
                        ));
                    }
                    let expr_start_line = self.line;
                    let expr_start_column = self.column;
                    let expr_start_position = self.position;
                    self.advance_char(); // consume $
                    self.advance_char(); // consume {
                    let raw = self.slice(expr_start_position, self.position).to_string();
                    self.pending_tokens.push_back(Token::new(
                        TokenKind::InterpolatedStringExprStart,
                        raw,
                        expr_start_line,
                        expr_start_column,
                    ));
                    if let Some(ctx) = self.template_stack.get_mut(idx) {
                        ctx.state = TemplateState::Expression { brace_depth: 0 };
                        ctx.segment_start_position = self.position;
                        ctx.segment_start_line = self.line;
                        ctx.segment_start_column = self.column;
                    }
                    return Ok(());
                }
                '\\' => {
                    let escape_line = self.line;
                    let escape_column = self.column;
                    self.advance_char();
                    let escaped = match self.peek_char() {
                        Some(ch) => ch,
                        None => {
                            return Err(LexerError::UnterminatedEscapeSequence {
                                line: escape_line,
                                column: escape_column,
                            }
                            .into());
                        }
                    };
                    let escaped_char = match escaped {
                        '`' => '`',
                        '$' => '$',
                        '\\' => '\\',
                        'n' => '\n',
                        'r' => '\r',
                        't' => '\t',
                        other => other,
                    };
                    value.push(escaped_char);
                    self.advance_char();
                }
                '\r' => {
                    self.advance_char();
                    value.push('\n');
                }
                '\n' => {
                    self.advance_char();
                    value.push('\n');
                }
                _ => {
                    value.push(ch);
                    self.advance_char();
                }
            }
        }
    }

    fn try_handle_interpolated_expression_char(&mut self, ch: char) -> Result<bool> {
        let len = self.template_stack.len();
        if len == 0 {
            return Ok(false);
        }
        let idx = len - 1;
        let is_expression = matches!(
            self.template_stack[idx].state,
            TemplateState::Expression { .. }
        );
        if !is_expression {
            return Ok(false);
        }

        match ch {
            '{' => {
                let token = self.simple_token(TokenKind::LBrace);
                self.pending_tokens.push_back(token);
                if let TemplateState::Expression {
                    ref mut brace_depth,
                } = self.template_stack[idx].state
                {
                    *brace_depth += 1;
                }
                Ok(true)
            }
            '}' => {
                let brace_depth = match self.template_stack[idx].state {
                    TemplateState::Expression { brace_depth } => brace_depth,
                    _ => return Ok(false),
                };

                if brace_depth == 0 {
                    let end_line = self.line;
                    let end_column = self.column;
                    let start = self.position;
                    self.advance_char();
                    let lexeme = self.slice(start, self.position).to_string();
                    self.pending_tokens.push_back(Token::new(
                        TokenKind::InterpolatedStringExprEnd,
                        lexeme,
                        end_line,
                        end_column,
                    ));
                    if let Some(ctx) = self.template_stack.get_mut(idx) {
                        ctx.state = TemplateState::Literal;
                        ctx.segment_start_position = self.position;
                        ctx.segment_start_line = self.line;
                        ctx.segment_start_column = self.column;
                    }
                    Ok(true)
                } else {
                    let token = self.simple_token(TokenKind::RBrace);
                    self.pending_tokens.push_back(token);
                    if let Some(ctx) = self.template_stack.get_mut(idx) {
                        if let TemplateState::Expression {
                            ref mut brace_depth,
                        } = ctx.state
                        {
                            *brace_depth -= 1;
                        }
                    }
                    Ok(true)
                }
            }
            _ => Ok(false),
        }
    }

    fn lex_string(&mut self) -> Result<Token> {
        let start = self.position;
        let start_line = self.line;
        let start_column = self.column;
        self.advance_char(); // consume opening quote

        let mut value = String::new();
        while let Some(ch) = self.peek_char() {
            match ch {
                '"' => {
                    self.advance_char(); // consume closing quote
                    let lexeme = self.slice(start, self.position);
                    return Ok(Token::new(
                        TokenKind::StringLiteral(value),
                        lexeme.to_string(),
                        start_line,
                        start_column,
                    ));
                }
                '\\' => {
                    let escape_line = self.line;
                    let escape_column = self.column;
                    self.advance_char();
                    let escaped = match self.peek_char() {
                        Some(ch) => ch,
                        None => {
                            return Err(LexerError::UnterminatedEscapeSequence {
                                line: escape_line,
                                column: escape_column,
                            }
                            .into());
                        }
                    };
                    let escaped_char = match escaped {
                        '"' => '"',
                        '\\' => '\\',
                        'n' => '\n',
                        'r' => '\r',
                        't' => '\t',
                        other => other,
                    };
                    value.push(escaped_char);
                    self.advance_char();
                }
                '\n' => {
                    return Err(LexerError::UnterminatedStringLiteral {
                        line: start_line,
                        column: start_column,
                    }
                    .into());
                }
                _ => {
                    value.push(ch);
                    self.advance_char();
                }
            }
        }

        Err(LexerError::UnterminatedStringLiteral {
            line: start_line,
            column: start_column,
        }
        .into())
    }

    fn lex_number(&mut self) -> Result<Token> {
        let start = self.position;
        let start_line = self.line;
        let start_column = self.column;
        let mut is_float = false;

        self.advance_char(); // consume first digit

        while let Some(ch) = self.peek_char() {
            match ch {
                '0'..='9' => {
                    self.advance_char();
                }
                '_' => {
                    self.advance_char();
                }
                '.' => {
                    if is_float {
                        break;
                    }
                    if matches!(self.peek_next_char(), Some('.') | None) {
                        break;
                    }
                    is_float = true;
                    self.advance_char();
                }
                _ => break,
            }
        }

        let raw_lexeme = self.slice(start, self.position).to_string();
        let lexeme = raw_lexeme.replace('_', "");
        if is_float {
            let value = lexeme
                .parse::<f64>()
                .map_err(|source| LexerError::FloatParse {
                    lexeme: raw_lexeme.clone(),
                    line: start_line,
                    column: start_column,
                    source,
                })?;
            Ok(Token::new(
                TokenKind::FloatLiteral(value),
                raw_lexeme,
                start_line,
                start_column,
            ))
        } else {
            let value = lexeme
                .parse::<i64>()
                .map_err(|source| LexerError::IntegerParse {
                    lexeme: raw_lexeme.clone(),
                    line: start_line,
                    column: start_column,
                    source,
                })?;
            Ok(Token::new(
                TokenKind::IntegerLiteral(value),
                raw_lexeme,
                start_line,
                start_column,
            ))
        }
    }

    fn lex_identifier_or_keyword(&mut self) -> Result<Token> {
        let start = self.position;
        let start_line = self.line;
        let start_column = self.column;
        self.advance_char();

        while let Some(ch) = self.peek_char() {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                self.advance_char();
            } else {
                break;
            }
        }

        let lexeme = self.slice(start, self.position).to_string();
        if let Some(keyword) = keyword_from_lexeme(&lexeme) {
            Ok(Token::new(
                TokenKind::Keyword(keyword),
                lexeme,
                start_line,
                start_column,
            ))
        } else if lexeme == "true" {
            Ok(Token::new(
                TokenKind::BooleanLiteral(true),
                lexeme,
                start_line,
                start_column,
            ))
        } else if lexeme == "false" {
            Ok(Token::new(
                TokenKind::BooleanLiteral(false),
                lexeme,
                start_line,
                start_column,
            ))
        } else {
            Ok(Token::new(
                TokenKind::Identifier,
                lexeme,
                start_line,
                start_column,
            ))
        }
    }

    fn lex_dot_variants(&mut self) -> Token {
        let start_line = self.line;
        let start_column = self.column;
        let start = self.position;
        self.advance_char(); // consume '.'

        if self.peek_char() == Some('.') {
            self.advance_char();
            if self.peek_char() == Some('.') {
                self.advance_char();
                Token::new(
                    TokenKind::DotDotDot,
                    self.slice(start, self.position).to_string(),
                    start_line,
                    start_column,
                )
            } else {
                Token::new(
                    TokenKind::DotDot,
                    self.slice(start, self.position).to_string(),
                    start_line,
                    start_column,
                )
            }
        } else {
            Token::new(
                TokenKind::Dot,
                self.slice(start, self.position).to_string(),
                start_line,
                start_column,
            )
        }
    }

    fn lex_equals_variants(&mut self) -> Token {
        let start_line = self.line;
        let start_column = self.column;
        let start = self.position;
        self.advance_char(); // consume '='

        if self.peek_char() == Some('=') {
            self.advance_char();
            Token::new(
                TokenKind::DoubleEqual,
                self.slice(start, self.position).to_string(),
                start_line,
                start_column,
            )
        } else if self.peek_char() == Some('>') {
            self.advance_char();
            Token::new(
                TokenKind::FatArrow,
                self.slice(start, self.position).to_string(),
                start_line,
                start_column,
            )
        } else {
            Token::new(
                TokenKind::Equal,
                self.slice(start, self.position).to_string(),
                start_line,
                start_column,
            )
        }
    }

    fn lex_bang_variants(&mut self) -> Token {
        let start_line = self.line;
        let start_column = self.column;
        let start = self.position;
        self.advance_char(); // consume '!'

        if self.peek_char() == Some('=') {
            self.advance_char();
            Token::new(
                TokenKind::BangEqual,
                self.slice(start, self.position).to_string(),
                start_line,
                start_column,
            )
        } else {
            Token::new(
                TokenKind::Bang,
                self.slice(start, self.position).to_string(),
                start_line,
                start_column,
            )
        }
    }

    fn lex_greater_variants(&mut self) -> Token {
        let start_line = self.line;
        let start_column = self.column;
        let start = self.position;
        self.advance_char(); // consume '>'

        if self.peek_char() == Some('=') {
            self.advance_char();
            Token::new(
                TokenKind::GreaterEqual,
                self.slice(start, self.position).to_string(),
                start_line,
                start_column,
            )
        } else {
            Token::new(
                TokenKind::Greater,
                self.slice(start, self.position).to_string(),
                start_line,
                start_column,
            )
        }
    }

    fn lex_less_variants(&mut self) -> Token {
        let start_line = self.line;
        let start_column = self.column;
        let start = self.position;
        self.advance_char(); // consume '<'

        if self.peek_char() == Some('=') {
            self.advance_char();
            Token::new(
                TokenKind::LessEqual,
                self.slice(start, self.position).to_string(),
                start_line,
                start_column,
            )
        } else {
            Token::new(
                TokenKind::Less,
                self.slice(start, self.position).to_string(),
                start_line,
                start_column,
            )
        }
    }

    fn lex_plus_variants(&mut self) -> Token {
        let start_line = self.line;
        let start_column = self.column;
        let start = self.position;
        self.advance_char(); // consume '+'

        if self.peek_char() == Some('+') {
            self.advance_char();
            Token::new(
                TokenKind::PlusPlus,
                self.slice(start, self.position).to_string(),
                start_line,
                start_column,
            )
        } else if self.peek_char() == Some('=') {
            self.advance_char();
            Token::new(
                TokenKind::PlusEqual,
                self.slice(start, self.position).to_string(),
                start_line,
                start_column,
            )
        } else {
            Token::new(
                TokenKind::Plus,
                self.slice(start, self.position).to_string(),
                start_line,
                start_column,
            )
        }
    }

    fn lex_minus_variants(&mut self) -> Token {
        let start_line = self.line;
        let start_column = self.column;
        let start = self.position;
        self.advance_char(); // consume '-'

        if self.peek_char() == Some('-') {
            self.advance_char();
            Token::new(
                TokenKind::MinusMinus,
                self.slice(start, self.position).to_string(),
                start_line,
                start_column,
            )
        } else if self.peek_char() == Some('>') {
            self.advance_char();
            Token::new(
                TokenKind::Arrow,
                self.slice(start, self.position).to_string(),
                start_line,
                start_column,
            )
        } else if self.peek_char() == Some('=') {
            self.advance_char();
            Token::new(
                TokenKind::MinusEqual,
                self.slice(start, self.position).to_string(),
                start_line,
                start_column,
            )
        } else {
            Token::new(
                TokenKind::Minus,
                self.slice(start, self.position).to_string(),
                start_line,
                start_column,
            )
        }
    }

    fn lex_star_variants(&mut self) -> Token {
        let start_line = self.line;
        let start_column = self.column;
        let start = self.position;
        self.advance_char(); // consume '*'

        if self.peek_char() == Some('=') {
            self.advance_char();
            Token::new(
                TokenKind::StarEqual,
                self.slice(start, self.position).to_string(),
                start_line,
                start_column,
            )
        } else {
            Token::new(
                TokenKind::Star,
                self.slice(start, self.position).to_string(),
                start_line,
                start_column,
            )
        }
    }

    fn lex_question_variants(&mut self) -> Token {
        let start_line = self.line;
        let start_column = self.column;
        let start = self.position;
        self.advance_char(); // consume '?'

        if self.peek_char() == Some('?') {
            self.advance_char();
            Token::new(
                TokenKind::QuestionQuestion,
                self.slice(start, self.position).to_string(),
                start_line,
                start_column,
            )
        } else {
            Token::new(
                TokenKind::Question,
                self.slice(start, self.position).to_string(),
                start_line,
                start_column,
            )
        }
    }

    fn lex_pipe_variants(&mut self) -> Token {
        let start_line = self.line;
        let start_column = self.column;
        let start = self.position;
        self.advance_char(); // consume '|'

        if self.peek_char() == Some('|') {
            self.advance_char();
            Token::new(
                TokenKind::PipePipe,
                self.slice(start, self.position).to_string(),
                start_line,
                start_column,
            )
        } else {
            Token::new(
                TokenKind::Pipe,
                self.slice(start, self.position).to_string(),
                start_line,
                start_column,
            )
        }
    }

    fn lex_ampersand_variants(&mut self) -> Token {
        let start_line = self.line;
        let start_column = self.column;
        let start = self.position;
        self.advance_char(); // consume '&'

        if self.peek_char() == Some('&') {
            self.advance_char();
            Token::new(
                TokenKind::AmpersandAmpersand,
                self.slice(start, self.position).to_string(),
                start_line,
                start_column,
            )
        } else {
            Token::new(
                TokenKind::Ampersand,
                self.slice(start, self.position).to_string(),
                start_line,
                start_column,
            )
        }
    }

    fn simple_token(&mut self, kind: TokenKind) -> Token {
        let start_line = self.line;
        let start_column = self.column;
        let start = self.position;
        self.advance_char();
        Token::new(
            kind,
            self.slice(start, self.position).to_string(),
            start_line,
            start_column,
        )
    }

    fn peek_char(&self) -> Option<char> {
        self.input[self.position..].chars().next()
    }

    fn peek_next_char(&self) -> Option<char> {
        let mut iter = self.input[self.position..].chars();
        iter.next()?;
        iter.next()
    }

    fn advance_char(&mut self) -> Option<char> {
        let ch = self.peek_char()?;
        self.position += ch.len_utf8();
        if ch == '\r' || ch == '\n' {
            if ch == '\r' && self.peek_char() == Some('\n') {
                let next = self.peek_char().unwrap();
                self.position += next.len_utf8();
            }
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        Some(ch)
    }

    fn slice(&self, start: usize, end: usize) -> &str {
        &self.input[start..end]
    }
}

fn keyword_from_lexeme(lexeme: &str) -> Option<Keyword> {
    match lexeme {
        "var" => Some(Keyword::Var),
        "const" => Some(Keyword::Const),
        "def" => Some(Keyword::Def),
        "pub" => Some(Keyword::Pub),
        "test" => Some(Keyword::Test),
        "match" => Some(Keyword::Match),
        "case" => Some(Keyword::Case),
        "if" => Some(Keyword::If),
        "end" => Some(Keyword::End),
        "error" => Some(Keyword::Error),
        "try" => Some(Keyword::Try),
        "catch" => Some(Keyword::Catch),
        "throw" => Some(Keyword::Throw),
        "for" => Some(Keyword::For),
        "of" => Some(Keyword::Of),
        "while" => Some(Keyword::While),
        "break" => Some(Keyword::Break),
        "continue" => Some(Keyword::Continue),
        "return" => Some(Keyword::Return),
        "use" => Some(Keyword::Use),
        "struct" => Some(Keyword::Struct),
        "union" => Some(Keyword::Union),
        "enum" => Some(Keyword::Enum),
        "else" => Some(Keyword::Else),
        "not" => Some(Keyword::Not),
        "in" => Some(Keyword::In),
        "is" => Some(Keyword::Is),
        "nil" => Some(Keyword::Nil),
        _ => None,
    }
}
