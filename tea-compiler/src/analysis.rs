use std::collections::HashMap;

use crate::ast::SourceSpan;
use crate::resolver::ModuleAliasBinding;
use crate::stdlib::{self, StdFunction, StdType};
use crate::typechecker::{
    EnumDefinition, ErrorDefinition, FunctionInstance, StructDefinition, StructInstance, Type,
    TypeChecker,
};

fn describe_std_type(ty: StdType) -> String {
    match ty {
        StdType::Any => "Unknown".into(),
        StdType::Bool => "Bool".into(),
        StdType::Int => "Int".into(),
        StdType::Float => "Float".into(),
        StdType::String => "String".into(),
        StdType::List => "List[Unknown]".into(),
        StdType::Dict => "Dict[String, Unknown]".into(),
        StdType::Struct => "Struct".into(),
        StdType::Nil => "Nil".into(),
        StdType::Void => "Void".into(),
    }
}

fn describe_std_function(function: &StdFunction) -> String {
    let params = if function.params.is_empty() {
        "()".to_string()
    } else {
        let joined = function
            .params
            .iter()
            .map(|param| describe_std_type(*param))
            .collect::<Vec<_>>()
            .join(", ");
        format!("({joined})")
    };
    format!(
        "Func{params} -> {}",
        describe_std_type(function.return_type)
    )
}

fn enrich_module_aliases(
    module_aliases: &mut HashMap<String, ModuleAliasBinding>,
    alias_exports: &HashMap<String, Vec<String>>,
    alias_export_renames: &HashMap<String, HashMap<String, String>>,
    alias_export_docstrings: &HashMap<String, HashMap<String, String>>,
    global_binding_types: &HashMap<String, Type>,
    enum_definitions: &HashMap<String, EnumDefinition>,
    struct_definitions: &HashMap<String, StructDefinition>,
) {
    for (alias, exports) in alias_exports {
        if let Some(binding) = module_aliases.get_mut(alias) {
            binding.exports = exports.clone();
            binding.export_types.clear();
            binding.export_docs.clear();

            if let Some(renames) = alias_export_renames.get(alias) {
                for export in &binding.exports {
                    if let Some(renamed) = renames.get(export) {
                        if let Some(ty) = global_binding_types.get(renamed) {
                            binding.export_types.insert(export.clone(), ty.describe());
                        } else if enum_definitions.contains_key(renamed) {
                            binding
                                .export_types
                                .insert(export.clone(), "Enum".to_string());
                        } else if struct_definitions.contains_key(renamed) {
                            binding
                                .export_types
                                .insert(export.clone(), "Struct".to_string());
                        }
                    }
                }
            }

            if let Some(docs) = alias_export_docstrings.get(alias) {
                for export in &binding.exports {
                    if let Some(doc) = docs.get(export) {
                        binding.export_docs.insert(export.clone(), doc.clone());
                    }
                }
            }
        }
    }

    for binding in module_aliases.values_mut() {
        if !binding.module_path.starts_with("std.") {
            continue;
        }

        if let Some(std_module) = stdlib::find_module(&binding.module_path) {
            if binding.exports.is_empty() {
                binding.exports = std_module
                    .functions
                    .iter()
                    .map(|func| func.name.to_string())
                    .collect();
                binding.export_types = std_module
                    .functions
                    .iter()
                    .map(|func| (func.name.to_string(), describe_std_function(func)))
                    .collect();
                binding.export_docs = std_module
                    .functions
                    .iter()
                    .map(|func| (func.name.to_string(), func.docstring.to_string()))
                    .collect();
            }

            if binding.docstring.is_none() && !std_module.docstring.is_empty() {
                binding.docstring = Some(std_module.docstring.to_string());
            }
        }
    }
}

#[derive(Debug)]
pub struct SemanticAnalysis {
    module_aliases: HashMap<String, ModuleAliasBinding>,
    binding_type_descriptions: HashMap<SourceSpan, String>,
    argument_type_descriptions: HashMap<SourceSpan, String>,
    match_exhaustiveness: HashMap<SourceSpan, Vec<String>>,
    lambda_captures: HashMap<usize, Vec<String>>,
    lambda_types: HashMap<usize, Type>,
    function_instances: HashMap<String, Vec<FunctionInstance>>,
    struct_instances: HashMap<String, Vec<StructInstance>>,
    function_call_metadata: HashMap<SourceSpan, (String, FunctionInstance)>,
    struct_call_metadata: HashMap<SourceSpan, (String, StructInstance)>,
    binding_types: HashMap<SourceSpan, Type>,
    type_test_metadata: HashMap<SourceSpan, Type>,
    struct_definitions: HashMap<String, StructDefinition>,
    error_definitions: HashMap<String, ErrorDefinition>,
}

