use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::fmt;
use std::rc::Rc;

use anyhow::{anyhow, bail, Result};
use tea_compiler::{
    AssignmentExpression, BinaryOperator, Block, CallArgument, Compilation, ConditionalExpression,
    Expression, ExpressionKind, ForPattern, FunctionParameter, Identifier, IndexExpression,
    InterpolatedStringPart, LambdaBody, LambdaExpression, Literal, LoopHeader, MemberExpression,
    Module, SourceSpan, Statement, TypeExpression, UnaryOperator,
};

#[derive(Debug, Clone)]
pub struct EvalOptions {
    pub fuel: usize,
}

impl Default for EvalOptions {
    fn default() -> Self {
        Self { fuel: 50_000 }
    }
}

#[derive(Debug, Clone, Default)]
pub struct EvalOutput {
    pub stdout: Vec<String>,
    pub result: Option<String>,
    pub runtime_error: Option<String>,
    pub exit_code: Option<i32>,
}

pub fn evaluate(compilation: &Compilation, options: EvalOptions) -> EvalOutput {
    let mut interpreter = Interpreter::new(options);
    match interpreter.execute_module(&compilation.module) {
        Ok(result) => EvalOutput {
            stdout: interpreter.stdout,
            result: result.map(|value| value.render()),
            runtime_error: None,
            exit_code: interpreter.exit_code,
        },
        Err(error) => EvalOutput {
            stdout: interpreter.stdout,
            result: None,
            runtime_error: Some(error.to_string()),
            exit_code: interpreter.exit_code,
        },
    }
}

#[derive(Clone)]
enum Value {
    Nil,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    List(Rc<RefCell<Vec<Value>>>),
    Dict(Rc<RefCell<BTreeMap<String, Value>>>),
    Struct(Rc<RefCell<StructValue>>),
    Function(Rc<FunctionValue>),
    Module(Rc<HashMap<String, Value>>),
    Range(RangeValue),
}

impl Value {
    fn type_name(&self) -> String {
        match self {
            Self::Nil => "Nil".into(),
            Self::Bool(_) => "Bool".into(),
            Self::Int(_) => "Int".into(),
            Self::Float(_) => "Float".into(),
            Self::String(_) => "String".into(),
            Self::List(_) => "List".into(),
            Self::Dict(_) => "Dict".into(),
            Self::Struct(struct_value) => struct_value.borrow().name.clone(),
            Self::Function(_) => "Function".into(),
            Self::Module(_) => "Module".into(),
            Self::Range(_) => "Range".into(),
        }
    }

    fn render(&self) -> String {
        match self {
            Self::Nil => "nil".into(),
            Self::Bool(value) => value.to_string(),
            Self::Int(value) => value.to_string(),
            Self::Float(value) => {
                if value.fract() == 0.0 {
                    format!("{value:.1}")
                } else {
                    value.to_string()
                }
            }
            Self::String(value) => value.clone(),
            Self::List(items) => {
                let rendered = items
                    .borrow()
                    .iter()
                    .map(Value::render)
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("[{rendered}]")
            }
            Self::Dict(entries) => {
                let rendered = entries
                    .borrow()
                    .iter()
                    .map(|(key, value)| format!("{key}: {}", value.render()))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{{ {rendered} }}")
            }
            Self::Struct(struct_value) => {
                let struct_value = struct_value.borrow();
                let rendered = struct_value
                    .fields
                    .iter()
                    .map(|(key, value)| format!("{key}: {}", value.render()))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{}({rendered})", struct_value.name)
            }
            Self::Function(_) => "<function>".into(),
            Self::Module(_) => "<module>".into(),
            Self::Range(range) => {
                if range.inclusive {
                    format!("{}...{}", range.start, range.end)
                } else {
                    format!("{}..{}", range.start, range.end)
                }
            }
        }
    }
}

#[derive(Clone)]
struct RangeValue {
    start: i64,
    end: i64,
    inclusive: bool,
}

#[derive(Clone)]
struct StructValue {
    name: String,
    fields: BTreeMap<String, Value>,
}

#[derive(Clone)]
enum FunctionValue {
    User(UserFunction),
    Native(NativeFunction),
}

#[derive(Clone)]
struct UserFunction {
    name: Option<String>,
    parameters: Vec<FunctionParameter>,
    body: CallableBody,
    closure: Rc<Environment>,
}

#[derive(Clone)]
enum CallableBody {
    Block(Block),
    Expression(Expression),
}

#[derive(Clone)]
struct NativeFunction {
    name: String,
}

type ValueCell = Rc<RefCell<Value>>;

struct Environment {
    parent: Option<Rc<Environment>>,
    values: RefCell<HashMap<String, ValueCell>>,
}

impl Environment {
    fn new(parent: Option<Rc<Environment>>) -> Rc<Self> {
        Rc::new(Self {
            parent,
            values: RefCell::new(HashMap::new()),
        })
    }

    fn define(&self, name: impl Into<String>, value: Value) -> ValueCell {
        let cell = Rc::new(RefCell::new(value));
        self.values.borrow_mut().insert(name.into(), cell.clone());
        cell
    }

    fn define_placeholder(&self, name: impl Into<String>) -> ValueCell {
        self.define(name, Value::Nil)
    }

    fn get(&self, name: &str) -> Option<Value> {
        self.values
            .borrow()
            .get(name)
            .map(|value| value.borrow().clone())
            .or_else(|| self.parent.as_ref().and_then(|parent| parent.get(name)))
    }

    fn get_cell(&self, name: &str) -> Option<ValueCell> {
        self.values.borrow().get(name).cloned().or_else(|| {
            self.parent
                .as_ref()
                .and_then(|parent| parent.get_cell(name))
        })
    }

    fn assign(&self, name: &str, value: Value) -> Result<()> {
        let cell = self
            .get_cell(name)
            .ok_or_else(|| anyhow!("undefined variable '{name}'"))?;
        *cell.borrow_mut() = value;
        Ok(())
    }
}

#[derive(Clone)]
struct StructDefinition {
    name: String,
    fields: Vec<String>,
}

enum Flow {
    Next(Option<Value>),
    Return(Value),
    Break,
    Continue,
}

