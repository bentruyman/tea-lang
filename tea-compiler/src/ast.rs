use crate::lexer::Token;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SourceSpan {
    pub line: usize,
    pub column: usize,
    pub end_line: usize,
    pub end_column: usize,
}

impl SourceSpan {
    pub fn new(line: usize, column: usize, end_line: usize, end_column: usize) -> Self {
        Self {
            line,
            column,
            end_line,
            end_column,
        }
    }

    pub fn single_point(line: usize, column: usize) -> Self {
        Self::new(line, column, line, column)
    }

    pub fn union(a: &Self, b: &Self) -> Self {
        if a.line == 0 {
            return *b;
        }
        if b.line == 0 {
            return *a;
        }

        let (start_line, start_column) =
            if (a.line < b.line) || (a.line == b.line && a.column <= b.column) {
                (a.line, a.column)
            } else {
                (b.line, b.column)
            };

        let (end_line, end_column) = if (a.end_line > b.end_line)
            || (a.end_line == b.end_line && a.end_column >= b.end_column)
        {
            (a.end_line, a.end_column)
        } else {
            (b.end_line, b.end_column)
        };

        Self::new(start_line, start_column, end_line, end_column)
    }
}

impl Default for SourceSpan {
    fn default() -> Self {
        Self {
            line: 0,
            column: 0,
            end_line: 0,
            end_column: 0,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Module {
    pub statements: Vec<Statement>,
}

impl Module {
    pub fn new(statements: Vec<Statement>) -> Self {
        Self { statements }
    }
}

#[derive(Debug, Clone)]
pub enum Statement {
    Use(UseStatement),
    Var(VarStatement),
    Function(FunctionStatement),
    Test(TestStatement),
    Struct(StructStatement),
    Conditional(ConditionalStatement),
    Loop(LoopStatement),
    Return(ReturnStatement),
    Expression(ExpressionStatement),
}

#[derive(Debug, Clone)]
pub struct UseStatement {
    pub alias: UseAlias,
    pub module_path: String,
    pub module_span: SourceSpan,
}

#[derive(Debug, Clone)]
pub struct UseAlias {
    pub name: String,
    pub span: SourceSpan,
}

#[derive(Debug, Clone)]
pub struct VarStatement {
    pub is_const: bool,
    pub bindings: Vec<VarBinding>,
    pub docstring: Option<String>,
}

#[derive(Debug, Clone)]
pub struct VarBinding {
    pub name: String,
    pub span: SourceSpan,
    pub type_annotation: Option<TypeExpression>,
    pub initializer: Option<Expression>,
}

#[derive(Debug, Clone)]
pub struct TypeExpression {
    pub tokens: Vec<Token>,
}

#[derive(Debug, Clone)]
pub struct TypeParameter {
    pub name: String,
    pub span: SourceSpan,
}

#[derive(Debug, Clone)]
pub struct FunctionStatement {
    pub is_public: bool,
    pub name: String,
    pub name_span: SourceSpan,
    pub type_parameters: Vec<TypeParameter>,
    pub parameters: Vec<FunctionParameter>,
    pub return_type: Option<TypeExpression>,
    pub body: Block,
    pub docstring: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TestStatement {
    pub name: String,
    pub name_span: SourceSpan,
    pub body: Block,
    pub docstring: Option<String>,
}

#[derive(Debug, Clone)]
pub struct FunctionParameter {
    pub name: String,
    pub span: SourceSpan,
    pub type_annotation: Option<TypeExpression>,
    pub default_value: Option<Expression>,
}

#[derive(Debug, Clone)]
pub struct StructStatement {
    pub name: String,
    pub name_span: SourceSpan,
    pub type_parameters: Vec<TypeParameter>,
    pub fields: Vec<StructField>,
    pub docstring: Option<String>,
}

#[derive(Debug, Clone)]
pub struct StructField {
    pub name: String,
    pub span: SourceSpan,
    pub type_annotation: TypeExpression,
}

#[derive(Debug, Clone)]
pub struct ConditionalStatement {
    pub kind: ConditionalKind,
    pub condition: Expression,
    pub consequent: Block,
    pub alternative: Option<Block>,
}

#[derive(Debug, Clone, Copy)]
pub enum ConditionalKind {
    If,
    Unless,
}

#[derive(Debug, Clone)]
pub struct LoopStatement {
    pub kind: LoopKind,
    pub header: LoopHeader,
    pub body: Block,
    pub span: SourceSpan,
}

#[derive(Debug, Clone)]
pub enum LoopKind {
    For,
    While,
    Until,
}

#[derive(Debug, Clone)]
pub enum LoopHeader {
    For {
        pattern: Expression,
        iterator: Expression,
    },
    Condition(Expression),
}

#[derive(Debug, Clone)]
pub struct ReturnStatement {
    pub span: SourceSpan,
    pub expression: Option<Expression>,
}

#[derive(Debug, Clone)]
pub struct ExpressionStatement {
    pub expression: Expression,
}

#[derive(Debug, Clone, Default)]
pub struct Block {
    pub statements: Vec<Statement>,
}

#[derive(Debug, Clone)]
pub struct Identifier {
    pub name: String,
    pub span: SourceSpan,
}

#[derive(Debug, Clone)]
pub enum Literal {
    Integer(i64),
    Float(f64),
    String(String),
    Boolean(bool),
    Nil,
}

#[derive(Debug, Clone, Copy)]
pub enum UnaryOperator {
    Positive,
    Negative,
    Not,
}

#[derive(Debug, Clone, Copy)]
pub enum BinaryOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
    Equal,
    NotEqual,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,
    And,
    Or,
}

#[derive(Debug, Clone)]
pub struct Expression {
    pub span: SourceSpan,
    pub kind: ExpressionKind,
}

#[derive(Debug, Clone)]
pub enum ExpressionKind {
    Identifier(Identifier),
    Literal(Literal),
    InterpolatedString(InterpolatedStringExpression),
    List(ListLiteral),
    Dict(DictLiteral),
    Unary(UnaryExpression),
    Binary(BinaryExpression),
    Call(CallExpression),
    Member(MemberExpression),
    Index(IndexExpression),
    Range(RangeExpression),
    Lambda(LambdaExpression),
    Assignment(AssignmentExpression),
    Grouping(Box<Expression>),
}

#[derive(Debug, Clone)]
pub struct InterpolatedStringExpression {
    pub parts: Vec<InterpolatedStringPart>,
}

#[derive(Debug, Clone)]
pub enum InterpolatedStringPart {
    Literal(String),
    Expression(Expression),
}

#[derive(Debug, Clone)]
pub struct ListLiteral {
    pub elements: Vec<Expression>,
}

#[derive(Debug, Clone)]
pub struct DictLiteral {
    pub entries: Vec<DictEntry>,
}

#[derive(Debug, Clone)]
pub struct DictEntry {
    pub key: String,
    pub value: Expression,
}

#[derive(Debug, Clone)]
pub struct UnaryExpression {
    pub operator: UnaryOperator,
    pub operand: Box<Expression>,
}

#[derive(Debug, Clone)]
pub struct BinaryExpression {
    pub operator: BinaryOperator,
    pub left: Box<Expression>,
    pub right: Box<Expression>,
}

#[derive(Debug, Clone)]
pub struct CallExpression {
    pub callee: Box<Expression>,
    pub type_arguments: Vec<TypeExpression>,
    pub arguments: Vec<CallArgument>,
}

#[derive(Debug, Clone)]
pub struct CallArgument {
    pub name: Option<String>,
    pub name_span: Option<SourceSpan>,
    pub expression: Expression,
}

#[derive(Debug, Clone)]
pub struct MemberExpression {
    pub object: Box<Expression>,
    pub property: String,
    pub property_span: SourceSpan,
}

#[derive(Debug, Clone)]
pub struct IndexExpression {
    pub object: Box<Expression>,
    pub index: Box<Expression>,
}

#[derive(Debug, Clone)]
pub struct RangeExpression {
    pub start: Box<Expression>,
    pub end: Box<Expression>,
    pub inclusive: bool,
}

#[derive(Debug, Clone)]
pub struct LambdaExpression {
    pub id: usize,
    pub parameters: Vec<FunctionParameter>,
    pub body: LambdaBody,
}

#[derive(Debug, Clone)]
pub enum LambdaBody {
    Expression(Box<Expression>),
    Block(Block),
}

#[derive(Debug, Clone)]
pub struct AssignmentExpression {
    pub target: Box<Expression>,
    pub value: Box<Expression>,
}
