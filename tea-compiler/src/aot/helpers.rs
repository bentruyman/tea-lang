//! LLVM code generation helpers to reduce duplication and improve readability.

use anyhow::Result;
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::types::{IntType, StructType};
use inkwell::values::{BasicValue, BasicValueEnum, FunctionValue, InstructionValue, IntValue};

use super::map_builder_error;

/// Tag values for TeaValue union type.
/// Layout: { tag: i32, padding: i32, payload: i64 }
#[repr(u32)]
#[derive(Clone, Copy)]
pub enum TeaValueTag {
    Int = 0,
    Float = 1,
    Bool = 2,
    String = 3,
    List = 4,
    Dict = 5,
    Struct = 6,
    Error = 7,
    Closure = 8,
    Nil = 9,
}

impl TeaValueTag {
    /// Returns the tag value as a u64 for use with LLVM const_int.
    pub fn as_u64(self) -> u64 {
        self as u64
    }
}

/// Build a TeaValue struct with given tag and i64 payload.
///
/// TeaValue layout: { tag: i32, padding: i32, payload: i64 }
/// This uses alloca+store pattern to avoid insertvalue/undef issues with ARM64 ABI.
pub fn build_tea_value<'ctx>(
    context: &'ctx Context,
    builder: &Builder<'ctx>,
    tea_value_type: StructType<'ctx>,
    tag: TeaValueTag,
    payload: IntValue<'ctx>,
    name: &str,
) -> Result<BasicValueEnum<'ctx>> {
    let alloca_name = format!("tea_val_{}", name);
    let loaded_name = format!("tea_val_{}_loaded", name);

    // Allocate TeaValue struct on stack
    let alloca = map_builder_error(builder.build_alloca(tea_value_type, &alloca_name))?;

    // Store tag at field 0
    let tag_ptr =
        map_builder_error(builder.build_struct_gep(tea_value_type, alloca, 0, "tag_ptr"))?;
    map_builder_error(
        builder.build_store(tag_ptr, context.i32_type().const_int(tag.as_u64(), false)),
    )?;

    // Store padding at field 1
    let padding_ptr =
        map_builder_error(builder.build_struct_gep(tea_value_type, alloca, 1, "padding_ptr"))?;
    map_builder_error(builder.build_store(padding_ptr, context.i32_type().const_zero()))?;

    // Store payload at field 2
    let payload_ptr =
        map_builder_error(builder.build_struct_gep(tea_value_type, alloca, 2, "payload_ptr"))?;
    map_builder_error(builder.build_store(payload_ptr, payload))?;

    // Load and return the complete struct
    let loaded = map_builder_error(builder.build_load(tea_value_type, alloca, &loaded_name))?;
    Ok(loaded)
}

/// Add a named function attribute to a function.
pub fn add_function_attr(context: &Context, function: FunctionValue, name: &str) {
    use inkwell::attributes::{Attribute, AttributeLoc};

    let attr = context.create_enum_attribute(Attribute::get_named_enum_kind_id(name), 0);
    function.add_attribute(AttributeLoc::Function, attr);
}

/// Builder for LLVM loop optimization metadata.
///
/// Creates metadata nodes that hint LLVM to vectorize and optimize loops.
pub struct LoopMetadataBuilder<'ctx> {
    context: &'ctx Context,
    nodes: Vec<inkwell::values::MetadataValue<'ctx>>,
    bool_type: IntType<'ctx>,
}

impl<'ctx> LoopMetadataBuilder<'ctx> {
    /// Create a new loop metadata builder.
    pub fn new(context: &'ctx Context, bool_type: IntType<'ctx>) -> Self {
        Self {
            context,
            nodes: Vec::new(),
            bool_type,
        }
    }

    /// Add a boolean metadata node (e.g., "llvm.loop.vectorize.enable" = true).
    pub fn with_bool(mut self, key: &str, value: bool) -> Self {
        let key_md = self.context.metadata_string(key);
        let value_const = self
            .bool_type
            .const_int(if value { 1 } else { 0 }, false)
            .as_basic_value_enum();
        let node = self
            .context
            .metadata_node(&[key_md.into(), value_const.into()]);
        self.nodes.push(node);
        self
    }

    /// Add an i32 metadata node (e.g., "llvm.loop.vectorize.width" = 4).
    pub fn with_i32(mut self, key: &str, value: u32) -> Self {
        let key_md = self.context.metadata_string(key);
        let value_const = self
            .context
            .i32_type()
            .const_int(value as u64, false)
            .as_basic_value_enum();
        let node = self
            .context
            .metadata_node(&[key_md.into(), value_const.into()]);
        self.nodes.push(node);
        self
    }

    /// Attach the loop metadata to an instruction (typically a branch at loop end).
    pub fn attach_to(self, instruction: InstructionValue<'ctx>) {
        let md_kind = self.context.get_kind_id("llvm.loop");

        // Create the loop metadata node that references itself (required by LLVM)
        let loop_id = self.context.metadata_node(&[]);

        // Build the full metadata array: [loop_id, node1, node2, ...]
        let mut md_values: Vec<inkwell::values::MetadataValue<'ctx>> = vec![loop_id];
        md_values.extend(self.nodes);

        let loop_metadata = self
            .context
            .metadata_node(&md_values.iter().map(|n| (*n).into()).collect::<Vec<_>>());

        // Update the loop ID to reference the full metadata
        loop_id.replace_all_uses_with(&loop_metadata);

        // Set the metadata on the instruction
        let _ = instruction.set_metadata(loop_metadata, md_kind);
    }
}