struct Interpreter {
    globals: Rc<Environment>,
    structs: HashMap<String, StructDefinition>,
    stdout: Vec<String>,
    exit_code: Option<i32>,
    fuel_remaining: usize,
}

impl Interpreter {
    fn new(options: EvalOptions) -> Self {
        let globals = Environment::new(None);
        let mut interpreter = Self {
            globals,
            structs: HashMap::new(),
            stdout: Vec::new(),
            exit_code: None,
            fuel_remaining: options.fuel,
        };
        interpreter.install_builtins();
        interpreter
    }

    fn install_builtins(&mut self) {
        for name in [
            "print",
            "println",
            "append",
            "to_string",
            "type_of",
            "panic",
            "len",
            "floor",
            "ceil",
            "round",
            "abs",
            "sqrt",
            "min",
            "max",
        ] {
            self.globals.define(
                name,
                Value::Function(Rc::new(FunctionValue::Native(NativeFunction {
                    name: name.to_string(),
                }))),
            );
        }
    }

    fn execute_module(&mut self, module: &Module) -> Result<Option<Value>> {
        self.register_declarations(&self.globals.clone(), &module.statements)?;

        let mut last_result = None;
        for statement in &module.statements {
            if matches!(statement, Statement::Function(_) | Statement::Struct(_)) {
                continue;
            }

            match self.execute_statement(&self.globals.clone(), statement)? {
                Flow::Next(result) => {
                    if result.is_some() {
                        last_result = result;
                    }
                }
                Flow::Return(value) => last_result = Some(value),
                Flow::Break => bail!("break cannot be used outside a loop"),
                Flow::Continue => bail!("continue cannot be used outside a loop"),
            }
        }

        Ok(last_result)
    }

    fn register_declarations(
        &mut self,
        env: &Rc<Environment>,
        statements: &[Statement],
    ) -> Result<()> {
        for statement in statements {
            match statement {
                Statement::Struct(struct_stmt) => {
                    self.structs.insert(
                        struct_stmt.name.clone(),
                        StructDefinition {
                            name: struct_stmt.name.clone(),
                            fields: struct_stmt
                                .fields
                                .iter()
                                .map(|field| field.name.clone())
                                .collect(),
                        },
                    );
                }
                Statement::Function(function) => {
                    let cell = env.define_placeholder(function.name.clone());
                    *cell.borrow_mut() =
                        Value::Function(Rc::new(FunctionValue::User(UserFunction {
                            name: Some(function.name.clone()),
                            parameters: function.parameters.clone(),
                            body: CallableBody::Block(function.body.clone()),
                            closure: env.clone(),
                        })));
                }
                _ => {}
            }
        }

        Ok(())
    }

    fn execute_statement(&mut self, env: &Rc<Environment>, statement: &Statement) -> Result<Flow> {
        self.consume_fuel()?;

        match statement {
            Statement::Use(use_stmt) => {
                if let Some(module) = self.runtime_module(&use_stmt.module_path) {
                    env.define(use_stmt.alias.name.clone(), module);
                }
                Ok(Flow::Next(None))
            }
            Statement::Var(var_stmt) => {
                for binding in &var_stmt.bindings {
                    let value = match &binding.initializer {
                        Some(initializer) => self.evaluate_expression(env, initializer)?,
                        None => Value::Nil,
                    };
                    env.define(binding.name.clone(), value);
                }
                Ok(Flow::Next(None))
            }
            Statement::Function(_) | Statement::Struct(_) => Ok(Flow::Next(None)),
            Statement::Test(test_stmt) => {
                let scope = Environment::new(Some(env.clone()));
                self.register_declarations(&scope, &test_stmt.body.statements)?;
                self.execute_block(&scope, &test_stmt.body)
            }
            Statement::Conditional(conditional) => {
                let condition = self.evaluate_expression(env, &conditional.condition)?;
                if self.is_truthy(&condition) {
                    let scope = Environment::new(Some(env.clone()));
                    self.register_declarations(&scope, &conditional.consequent.statements)?;
                    self.execute_block(&scope, &conditional.consequent)
                } else if let Some(alternative) = &conditional.alternative {
                    let scope = Environment::new(Some(env.clone()));
                    self.register_declarations(&scope, &alternative.statements)?;
                    self.execute_block(&scope, alternative)
                } else {
                    Ok(Flow::Next(None))
                }
            }
            Statement::Loop(loop_stmt) => match &loop_stmt.header {
                LoopHeader::Condition(condition) => {
                    let mut last_value = None;
                    while {
                        let condition_value = self.evaluate_expression(env, condition)?;
                        self.is_truthy(&condition_value)
                    } {
                        let scope = Environment::new(Some(env.clone()));
                        self.register_declarations(&scope, &loop_stmt.body.statements)?;
                        match self.execute_block(&scope, &loop_stmt.body)? {
                            Flow::Next(result) => {
                                if result.is_some() {
                                    last_value = result;
                                }
                            }
                            Flow::Return(value) => return Ok(Flow::Return(value)),
                            Flow::Break => break,
                            Flow::Continue => continue,
                        }
                    }
                    Ok(Flow::Next(last_value))
                }
                LoopHeader::For { pattern, iterator } => {
                    let iterable = self.evaluate_expression(env, iterator)?;
                    let mut last_value = None;
                    let entries = self.iter_entries(iterable)?;
                    for (key, value) in entries {
                        let scope = Environment::new(Some(env.clone()));
                        self.bind_for_pattern(&scope, pattern, key, value)?;
                        self.register_declarations(&scope, &loop_stmt.body.statements)?;
                        match self.execute_block(&scope, &loop_stmt.body)? {
                            Flow::Next(result) => {
                                if result.is_some() {
                                    last_value = result;
                                }
                            }
                            Flow::Return(value) => return Ok(Flow::Return(value)),
                            Flow::Break => break,
                            Flow::Continue => continue,
                        }
                    }
                    Ok(Flow::Next(last_value))
                }
            },
            Statement::Break(_) => Ok(Flow::Break),
            Statement::Continue(_) => Ok(Flow::Continue),
            Statement::Throw(_) => bail!("throw is not supported in the browser runner"),
            Statement::Return(return_stmt) => {
                let value = match &return_stmt.expression {
                    Some(expression) => self.evaluate_expression(env, expression)?,
                    None => Value::Nil,
                };
                Ok(Flow::Return(value))
            }
            Statement::Match(_) => bail!("match is not supported in the browser runner"),
            Statement::Expression(expression) => {
                let value = self.evaluate_expression(env, &expression.expression)?;
                Ok(Flow::Next(Some(value)))
            }
            Statement::Union(_) | Statement::Enum(_) | Statement::Error(_) => {
                bail!("this language feature is not supported in the browser runner")
            }
        }
    }

