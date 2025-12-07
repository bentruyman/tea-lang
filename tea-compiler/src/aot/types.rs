use anyhow::{bail, Result};
use inkwell::values::{
    BasicValueEnum, FloatValue, GlobalValue, IntValue, PointerValue, StructValue,
};

use crate::typechecker::{StructType, Type};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ValueType {
    Int,
    Float,
    Bool,
    String,
    List(Box<ValueType>),
    Dict(Box<ValueType>),
    Function(Vec<ValueType>, Box<ValueType>),
    Struct(String),
    Error {
        error_name: String,
        variant_name: Option<String>,
    },
    Optional(Box<ValueType>),
    Void,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ErrorHandlingMode {
    Propagate,
    Capture,
}

#[derive(Clone)]
pub struct LambdaSignature {
    pub param_types: Vec<ValueType>,
    pub return_type: ValueType,
}

#[derive(Clone)]
pub struct FunctionSignature<'ctx> {
    pub value: inkwell::values::FunctionValue<'ctx>,
    pub return_type: ValueType,
    pub param_types: Vec<ValueType>,
    pub can_throw: bool,
}

#[derive(Clone)]
pub struct LocalVariable<'ctx> {
    /// For mutable variables: pointer to stack allocation
    /// For immutable parameters: None (use SSA value directly)
    pub pointer: Option<PointerValue<'ctx>>,
    /// For immutable parameters: the SSA value
    /// For mutable variables: None (load from pointer)
    pub value: Option<BasicValueEnum<'ctx>>,
    pub ty: ValueType,
    pub mutable: bool,
}

pub struct StructLowering<'ctx> {
    pub field_names: Vec<String>,
    pub field_types: Vec<ValueType>,
    pub template_global: Option<GlobalValue<'ctx>>,
    pub field_names_global: Option<GlobalValue<'ctx>>,
    pub template_pointer: Option<PointerValue<'ctx>>,
}

impl<'ctx> StructLowering<'ctx> {
    pub fn new() -> Self {
        Self {
            field_names: Vec::new(),
            field_types: Vec::new(),
            template_global: None,
            field_names_global: None,
            template_pointer: None,
        }
    }

    pub fn field_index(&self, name: &str) -> Option<usize> {
        self.field_names.iter().position(|field| field == name)
    }
}

impl Default for StructLowering<'_> {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone)]
pub struct ErrorVariantLowering<'ctx> {
    pub field_names: Vec<String>,
    pub field_types: Vec<ValueType>,
    pub template_global: Option<GlobalValue<'ctx>>,
    pub field_names_global: Option<GlobalValue<'ctx>>,
    pub template_pointer: Option<PointerValue<'ctx>>,
}

impl<'ctx> ErrorVariantLowering<'ctx> {
    pub fn new() -> Self {
        Self {
            field_names: Vec::new(),
            field_types: Vec::new(),
            template_global: None,
            field_names_global: None,
            template_pointer: None,
        }
    }
}

impl Default for ErrorVariantLowering<'_> {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone)]
pub struct GlobalBindingSlot<'ctx> {
    pub pointer: GlobalValue<'ctx>,
    pub ty: ValueType,
    pub mutable: bool,
    pub initialized: bool,
}

#[derive(Clone)]
pub enum ExprValue<'ctx> {
    Int(IntValue<'ctx>),
    Float(FloatValue<'ctx>),
    Bool(IntValue<'ctx>),
    String(PointerValue<'ctx>),
    List {
        pointer: PointerValue<'ctx>,
        element_type: Box<ValueType>,
    },
    Dict {
        pointer: PointerValue<'ctx>,
        value_type: Box<ValueType>,
    },
    Struct {
        pointer: PointerValue<'ctx>,
        struct_name: String,
    },
    Error {
        pointer: PointerValue<'ctx>,
        error_name: String,
        variant_name: Option<String>,
    },
    Closure {
        pointer: PointerValue<'ctx>,
        param_types: Vec<ValueType>,
        return_type: Box<ValueType>,
    },
    Optional {
        value: StructValue<'ctx>,
        inner: Box<ValueType>,
    },
    Void,
}

impl<'ctx> ExprValue<'ctx> {
    pub fn ty(&self) -> ValueType {
        match self {
            ExprValue::Int(_) => ValueType::Int,
            ExprValue::Float(_) => ValueType::Float,
            ExprValue::Bool(_) => ValueType::Bool,
            ExprValue::String(_) => ValueType::String,
            ExprValue::List { element_type, .. } => ValueType::List(element_type.clone()),
            ExprValue::Dict { value_type, .. } => ValueType::Dict(value_type.clone()),
            ExprValue::Struct { struct_name, .. } => ValueType::Struct(struct_name.clone()),
            ExprValue::Error {
                error_name,
                variant_name,
                ..
            } => ValueType::Error {
                error_name: error_name.clone(),
                variant_name: variant_name.clone(),
            },
            ExprValue::Closure {
                param_types,
                return_type,
                ..
            } => ValueType::Function(param_types.clone(), return_type.clone()),
            ExprValue::Optional { inner, .. } => ValueType::Optional(inner.clone()),
            ExprValue::Void => ValueType::Void,
        }
    }

    pub fn into_basic_value(self) -> Option<BasicValueEnum<'ctx>> {
        match self {
            ExprValue::Int(v) => Some(v.into()),
            ExprValue::Float(v) => Some(v.into()),
            ExprValue::Bool(v) => Some(v.into()),
            ExprValue::String(ptr) => Some(ptr.into()),
            ExprValue::List { pointer, .. } => Some(pointer.into()),
            ExprValue::Dict { pointer, .. } => Some(pointer.into()),
            ExprValue::Struct { pointer, .. } => Some(pointer.into()),
            ExprValue::Error { pointer, .. } => Some(pointer.into()),
            ExprValue::Closure { pointer, .. } => Some(pointer.into()),
            ExprValue::Optional { value, .. } => Some(value.into()),
            ExprValue::Void => None,
        }
    }

    pub fn into_int(self) -> Result<IntValue<'ctx>> {
        match self {
            ExprValue::Int(v) => Ok(v),
            _ => bail!("expected Int value"),
        }
    }

    pub fn into_bool(self) -> Result<IntValue<'ctx>> {
        match self {
            ExprValue::Bool(v) => Ok(v),
            _ => bail!("expected Bool value"),
        }
    }

    pub fn into_string(self) -> Result<PointerValue<'ctx>> {
        match self {
            ExprValue::String(ptr) => Ok(ptr),
            _ => bail!("expected String value"),
        }
    }
}

