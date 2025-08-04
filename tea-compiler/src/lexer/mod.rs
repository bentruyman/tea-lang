use anyhow::{bail, Context, Result};

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
    Eof,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Keyword {
    Var,
    Const,
    Def,
    Test,
    If,
    Unless,
    End,
    For,
    Of,
    While,
    Until,
    Return,
    Use,
    Struct,
    Else,
    And,
    Or,
    Not,
    In,
    Nil,
}

pub struct Lexer<'a> {
    input: &'a str,
    position: usize,
    line: usize,
    column: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a SourceFile) -> Result<Self> {
        Ok(Self {
            input: &source.contents,
            position: 0,
            line: 1,
            column: 1,
        })
    }

    pub fn tokenize(&mut self) -> Result<Vec<Token>> {
        let mut tokens = Vec::new();

        while let Some(ch) = self.peek_char() {
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
                    self.skip_comment();
                }
                '"' => {
                    let token = self.lex_string()?;
                    tokens.push(token);
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
                '*' => tokens.push(self.simple_token(TokenKind::Star)),
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
                '?' => tokens.push(self.simple_token(TokenKind::Question)),
                other => {
                    bail!(
                        "Unexpected character '{}' at line {}, column {}",
                        other,
                        self.line,
                        self.column
                    );
                }
            }
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
                    self.advance_char();
                    let escaped = self
                        .peek_char()
                        .context("Unterminated escape sequence in string literal")?;
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
                    bail!(
                        "Unterminated string literal starting at line {}, column {}",
                        start_line,
                        start_column
                    );
                }
                _ => {
                    value.push(ch);
                    self.advance_char();
                }
            }
        }

        bail!(
            "Unterminated string literal starting at line {}, column {}",
            start_line,
            start_column
        );
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

        let lexeme = self.slice(start, self.position).replace('_', "");
        if is_float {
            let value = lexeme.parse::<f64>().with_context(|| {
                format!(
                    "Failed to parse float literal '{}' at line {}, column {}",
                    lexeme, start_line, start_column
                )
            })?;
            Ok(Token::new(
                TokenKind::FloatLiteral(value),
                self.slice(start, self.position).to_string(),
                start_line,
                start_column,
            ))
        } else {
            let value = lexeme.parse::<i64>().with_context(|| {
                format!(
                    "Failed to parse integer literal '{}' at line {}, column {}",
                    lexeme, start_line, start_column
                )
            })?;
            Ok(Token::new(
                TokenKind::IntegerLiteral(value),
                self.slice(start, self.position).to_string(),
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
        } else {
            Token::new(
                TokenKind::Minus,
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
        "test" => Some(Keyword::Test),
        "if" => Some(Keyword::If),
        "unless" => Some(Keyword::Unless),
        "end" => Some(Keyword::End),
        "for" => Some(Keyword::For),
        "of" => Some(Keyword::Of),
        "while" => Some(Keyword::While),
        "until" => Some(Keyword::Until),
        "return" => Some(Keyword::Return),
        "use" => Some(Keyword::Use),
        "struct" => Some(Keyword::Struct),
        "else" => Some(Keyword::Else),
        "and" => Some(Keyword::And),
        "or" => Some(Keyword::Or),
        "not" => Some(Keyword::Not),
        "in" => Some(Keyword::In),
        "nil" => Some(Keyword::Nil),
        _ => None,
    }
}