    fn execute_block(&mut self, env: &Rc<Environment>, block: &Block) -> Result<Flow> {
        let mut last_result = None;
        for statement in &block.statements {
            match self.execute_statement(env, statement)? {
                Flow::Next(result) => {
                    if result.is_some() {
                        last_result = result;
                    }
                }
                Flow::Return(value) => return Ok(Flow::Return(value)),
                Flow::Break => return Ok(Flow::Break),
                Flow::Continue => return Ok(Flow::Continue),
            }
        }
        Ok(Flow::Next(last_result))
    }

    fn evaluate_expression(
        &mut self,
        env: &Rc<Environment>,
        expression: &Expression,
    ) -> Result<Value> {
        self.consume_fuel()?;

        match &expression.kind {
            ExpressionKind::Identifier(identifier) => self.lookup_identifier(env, identifier),
            ExpressionKind::Literal(literal) => Ok(self.literal_value(literal)),
            ExpressionKind::InterpolatedString(interpolated) => {
                let mut result = String::new();
                for part in &interpolated.parts {
                    match part {
                        InterpolatedStringPart::Literal(text) => result.push_str(text),
                        InterpolatedStringPart::Expression(expression) => {
                            result.push_str(&self.evaluate_expression(env, expression)?.render());
                        }
                    }
                }
                Ok(Value::String(result))
            }
            ExpressionKind::List(list) => {
                let mut values = Vec::with_capacity(list.elements.len());
                for element in &list.elements {
                    values.push(self.evaluate_expression(env, element)?);
                }
                Ok(Value::List(Rc::new(RefCell::new(values))))
            }
            ExpressionKind::Dict(dict) => {
                let mut values = BTreeMap::new();
                for entry in &dict.entries {
                    values.insert(
                        entry.key.clone(),
                        self.evaluate_expression(env, &entry.value)?,
                    );
                }
                Ok(Value::Dict(Rc::new(RefCell::new(values))))
            }
            ExpressionKind::Unary(unary) => {
                let operand = self.evaluate_expression(env, &unary.operand)?;
                match unary.operator {
                    UnaryOperator::Positive => match operand {
                        Value::Int(_) | Value::Float(_) => Ok(operand),
                        _ => bail!("unary '+' expects a number"),
                    },
                    UnaryOperator::Negative => match operand {
                        Value::Int(value) => Ok(Value::Int(-value)),
                        Value::Float(value) => Ok(Value::Float(-value)),
                        _ => bail!("unary '-' expects a number"),
                    },
                    UnaryOperator::Not => Ok(Value::Bool(!self.is_truthy(&operand))),
                }
            }
            ExpressionKind::Binary(binary) => self.evaluate_binary(
                env,
                binary.left.as_ref(),
                binary.operator,
                binary.right.as_ref(),
            ),
            ExpressionKind::Is(is_expr) => {
                let value = self.evaluate_expression(env, &is_expr.value)?;
                Ok(Value::Bool(
                    self.matches_type(&value, &is_expr.type_annotation),
                ))
            }
            ExpressionKind::Call(call) => {
                self.evaluate_call(env, &call.callee, &call.arguments, expression.span)
            }
            ExpressionKind::Member(member) => self.evaluate_member(env, member),
            ExpressionKind::Index(index) => self.evaluate_index(env, index),
            ExpressionKind::Range(range) => {
                let start_value = self.evaluate_expression(env, &range.start)?;
                let end_value = self.evaluate_expression(env, &range.end)?;
                let start = self.expect_int(start_value)?;
                let end = self.expect_int(end_value)?;
                Ok(Value::Range(RangeValue {
                    start,
                    end,
                    inclusive: range.inclusive,
                }))
            }
            ExpressionKind::Lambda(lambda) => Ok(Value::Function(Rc::new(FunctionValue::User(
                self.lambda_value(env, lambda),
            )))),
            ExpressionKind::Assignment(assignment) => self.evaluate_assignment(env, assignment),
            ExpressionKind::Match(_) => bail!("match is not supported in the browser runner"),
            ExpressionKind::Conditional(ConditionalExpression {
                condition,
                consequent,
                alternative,
            }) => {
                let condition_value = self.evaluate_expression(env, condition)?;
                if self.is_truthy(&condition_value) {
                    self.evaluate_expression(env, consequent)
                } else {
                    self.evaluate_expression(env, alternative)
                }
            }
            ExpressionKind::Unwrap(inner) => {
                let value = self.evaluate_expression(env, inner)?;
                if matches!(value, Value::Nil) {
                    bail!("attempted to unwrap nil")
                } else {
                    Ok(value)
                }
            }
            ExpressionKind::Try(_) => bail!("try is not supported in the browser runner"),
            ExpressionKind::Grouping(inner) => self.evaluate_expression(env, inner),
        }
    }