impl SemanticAnalysis {
    pub(crate) fn from_parts(
        lambda_captures: HashMap<usize, Vec<String>>,
        mut module_aliases: HashMap<String, ModuleAliasBinding>,
        type_checker: &TypeChecker,
        alias_exports: &HashMap<String, Vec<String>>,
        alias_export_renames: &HashMap<String, HashMap<String, String>>,
        alias_export_docstrings: &HashMap<String, HashMap<String, String>>,
    ) -> Self {
        let binding_types = type_checker.binding_types().clone();
        let argument_expected_types = type_checker.argument_expected_types().clone();
        let struct_definitions = type_checker.struct_definitions();
        let enum_definitions = type_checker.enum_definitions();
        let global_binding_types = type_checker.global_binding_types();

        enrich_module_aliases(
            &mut module_aliases,
            alias_exports,
            alias_export_renames,
            alias_export_docstrings,
            &global_binding_types,
            &enum_definitions,
            &struct_definitions,
        );

        let binding_type_descriptions = binding_types
            .iter()
            .map(|(span, ty)| (*span, ty.describe()))
            .collect();
        let argument_type_descriptions = argument_expected_types
            .iter()
            .map(|(span, ty)| (*span, ty.describe()))
            .collect();

        Self {
            module_aliases,
            binding_type_descriptions,
            argument_type_descriptions,
            match_exhaustiveness: type_checker.match_exhaustiveness().clone(),
            lambda_captures,
            lambda_types: type_checker.lambda_types().clone(),
            function_instances: type_checker.function_instances().clone(),
            struct_instances: type_checker.struct_instances().clone(),
            function_call_metadata: type_checker.function_call_metadata().clone(),
            struct_call_metadata: type_checker.struct_call_metadata().clone(),
            binding_types,
            type_test_metadata: type_checker.type_test_metadata().clone(),
            struct_definitions,
            error_definitions: type_checker.error_definitions(),
        }
    }

    pub fn module_aliases(&self) -> &HashMap<String, ModuleAliasBinding> {
        &self.module_aliases
    }

    pub fn binding_type_descriptions(&self) -> &HashMap<SourceSpan, String> {
        &self.binding_type_descriptions
    }

    pub fn argument_type_descriptions(&self) -> &HashMap<SourceSpan, String> {
        &self.argument_type_descriptions
    }

    pub fn match_exhaustiveness(&self) -> &HashMap<SourceSpan, Vec<String>> {
        &self.match_exhaustiveness
    }

    pub(crate) fn lambda_captures(&self) -> &HashMap<usize, Vec<String>> {
        &self.lambda_captures
    }

    pub(crate) fn lambda_types(&self) -> &HashMap<usize, Type> {
        &self.lambda_types
    }

    pub(crate) fn function_instances(&self) -> &HashMap<String, Vec<FunctionInstance>> {
        &self.function_instances
    }

    pub(crate) fn struct_instances(&self) -> &HashMap<String, Vec<StructInstance>> {
        &self.struct_instances
    }

    pub(crate) fn function_call_metadata(
        &self,
    ) -> &HashMap<SourceSpan, (String, FunctionInstance)> {
        &self.function_call_metadata
    }

    pub(crate) fn struct_call_metadata(&self) -> &HashMap<SourceSpan, (String, StructInstance)> {
        &self.struct_call_metadata
    }

    pub(crate) fn typed_binding_types(&self) -> &HashMap<SourceSpan, Type> {
        &self.binding_types
    }

    pub(crate) fn typed_type_test_metadata(&self) -> &HashMap<SourceSpan, Type> {
        &self.type_test_metadata
    }

    pub(crate) fn struct_definitions(&self) -> &HashMap<String, StructDefinition> {
        &self.struct_definitions
    }

    pub(crate) fn error_definitions(&self) -> &HashMap<String, ErrorDefinition> {
        &self.error_definitions
    }
}