pub(crate) fn format_type_name(ty: &Type) -> String {
    match ty {
        Type::Bool => "Bool".to_string(),
        Type::Int => "Int".to_string(),
        Type::Float => "Float".to_string(),
        Type::String => "String".to_string(),
        Type::Nil => "Nil".to_string(),
        Type::Void => "Void".to_string(),
        Type::List(inner) => format!("List[{}]", format_type_name(inner)),
        Type::Dict(inner) => format!("Dict[String, {}]", format_type_name(inner)),
        Type::Optional(inner) => format!("{}?", format_type_name(inner)),
        Type::Function(params, return_type) => {
            let param_str = if params.is_empty() {
                "()".to_string()
            } else {
                let joined = params
                    .iter()
                    .map(|param| format_type_name(param))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("({joined})")
            };
            format!("Func{param_str} -> {}", format_type_name(return_type))
        }
        Type::Struct(struct_type) => format_struct_type_name(struct_type),
        Type::Enum(enum_type) => enum_type.name.clone(),
        Type::Union(union_type) => union_type.name.clone(),
        Type::Error(error_type) => match &error_type.variant {
            Some(variant) => format!("{}.{}", error_type.name, variant),
            None => error_type.name.clone(),
        },
        Type::GenericParameter(name) => name.clone(),
        Type::Unknown => "Unknown".to_string(),
    }
}

pub(crate) fn format_struct_type_name(struct_type: &StructType) -> String {
    if struct_type.type_arguments.is_empty() {
        struct_type.name.clone()
    } else {
        let args = struct_type
            .type_arguments
            .iter()
            .map(|arg| format_type_name(arg))
            .collect::<Vec<_>>()
            .join(",");
        format!("{}[{}]", struct_type.name, args)
    }
}

pub(crate) fn type_to_value_type(ty: &Type) -> Result<ValueType> {
    match ty {
        Type::Bool => Ok(ValueType::Bool),
        Type::Int => Ok(ValueType::Int),
        Type::Float => Ok(ValueType::Float),
        Type::String => Ok(ValueType::String),
        Type::Nil => Ok(ValueType::Void),
        Type::Void => Ok(ValueType::Void),
        Type::List(inner) => Ok(ValueType::List(Box::new(type_to_value_type(inner)?))),
        Type::Function(params, return_type) => {
            let mut lowered_params = Vec::with_capacity(params.len());
            for param in params {
                lowered_params.push(type_to_value_type(param)?);
            }
            Ok(ValueType::Function(
                lowered_params,
                Box::new(type_to_value_type(return_type)?),
            ))
        }
        Type::Struct(struct_type) => Ok(ValueType::Struct(format_struct_type_name(struct_type))),
        Type::Enum(enum_type) => bail!(format!(
            "LLVM backend does not yet support enums like '{}'",
            enum_type.name
        )),
        Type::Union(union_type) => bail!(format!(
            "LLVM backend does not yet support union '{}'",
            union_type.name
        )),
        Type::GenericParameter(name) => {
            bail!(format!(
                "cannot lower generic parameter '{}' in LLVM backend",
                name
            ))
        }
        Type::Dict(inner) => Ok(ValueType::Dict(Box::new(type_to_value_type(inner)?))),
        Type::Optional(inner) => Ok(ValueType::Optional(Box::new(type_to_value_type(inner)?))),
        Type::Error(error_type) => Ok(ValueType::Error {
            error_name: error_type.name.clone(),
            variant_name: error_type.variant.clone(),
        }),
        Type::Unknown => bail!("cannot lower Unknown type in LLVM backend"),
    }
}

pub fn sanitize_symbol_component(component: &str) -> String {
    component
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

pub(crate) fn mangle_function_name(name: &str, type_arguments: &[Type]) -> String {
    if type_arguments.is_empty() {
        return name.to_string();
    }
    let components: Vec<String> = type_arguments
        .iter()
        .map(|ty| sanitize_symbol_component(&format_type_name(ty)))
        .collect();
    format!("{}$g{}", name, components.join("$"))
}