    fn evaluate_binary(
        &mut self,
        env: &Rc<Environment>,
        left: &Expression,
        operator: BinaryOperator,
        right: &Expression,
    ) -> Result<Value> {
        match operator {
            BinaryOperator::And => {
                let left_value = self.evaluate_expression(env, left)?;
                if !self.is_truthy(&left_value) {
                    return Ok(Value::Bool(false));
                }
                let right_value = self.evaluate_expression(env, right)?;
                return Ok(Value::Bool(self.is_truthy(&right_value)));
            }
            BinaryOperator::Or => {
                let left_value = self.evaluate_expression(env, left)?;
                if self.is_truthy(&left_value) {
                    return Ok(Value::Bool(true));
                }
                let right_value = self.evaluate_expression(env, right)?;
                return Ok(Value::Bool(self.is_truthy(&right_value)));
            }
            BinaryOperator::Coalesce => {
                let left_value = self.evaluate_expression(env, left)?;
                if !matches!(left_value, Value::Nil) {
                    return Ok(left_value);
                }
                return self.evaluate_expression(env, right);
            }
            _ => {}
        }

        let left_value = self.evaluate_expression(env, left)?;
        let right_value = self.evaluate_expression(env, right)?;

        match operator {
            BinaryOperator::Add => self.add_values(left_value, right_value),
            BinaryOperator::Subtract => {
                self.numeric_binary(left_value, right_value, |a, b| a - b, |a, b| a - b)
            }
            BinaryOperator::Multiply => {
                self.numeric_binary(left_value, right_value, |a, b| a * b, |a, b| a * b)
            }
            BinaryOperator::Divide => self.divide_values(left_value, right_value),
            BinaryOperator::Modulo => {
                let left = self.expect_int(left_value)?;
                let right = self.expect_int(right_value)?;
                Ok(Value::Int(left % right))
            }
            BinaryOperator::Equal => Ok(Value::Bool(self.values_equal(&left_value, &right_value))),
            BinaryOperator::NotEqual => {
                Ok(Value::Bool(!self.values_equal(&left_value, &right_value)))
            }
            BinaryOperator::Greater => {
                self.compare_values(left_value, right_value, |ordering| ordering.is_gt())
            }
            BinaryOperator::GreaterEqual => {
                self.compare_values(left_value, right_value, |ordering| {
                    ordering.is_gt() || ordering.is_eq()
                })
            }
            BinaryOperator::Less => {
                self.compare_values(left_value, right_value, |ordering| ordering.is_lt())
            }
            BinaryOperator::LessEqual => self.compare_values(left_value, right_value, |ordering| {
                ordering.is_lt() || ordering.is_eq()
            }),
            BinaryOperator::And | BinaryOperator::Or | BinaryOperator::Coalesce => unreachable!(),
        }
    }

    fn evaluate_call(
        &mut self,
        env: &Rc<Environment>,
        callee: &Expression,
        arguments: &[CallArgument],
        span: SourceSpan,
    ) -> Result<Value> {
        if let ExpressionKind::Identifier(identifier) = &callee.kind {
            if self.structs.contains_key(&identifier.name) && env.get(&identifier.name).is_none() {
                return self.construct_struct(&identifier.name, env, arguments, span);
            }
        }

        let callee_value = self.evaluate_expression(env, callee)?;

        match callee_value {
            Value::Function(function) => match function.as_ref() {
                FunctionValue::Native(function) => self.call_native(env, function, arguments),
                FunctionValue::User(function) => self.call_user(function, env, arguments),
            },
            _ => bail!("attempted to call a non-function value"),
        }
    }

    fn evaluate_member(
        &mut self,
        env: &Rc<Environment>,
        member: &MemberExpression,
    ) -> Result<Value> {
        let object = self.evaluate_expression(env, &member.object)?;
        match object {
            Value::Module(module) => module
                .get(&member.property)
                .cloned()
                .ok_or_else(|| anyhow!("module member '{}' is undefined", member.property)),
            Value::Struct(struct_value) => struct_value
                .borrow()
                .fields
                .get(&member.property)
                .cloned()
                .ok_or_else(|| anyhow!("field '{}' is undefined", member.property)),
            Value::Dict(entries) => Ok(entries
                .borrow()
                .get(&member.property)
                .cloned()
                .unwrap_or(Value::Nil)),
            _ => bail!("member access is only supported on modules, structs, and dicts"),
        }
    }

    fn evaluate_index(&mut self, env: &Rc<Environment>, index: &IndexExpression) -> Result<Value> {
        let object = self.evaluate_expression(env, &index.object)?;
        let index_value = self.evaluate_expression(env, &index.index)?;
        self.index_value(object, index_value)
    }

    fn evaluate_assignment(
        &mut self,
        env: &Rc<Environment>,
        assignment: &AssignmentExpression,
    ) -> Result<Value> {
        let value = self.evaluate_expression(env, &assignment.value)?;
        self.assign_target(env, &assignment.target, value.clone())?;
        Ok(value)
    }

    fn assign_target(
        &mut self,
        env: &Rc<Environment>,
        target: &Expression,
        value: Value,
    ) -> Result<()> {
        match &target.kind {
            ExpressionKind::Identifier(identifier) => env.assign(&identifier.name, value),
            ExpressionKind::Member(member) => {
                let object = self.evaluate_expression(env, &member.object)?;
                match object {
                    Value::Struct(struct_value) => {
                        struct_value
                            .borrow_mut()
                            .fields
                            .insert(member.property.clone(), value);
                        Ok(())
                    }
                    Value::Dict(entries) => {
                        entries.borrow_mut().insert(member.property.clone(), value);
                        Ok(())
                    }
                    _ => bail!("member assignment is only supported on structs and dicts"),
                }
            }
            ExpressionKind::Index(index_expr) => {
                let object = self.evaluate_expression(env, &index_expr.object)?;
                let index = self.evaluate_expression(env, &index_expr.index)?;
                match (object, index) {
                    (Value::List(items), Value::Int(index)) => {
                        let index = self.normalize_index(index, items.borrow().len())?;
                        items.borrow_mut()[index] = value;
                        Ok(())
                    }
                    (Value::Dict(entries), Value::String(key)) => {
                        entries.borrow_mut().insert(key, value);
                        Ok(())
                    }
                    _ => bail!("index assignment is only supported on lists and dicts"),
                }
            }
            _ => bail!("invalid assignment target"),
        }
    }

    fn call_user(
        &mut self,
        function: &UserFunction,
        env: &Rc<Environment>,
        arguments: &[CallArgument],
    ) -> Result<Value> {
        let call_env = Environment::new(Some(function.closure.clone()));
        let (mut positional, named) = self.evaluate_arguments(env, arguments)?;
        self.bind_parameters(&call_env, &function.parameters, &mut positional, &named)?;

        match &function.body {
            CallableBody::Block(block) => {
                self.register_declarations(&call_env, &block.statements)?;
                match self.execute_block(&call_env, block)? {
                    Flow::Next(result) => Ok(result.unwrap_or(Value::Nil)),
                    Flow::Return(value) => Ok(value),
                    Flow::Break => bail!("break cannot escape a function"),
                    Flow::Continue => bail!("continue cannot escape a function"),
                }
            }
            CallableBody::Expression(expression) => self.evaluate_expression(&call_env, expression),
        }
    }

