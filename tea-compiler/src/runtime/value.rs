use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;

#[derive(Debug, Clone)]
pub struct StructTemplate {
    pub name: String,
    pub field_names: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct StructInstance {
    pub template: Rc<StructTemplate>,
    pub fields: Vec<Value>,
}

impl StructInstance {
    pub fn field_index(&self, name: &str) -> Option<usize> {
        self.template
            .field_names
            .iter()
            .position(|field| field == name)
    }

    pub fn get_field(&self, name: &str) -> Option<&Value> {
        self.field_index(name)
            .and_then(|index| self.fields.get(index))
    }
}

#[derive(Debug, Clone)]
pub struct ClosureInstance {
    pub function_index: usize,
    pub captures: Rc<Vec<Value>>,
}

#[derive(Debug, Clone)]
pub struct EnumVariantValue {
    pub enum_name: String,
    pub variant_name: String,
    pub discriminant: usize,
}

#[derive(Debug, Clone)]
pub enum Value {
    Nil,
    Void,
    Int(i64),
    Float(f64),
    Bool(bool),
    String(String),
    Function(usize),
    Closure(Rc<ClosureInstance>),
    List(Rc<Vec<Value>>),
    Dict(Rc<HashMap<String, Value>>),
    Struct(Rc<StructInstance>),
    EnumVariant(Rc<EnumVariantValue>),
}

impl Value {
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Nil => false,
            Value::Void => false,
            Value::Bool(value) => *value,
            _ => true,
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Nil => write!(f, "nil"),
            Value::Void => write!(f, "void"),
            Value::Int(value) => write!(f, "{value}"),
            Value::Float(value) => write!(f, "{value}"),
            Value::Bool(value) => write!(f, "{value}"),
            Value::String(value) => write!(f, "{value}"),
            Value::Function(_) => write!(f, "<function>"),
            Value::Closure(_) => write!(f, "<closure>"),
            Value::List(values) => {
                write!(f, "[")?;
                for (index, value) in values.iter().enumerate() {
                    if index > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{value}")?;
                }
                write!(f, "]")
            }
            Value::Dict(entries) => {
                write!(f, "{{")?;
                for (index, (key, value)) in entries.iter().enumerate() {
                    if index > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{key}: {value}")?;
                }
                write!(f, "}}")
            }
            Value::Struct(instance) => {
                write!(f, "{}(", instance.template.name)?;
                for (index, (field_name, value)) in instance
                    .template
                    .field_names
                    .iter()
                    .zip(instance.fields.iter())
                    .enumerate()
                {
                    if index > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{field_name}: {value}")?;
                }
                write!(f, ")")
            }
            Value::EnumVariant(variant) => {
                write!(f, "{}.{}", variant.enum_name, variant.variant_name)
            }
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Nil, Value::Nil) => true,
            (Value::Void, Value::Void) => true,
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => a == b,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Function(a), Value::Function(b)) => a == b,
            (Value::Closure(a), Value::Closure(b)) => Rc::ptr_eq(a, b),
            (Value::List(a), Value::List(b)) => Rc::ptr_eq(a, b),
            (Value::Dict(a), Value::Dict(b)) => Rc::ptr_eq(a, b),
            (Value::Struct(a), Value::Struct(b)) => Rc::ptr_eq(a, b),
            (Value::EnumVariant(a), Value::EnumVariant(b)) => {
                Rc::ptr_eq(a, b)
                    || (a.enum_name == b.enum_name
                        && a.variant_name == b.variant_name
                        && a.discriminant == b.discriminant)
            }
            _ => false,
        }
    }
}
