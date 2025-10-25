use std::path::PathBuf;

use tea_compiler::{CompileOptions, Compiler, SourceFile, SourceId};

fn compile_program(source: &str) -> tea_compiler::Program {
    let mut compiler = Compiler::new(CompileOptions::default());
    let source_file = SourceFile::new(SourceId(0), PathBuf::from("enums.tea"), source.to_string());
    compiler
        .compile(&source_file)
        .expect("source should compile")
        .program
}

#[test]
fn vm_emits_enum_variant_constants() {
    let program = compile_program(
        r#"
enum Color {
  Red
  Green
}

var first = Color.Red
var second = Color.Green
"#,
    );

    let instructions: Vec<String> = program
        .chunk
        .instructions
        .iter()
        .map(|instruction| instruction.to_string())
        .collect();

    assert!(
        !instructions
            .iter()
            .any(|instruction| instruction == "GET_FIELD"),
        "expected enum variant construction to avoid GET_FIELD instructions: {instructions:?}",
    );

    let variant_constants: Vec<String> = program
        .chunk
        .constants
        .iter()
        .map(|value| value.to_string())
        .filter(|value| value.starts_with("Color."))
        .collect();

    assert!(
        variant_constants.contains(&"Color.Red".to_string()),
        "expected Color.Red constant in chunk constants: {variant_constants:?}",
    );
    assert!(
        variant_constants.contains(&"Color.Green".to_string()),
        "expected Color.Green constant in chunk constants: {variant_constants:?}",
    );
}