    fn call_native(
        &mut self,
        env: &Rc<Environment>,
        function: &NativeFunction,
        arguments: &[CallArgument],
    ) -> Result<Value> {
        let (mut positional, named) = self.evaluate_arguments(env, arguments)?;
        if !named.is_empty() {
            bail!("named arguments are not supported for '{}'", function.name);
        }

        match function.name.as_str() {
            "print" => {
                let value = self.take_arg(&mut positional, "print")?;
                self.stdout.push(value.render());
                Ok(Value::Nil)
            }
            "println" => {
                let value = self.take_arg(&mut positional, "println")?;
                self.stdout.push(format!("{}\n", value.render()));
                Ok(Value::Nil)
            }
            "append" => {
                let list = self.take_arg(&mut positional, "append")?;
                let value = self.take_arg(&mut positional, "append")?;
                if let Value::List(items) = list {
                    items.borrow_mut().push(value);
                    Ok(Value::Nil)
                } else {
                    bail!("append expects a list as the first argument")
                }
            }
            "to_string" => Ok(Value::String(
                self.take_arg(&mut positional, "to_string")?.render(),
            )),
            "type_of" => Ok(Value::String(
                self.take_arg(&mut positional, "type_of")?.type_name(),
            )),
            "panic" => {
                let message = self.expect_string(self.take_arg(&mut positional, "panic")?)?;
                bail!("{message}")
            }
            "len" => match self.take_arg(&mut positional, "len")? {
                Value::String(value) => Ok(Value::Int(value.chars().count() as i64)),
                Value::List(items) => Ok(Value::Int(items.borrow().len() as i64)),
                Value::Dict(entries) => Ok(Value::Int(entries.borrow().len() as i64)),
                _ => bail!("len expects a string, list, or dict"),
            },
            "floor" => Ok(Value::Int(
                self.expect_number(self.take_arg(&mut positional, "floor")?)?
                    .floor() as i64,
            )),
            "ceil" => Ok(Value::Int(
                self.expect_number(self.take_arg(&mut positional, "ceil")?)?
                    .ceil() as i64,
            )),
            "round" => Ok(Value::Int(
                self.expect_number(self.take_arg(&mut positional, "round")?)?
                    .round() as i64,
            )),
            "abs" => Ok(Value::Float(
                self.expect_number(self.take_arg(&mut positional, "abs")?)?
                    .abs(),
            )),
            "sqrt" => Ok(Value::Float(
                self.expect_number(self.take_arg(&mut positional, "sqrt")?)?
                    .sqrt(),
            )),
            "min" => {
                let left = self.expect_number(self.take_arg(&mut positional, "min")?)?;
                let right = self.expect_number(self.take_arg(&mut positional, "min")?)?;
                Ok(Value::Float(left.min(right)))
            }
            "max" => {
                let left = self.expect_number(self.take_arg(&mut positional, "max")?)?;
                let right = self.expect_number(self.take_arg(&mut positional, "max")?)?;
                Ok(Value::Float(left.max(right)))
            }
            "std.assert.ok" => {
                let value = self.take_arg(&mut positional, "std.assert.ok")?;
                if self.is_truthy(&value) {
                    Ok(Value::Nil)
                } else {
                    bail!("assertion failed")
                }
            }
            "std.assert.eq" => {
                let left = self.take_arg(&mut positional, "std.assert.eq")?;
                let right = self.take_arg(&mut positional, "std.assert.eq")?;
                if self.values_equal(&left, &right) {
                    Ok(Value::Nil)
                } else {
                    bail!(
                        "assertion failed: '{}' != '{}'",
                        left.render(),
                        right.render()
                    )
                }
            }
            "std.assert.ne" => {
                let left = self.take_arg(&mut positional, "std.assert.ne")?;
                let right = self.take_arg(&mut positional, "std.assert.ne")?;
                if self.values_equal(&left, &right) {
                    bail!(
                        "assertion failed: '{}' == '{}'",
                        left.render(),
                        right.render()
                    )
                } else {
                    Ok(Value::Nil)
                }
            }
            "std.assert.snapshot" => {
                bail!("snapshot assertions are not supported in the browser runner")
            }
            "std.intrinsics.string_index_of" => {
                let text =
                    self.expect_string(self.take_arg(&mut positional, "string_index_of")?)?;
                let pattern =
                    self.expect_string(self.take_arg(&mut positional, "string_index_of")?)?;
                let result = text
                    .find(&pattern)
                    .map(|offset| text[..offset].chars().count() as i64)
                    .unwrap_or(-1);
                Ok(Value::Int(result))
            }
            "std.intrinsics.string_split" => {
                let text = self.expect_string(self.take_arg(&mut positional, "string_split")?)?;
                let separator =
                    self.expect_string(self.take_arg(&mut positional, "string_split")?)?;
                let parts = if separator.is_empty() {
                    text.chars()
                        .map(|character| Value::String(character.to_string()))
                        .collect()
                } else {
                    text.split(&separator)
                        .map(|part| Value::String(part.to_string()))
                        .collect()
                };
                Ok(Value::List(Rc::new(RefCell::new(parts))))
            }
            "std.intrinsics.string_contains" => {
                let text =
                    self.expect_string(self.take_arg(&mut positional, "string_contains")?)?;
                let pattern =
                    self.expect_string(self.take_arg(&mut positional, "string_contains")?)?;
                Ok(Value::Bool(text.contains(&pattern)))
            }
            "std.intrinsics.string_replace" => {
                let text = self.expect_string(self.take_arg(&mut positional, "string_replace")?)?;
                let pattern =
                    self.expect_string(self.take_arg(&mut positional, "string_replace")?)?;
                let replacement =
                    self.expect_string(self.take_arg(&mut positional, "string_replace")?)?;
                Ok(Value::String(text.replace(&pattern, &replacement)))
            }
            "std.intrinsics.string_to_lower" => {
                let text =
                    self.expect_string(self.take_arg(&mut positional, "string_to_lower")?)?;
                Ok(Value::String(text.to_lowercase()))
            }
            "std.intrinsics.string_to_upper" => {
                let text =
                    self.expect_string(self.take_arg(&mut positional, "string_to_upper")?)?;
                Ok(Value::String(text.to_uppercase()))
            }
            "std.intrinsics.json_encode" => {
                let value = self.take_arg(&mut positional, "json_encode")?;
                Ok(Value::String(self.value_to_json(&value)?.to_string()))
            }
            "std.intrinsics.json_decode" => {
                let text = self.expect_string(self.take_arg(&mut positional, "json_decode")?)?;
                Ok(self.json_to_value(&serde_json::from_str(&text)?))
            }
            _ => bail!("native function '{}' is not implemented", function.name),
        }
    }

