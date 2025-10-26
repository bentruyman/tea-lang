use std::fmt;
use std::rc::Rc;

use super::value::{StructTemplate, Value};
use crate::ast::SourceSpan;
use crate::stdlib::StdFunctionKind;

#[derive(Debug, Clone)]
pub struct Program {
    pub chunk: Chunk,
    pub functions: Vec<Function>,
    pub globals: Vec<String>,
    pub structs: Vec<Rc<StructTemplate>>,
    pub tests: Vec<TestCase>,
}

impl Program {
    pub fn new(
        chunk: Chunk,
        functions: Vec<Function>,
        globals: Vec<String>,
        structs: Vec<StructTemplate>,
        tests: Vec<TestCase>,
    ) -> Self {
        Self {
            chunk,
            functions,
            globals,
            structs: structs.into_iter().map(Rc::new).collect(),
            tests,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Chunk {
    pub instructions: Vec<Instruction>,
    pub constants: Vec<Value>,
}

impl Chunk {
    pub fn new() -> Self {
        Self {
            instructions: Vec::new(),
            constants: Vec::new(),
        }
    }

    pub fn emit(&mut self, instruction: Instruction) -> usize {
        let index = self.instructions.len();
        self.instructions.push(instruction);
        index
    }

    pub fn add_constant(&mut self, value: Value) -> usize {
        self.constants.push(value);
        self.constants.len() - 1
    }

    pub fn len(&self) -> usize {
        self.instructions.len()
    }
}

#[derive(Debug, Clone)]
pub enum TypeCheck {
    Bool,
    Int,
    Float,
    String,
    Nil,
    Struct(String),
    Enum(String),
    Optional(Box<TypeCheck>),
    Union(Vec<TypeCheck>),
}

#[derive(Debug, Clone)]
pub enum Instruction {
    Constant(usize),
    GetGlobal(usize),
    SetGlobal(usize),
    Pop,
    GetLocal(usize),
    SetLocal(usize),
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
    Negate,
    Not,
    TypeIs(TypeCheck),
    Jump(usize),
    JumpIfFalse(usize),
    JumpIfNil(usize),
    Print,
    BuiltinCall {
        kind: StdFunctionKind,
        arg_count: usize,
    },
    Call(usize),
    MakeList(usize),
    Index,
    MakeDict(usize),
    GetField,
    MakeStructPositional(usize),
    MakeStructNamed(usize),
    MakeClosure {
        function_index: usize,
        capture_count: usize,
    },
    ConcatStrings(usize),
    AssertNonNil,
    Return,
}

impl fmt::Display for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Instruction::Constant(index) => write!(f, "CONSTANT {index}"),
            Instruction::GetGlobal(index) => write!(f, "GET_GLOBAL {index}"),
            Instruction::SetGlobal(index) => write!(f, "SET_GLOBAL {index}"),
            Instruction::Pop => write!(f, "POP"),
            Instruction::GetLocal(index) => write!(f, "GET_LOCAL {index}"),
            Instruction::SetLocal(index) => write!(f, "SET_LOCAL {index}"),
            Instruction::Add => write!(f, "ADD"),
            Instruction::Subtract => write!(f, "SUB"),
            Instruction::Multiply => write!(f, "MUL"),
            Instruction::Divide => write!(f, "DIV"),
            Instruction::Modulo => write!(f, "MOD"),
            Instruction::Equal => write!(f, "EQ"),
            Instruction::NotEqual => write!(f, "NEQ"),
            Instruction::Greater => write!(f, "GT"),
            Instruction::GreaterEqual => write!(f, "GTE"),
            Instruction::Less => write!(f, "LT"),
            Instruction::LessEqual => write!(f, "LTE"),
            Instruction::Negate => write!(f, "NEG"),
            Instruction::Not => write!(f, "NOT"),
            Instruction::TypeIs(check) => write!(f, "TYPE_IS {:?}", check),
            Instruction::Jump(target) => write!(f, "JUMP {target}"),
            Instruction::JumpIfFalse(target) => write!(f, "JUMP_IF_FALSE {target}"),
            Instruction::JumpIfNil(target) => write!(f, "JUMP_IF_NIL {target}"),
            Instruction::Print => write!(f, "PRINT"),
            Instruction::BuiltinCall { kind, arg_count } => {
                write!(f, "BUILTIN {:?} {arg_count}", kind)
            }
            Instruction::Call(args) => write!(f, "CALL {args}"),
            Instruction::MakeList(count) => write!(f, "MAKE_LIST {count}"),
            Instruction::Index => write!(f, "INDEX"),
            Instruction::MakeDict(count) => write!(f, "MAKE_DICT {count}"),
            Instruction::GetField => write!(f, "GET_FIELD"),
            Instruction::MakeStructPositional(index) => {
                write!(f, "MAKE_STRUCT_POS {index}")
            }
            Instruction::MakeStructNamed(index) => write!(f, "MAKE_STRUCT_NAMED {index}"),
            Instruction::MakeClosure {
                function_index,
                capture_count,
            } => write!(f, "MAKE_CLOSURE {function_index} {capture_count}"),
            Instruction::ConcatStrings(count) => write!(f, "CONCAT_STRINGS {count}"),
            Instruction::AssertNonNil => write!(f, "ASSERT_NON_NIL"),
            Instruction::Return => write!(f, "RETURN"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Function {
    pub name: String,
    pub arity: usize,
    pub chunk: Chunk,
}

#[derive(Debug, Clone)]
pub struct TestCase {
    pub name: String,
    pub name_span: SourceSpan,
    pub function_index: usize,
}