    fn bind_parameters(
        &mut self,
        env: &Rc<Environment>,
        parameters: &[FunctionParameter],
        positional: &mut VecDeque<Value>,
        named: &HashMap<String, Value>,
    ) -> Result<()> {
        for parameter in parameters {
            let value = if let Some(value) = named.get(&parameter.name) {
                value.clone()
            } else if let Some(value) = positional.pop_front() {
                value
            } else if let Some(default) = &parameter.default_value {
                self.evaluate_expression(env, default)?
            } else {
                bail!("missing argument '{}'", parameter.name)
            };
            env.define(parameter.name.clone(), value);
        }

        if !positional.is_empty() {
            bail!("too many positional arguments")
        }

        for key in named.keys() {
            if !parameters.iter().any(|parameter| parameter.name == *key) {
                bail!("unknown named argument '{key}'")
            }
        }

        Ok(())
    }

    fn evaluate_arguments(
        &mut self,
        env: &Rc<Environment>,
        arguments: &[CallArgument],
    ) -> Result<(VecDeque<Value>, HashMap<String, Value>)> {
        let mut positional = VecDeque::new();
        let mut named = HashMap::new();
        for argument in arguments {
            let value = self.evaluate_expression(env, &argument.expression)?;
            if let Some(name) = &argument.name {
                named.insert(name.clone(), value);
            } else {
                positional.push_back(value);
            }
        }
        Ok((positional, named))
    }

    fn construct_struct(
        &mut self,
        name: &str,
        env: &Rc<Environment>,
        arguments: &[CallArgument],
        _span: SourceSpan,
    ) -> Result<Value> {
        let definition = self
            .structs
            .get(name)
            .cloned()
            .ok_or_else(|| anyhow!("unknown struct '{name}'"))?;
        let (mut positional, named) = self.evaluate_arguments(env, arguments)?;
        let mut fields = BTreeMap::new();

        for field in &definition.fields {
            let value = if let Some(value) = named.get(field) {
                value.clone()
            } else if let Some(value) = positional.pop_front() {
                value
            } else {
                bail!("missing field '{field}' for struct '{name}'")
            };
            fields.insert(field.clone(), value);
        }

        if !positional.is_empty() {
            bail!("too many arguments for struct '{name}'")
        }

        for field in named.keys() {
            if !definition.fields.contains(field) {
                bail!("unknown field '{field}' for struct '{name}'")
            }
        }

        Ok(Value::Struct(Rc::new(RefCell::new(StructValue {
            name: definition.name,
            fields,
        }))))
    }

    fn lookup_identifier(&self, env: &Rc<Environment>, identifier: &Identifier) -> Result<Value> {
        env.get(&identifier.name)
            .ok_or_else(|| anyhow!("undefined identifier '{}'", identifier.name))
    }

    fn literal_value(&self, literal: &Literal) -> Value {
        match literal {
            Literal::Integer(value) => Value::Int(*value),
            Literal::Float(value) => Value::Float(*value),
            Literal::String(value) => Value::String(value.clone()),
            Literal::Boolean(value) => Value::Bool(*value),
            Literal::Nil => Value::Nil,
        }
    }

    fn lambda_value(&self, env: &Rc<Environment>, lambda: &LambdaExpression) -> UserFunction {
        UserFunction {
            name: None,
            parameters: lambda.parameters.clone(),
            body: match &lambda.body {
                LambdaBody::Expression(expression) => {
                    CallableBody::Expression(expression.as_ref().clone())
                }
                LambdaBody::Block(block) => CallableBody::Block(block.clone()),
            },
            closure: env.clone(),
        }
    }

    fn runtime_module(&self, module_path: &str) -> Option<Value> {
        match module_path {
            "std.assert" => Some(Value::Module(Rc::new(HashMap::from([
                (
                    "ok".into(),
                    Value::Function(Rc::new(FunctionValue::Native(NativeFunction {
                        name: "std.assert.ok".into(),
                    }))),
                ),
                (
                    "eq".into(),
                    Value::Function(Rc::new(FunctionValue::Native(NativeFunction {
                        name: "std.assert.eq".into(),
                    }))),
                ),
                (
                    "ne".into(),
                    Value::Function(Rc::new(FunctionValue::Native(NativeFunction {
                        name: "std.assert.ne".into(),
                    }))),
                ),
                (
                    "snapshot".into(),
                    Value::Function(Rc::new(FunctionValue::Native(NativeFunction {
                        name: "std.assert.snapshot".into(),
                    }))),
                ),
            ])))),
            "std.json" => Some(Value::Module(Rc::new(HashMap::from([
                (
                    "encode".into(),
                    self.native_function_value("std.intrinsics.json_encode"),
                ),
                (
                    "decode".into(),
                    self.native_function_value("std.intrinsics.json_decode"),
                ),
            ])))),
            "std.intrinsics" => Some(Value::Module(Rc::new(HashMap::from([
                (
                    "string_index_of".into(),
                    self.native_function_value("std.intrinsics.string_index_of"),
                ),
                (
                    "string_split".into(),
                    self.native_function_value("std.intrinsics.string_split"),
                ),
                (
                    "string_contains".into(),
                    self.native_function_value("std.intrinsics.string_contains"),
                ),
                (
                    "string_replace".into(),
                    self.native_function_value("std.intrinsics.string_replace"),
                ),
                (
                    "string_to_lower".into(),
                    self.native_function_value("std.intrinsics.string_to_lower"),
                ),
                (
                    "string_to_upper".into(),
                    self.native_function_value("std.intrinsics.string_to_upper"),
                ),
                (
                    "json_encode".into(),
                    self.native_function_value("std.intrinsics.json_encode"),
                ),
                (
                    "json_decode".into(),
                    self.native_function_value("std.intrinsics.json_decode"),
                ),
            ])))),
            _ => None,
        }
    }

    fn native_function_value(&self, name: &str) -> Value {
        Value::Function(Rc::new(FunctionValue::Native(NativeFunction {
            name: name.to_string(),
        })))
    }

    fn iter_entries(&self, value: Value) -> Result<Vec<(Option<Value>, Value)>> {
        match value {
            Value::List(items) => Ok(items
                .borrow()
                .iter()
                .cloned()
                .enumerate()
                .map(|(index, value)| (Some(Value::Int(index as i64)), value))
                .collect()),
            Value::String(text) => Ok(text
                .chars()
                .map(|character| (None, Value::String(character.to_string())))
                .collect()),
            Value::Dict(entries) => Ok(entries
                .borrow()
                .iter()
                .map(|(key, value)| (Some(Value::String(key.clone())), value.clone()))
                .collect()),
            Value::Range(range) => {
                let end = if range.inclusive {
                    range.end + 1
                } else {
                    range.end
                };
                Ok((range.start..end)
                    .map(|value| (None, Value::Int(value)))
                    .collect())
            }
            _ => bail!("value is not iterable"),
        }
    }

    fn bind_for_pattern(
        &self,
        env: &Rc<Environment>,
        pattern: &ForPattern,
        key: Option<Value>,
        value: Value,
    ) -> Result<()> {
        match pattern {
            ForPattern::Single(identifier) => {
                env.define(identifier.name.clone(), value);
                Ok(())
            }
            ForPattern::Pair(left, right) => {
                let key =
                    key.ok_or_else(|| anyhow!("pair iteration requires key/value entries"))?;
                env.define(left.name.clone(), key);
                env.define(right.name.clone(), value);
                Ok(())
            }
        }
    }

    fn index_value(&self, object: Value, index: Value) -> Result<Value> {
        match (object, index) {
            (Value::List(items), Value::Int(index)) => {
                let index = self.normalize_index(index, items.borrow().len())?;
                Ok(items.borrow()[index].clone())
            }
            (Value::List(items), Value::Range(range)) => {
                let list = items.borrow();
                let (start, end) = self.slice_bounds(range, list.len())?;
                Ok(Value::List(Rc::new(RefCell::new(
                    list[start..end].to_vec(),
                ))))
            }
            (Value::String(text), Value::Int(index)) => {
                let chars = text.chars().collect::<Vec<_>>();
                let index = self.normalize_index(index, chars.len())?;
                Ok(Value::String(chars[index].to_string()))
            }
            (Value::String(text), Value::Range(range)) => {
                let chars = text.chars().collect::<Vec<_>>();
                let (start, end) = self.slice_bounds(range, chars.len())?;
                Ok(Value::String(chars[start..end].iter().collect()))
            }
            (Value::Dict(entries), Value::String(key)) => {
                Ok(entries.borrow().get(&key).cloned().unwrap_or(Value::Nil))
            }
            _ => bail!("unsupported index expression"),
        }
    }

    fn slice_bounds(&self, range: RangeValue, len: usize) -> Result<(usize, usize)> {
        let start = self.normalize_index_allow_end(range.start, len)?;
        let end = self.normalize_index_allow_end(
            if range.inclusive {
                range.end + 1
            } else {
                range.end
            },
            len,
        )?;
        if start > end {
            return Ok((start, start));
        }
        Ok((start, end))
    }

    fn normalize_index(&self, index: i64, len: usize) -> Result<usize> {
        if index < 0 {
            bail!("negative indexes are not supported")
        }
        let index = index as usize;
        if index >= len {
            bail!("index out of bounds")
        }
        Ok(index)
    }

    fn normalize_index_allow_end(&self, index: i64, len: usize) -> Result<usize> {
        if index < 0 {
            bail!("negative indexes are not supported")
        }
        let index = index as usize;
        if index > len {
            bail!("slice bound out of range")
        }
        Ok(index)
    }

    fn add_values(&self, left: Value, right: Value) -> Result<Value> {
        match (left, right) {
            (Value::Int(left), Value::Int(right)) => Ok(Value::Int(left + right)),
            (Value::Float(left), Value::Float(right)) => Ok(Value::Float(left + right)),
            (Value::Int(left), Value::Float(right)) => Ok(Value::Float(left as f64 + right)),
            (Value::Float(left), Value::Int(right)) => Ok(Value::Float(left + right as f64)),
            (Value::String(left), Value::String(right)) => {
                Ok(Value::String(format!("{left}{right}")))
            }
            (Value::String(left), right) => Ok(Value::String(format!("{left}{}", right.render()))),
            (left, Value::String(right)) => Ok(Value::String(format!("{}{right}", left.render()))),
            _ => bail!("unsupported operands for '+'"),
        }
    }

    fn divide_values(&self, left: Value, right: Value) -> Result<Value> {
        match (left, right) {
            (Value::Int(left), Value::Int(right)) => Ok(Value::Int(left / right)),
            (Value::Float(left), Value::Float(right)) => Ok(Value::Float(left / right)),
            (Value::Int(left), Value::Float(right)) => Ok(Value::Float(left as f64 / right)),
            (Value::Float(left), Value::Int(right)) => Ok(Value::Float(left / right as f64)),
            _ => bail!("unsupported operands for '/'"),
        }
    }

    fn numeric_binary(
        &self,
        left: Value,
        right: Value,
        int_op: impl FnOnce(i64, i64) -> i64,
        float_op: impl FnOnce(f64, f64) -> f64,
    ) -> Result<Value> {
        match (left, right) {
            (Value::Int(left), Value::Int(right)) => Ok(Value::Int(int_op(left, right))),
            (Value::Float(left), Value::Float(right)) => Ok(Value::Float(float_op(left, right))),
            (Value::Int(left), Value::Float(right)) => {
                Ok(Value::Float(float_op(left as f64, right)))
            }
            (Value::Float(left), Value::Int(right)) => {
                Ok(Value::Float(float_op(left, right as f64)))
            }
            _ => bail!("expected numeric operands"),
        }
    }

    fn compare_values(
        &self,
        left: Value,
        right: Value,
        predicate: impl FnOnce(std::cmp::Ordering) -> bool,
    ) -> Result<Value> {
        let ordering = match (left, right) {
            (Value::Int(left), Value::Int(right)) => left.cmp(&right),
            (Value::Float(left), Value::Float(right)) => left
                .partial_cmp(&right)
                .ok_or_else(|| anyhow!("cannot compare NaN values"))?,
            (Value::Int(left), Value::Float(right)) => (left as f64)
                .partial_cmp(&right)
                .ok_or_else(|| anyhow!("cannot compare NaN values"))?,
            (Value::Float(left), Value::Int(right)) => left
                .partial_cmp(&(right as f64))
                .ok_or_else(|| anyhow!("cannot compare NaN values"))?,
            (Value::String(left), Value::String(right)) => left.cmp(&right),
            _ => bail!("values are not comparable"),
        };
        Ok(Value::Bool(predicate(ordering)))
    }

    fn matches_type(&self, value: &Value, type_expression: &TypeExpression) -> bool {
        let name = type_expression
            .tokens
            .iter()
            .map(|token| token.lexeme.as_str())
            .collect::<String>();

        match name.as_str() {
            "Any" => true,
            "Bool" => matches!(value, Value::Bool(_)),
            "Int" => matches!(value, Value::Int(_)),
            "Float" => matches!(value, Value::Float(_)),
            "String" => matches!(value, Value::String(_)),
            "List" => matches!(value, Value::List(_)),
            "Dict" => matches!(value, Value::Dict(_)),
            "Nil" => matches!(value, Value::Nil),
            other => {
                matches!(value, Value::Struct(struct_value) if struct_value.borrow().name == other)
            }
        }
    }

    fn values_equal(&self, left: &Value, right: &Value) -> bool {
        match (left, right) {
            (Value::Nil, Value::Nil) => true,
            (Value::Bool(left), Value::Bool(right)) => left == right,
            (Value::Int(left), Value::Int(right)) => left == right,
            (Value::Float(left), Value::Float(right)) => left == right,
            (Value::String(left), Value::String(right)) => left == right,
            (Value::List(left), Value::List(right)) => {
                let left = left.borrow();
                let right = right.borrow();
                left.len() == right.len()
                    && left
                        .iter()
                        .zip(right.iter())
                        .all(|(left, right)| self.values_equal(left, right))
            }
            (Value::Dict(left), Value::Dict(right)) => {
                let left = left.borrow();
                let right = right.borrow();
                left.len() == right.len()
                    && left.iter().all(|(key, left_value)| {
                        right
                            .get(key)
                            .map(|right_value| self.values_equal(left_value, right_value))
                            .unwrap_or(false)
                    })
            }
            (Value::Struct(left), Value::Struct(right)) => {
                let left = left.borrow();
                let right = right.borrow();
                left.name == right.name
                    && left.fields.len() == right.fields.len()
                    && left.fields.iter().all(|(key, left_value)| {
                        right
                            .fields
                            .get(key)
                            .map(|right_value| self.values_equal(left_value, right_value))
                            .unwrap_or(false)
                    })
            }
            _ => false,
        }
    }

    fn is_truthy(&self, value: &Value) -> bool {
        match value {
            Value::Nil => false,
            Value::Bool(value) => *value,
            _ => true,
        }
    }

    fn expect_string(&self, value: Value) -> Result<String> {
        match value {
            Value::String(value) => Ok(value),
            _ => bail!("expected a string"),
        }
    }

    fn expect_int(&self, value: Value) -> Result<i64> {
        match value {
            Value::Int(value) => Ok(value),
            _ => bail!("expected an integer"),
        }
    }

    fn expect_number(&self, value: Value) -> Result<f64> {
        match value {
            Value::Int(value) => Ok(value as f64),
            Value::Float(value) => Ok(value),
            _ => bail!("expected a number"),
        }
    }

    fn take_arg(&self, positional: &mut VecDeque<Value>, name: &str) -> Result<Value> {
        positional
            .pop_front()
            .ok_or_else(|| anyhow!("missing argument for '{name}'"))
    }

    fn value_to_json(&self, value: &Value) -> Result<serde_json::Value> {
        Ok(match value {
            Value::Nil => serde_json::Value::Null,
            Value::Bool(value) => serde_json::Value::Bool(*value),
            Value::Int(value) => serde_json::Value::Number((*value).into()),
            Value::Float(value) => serde_json::json!(value),
            Value::String(value) => serde_json::Value::String(value.clone()),
            Value::List(items) => serde_json::Value::Array(
                items
                    .borrow()
                    .iter()
                    .map(|item| self.value_to_json(item))
                    .collect::<Result<Vec<_>>>()?,
            ),
            Value::Dict(entries) => {
                let mut map = serde_json::Map::new();
                for (key, value) in entries.borrow().iter() {
                    map.insert(key.clone(), self.value_to_json(value)?);
                }
                serde_json::Value::Object(map)
            }
            Value::Struct(struct_value) => {
                let mut map = serde_json::Map::new();
                for (key, value) in struct_value.borrow().fields.iter() {
                    map.insert(key.clone(), self.value_to_json(value)?);
                }
                serde_json::Value::Object(map)
            }
            Value::Function(_) | Value::Module(_) | Value::Range(_) => {
                bail!("value cannot be encoded as JSON")
            }
        })
    }

    fn json_to_value(&self, value: &serde_json::Value) -> Value {
        match value {
            serde_json::Value::Null => Value::Nil,
            serde_json::Value::Bool(value) => Value::Bool(*value),
            serde_json::Value::Number(value) => value
                .as_i64()
                .map(Value::Int)
                .or_else(|| value.as_f64().map(Value::Float))
                .unwrap_or(Value::Nil),
            serde_json::Value::String(value) => Value::String(value.clone()),
            serde_json::Value::Array(values) => Value::List(Rc::new(RefCell::new(
                values
                    .iter()
                    .map(|value| self.json_to_value(value))
                    .collect(),
            ))),
            serde_json::Value::Object(values) => Value::Dict(Rc::new(RefCell::new(
                values
                    .iter()
                    .map(|(key, value)| (key.clone(), self.json_to_value(value)))
                    .collect(),
            ))),
        }
    }

    fn consume_fuel(&mut self) -> Result<()> {
        if self.fuel_remaining == 0 {
            bail!("execution limit reached")
        }
        self.fuel_remaining -= 1;
        Ok(())
    }
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.render())
    }
}
