use std::path::PathBuf;

use tea_compiler::{CompileOptions, Compiler, SourceFile, SourceId};

fn compile_program(source: &str) -> tea_compiler::Program {
    let mut compiler = Compiler::new(CompileOptions::default());
    let source_file = SourceFile::new(
        SourceId(0),
        PathBuf::from("structs.tea"),
        source.to_string(),
    );
    compiler
        .compile(&source_file)
        .expect("source should compile")
        .program
}

fn parse_constant_index(instruction: &str) -> Option<usize> {
    instruction
        .strip_prefix("CONSTANT ")
        .and_then(|index| index.parse::<usize>().ok())
}

#[test]
fn vm_emits_struct_constructor_and_field_access() {
    let program = compile_program(
        r#"
struct User
  name: String
  age: Int
end

var user = User("Ada", 37)
var name = user.name
"#,
    );

    let instructions: Vec<String> = program
        .chunk
        .instructions
        .iter()
        .map(|instruction| instruction.to_string())
        .collect();

    assert!(
        instructions
            .iter()
            .any(|instruction| instruction == "MAKE_STRUCT_POS 0"),
        "expected MAKE_STRUCT_POS 0 in instruction stream: {instructions:?}",
    );

    let get_field_constant = instructions
        .windows(2)
        .find_map(|window| {
            let [first, second] = window else {
                return None;
            };
            if second == "GET_FIELD" {
                parse_constant_index(first)
            } else {
                None
            }
        })
        .expect("expected constant before GET_FIELD");

    let field_name = program.chunk.constants[get_field_constant].to_string();
    assert_eq!(field_name, "name");
}

#[test]
fn vm_emits_named_struct_constructor_operands() {
    let program = compile_program(
        r#"
struct User
  name: String
  age: Int
end

var user = User(name: "Ada", age: 37)
"#,
    );

    let instructions: Vec<String> = program
        .chunk
        .instructions
        .iter()
        .map(|instruction| instruction.to_string())
        .collect();

    let mut field_operands = Vec::new();
    for instruction in &instructions {
        if instruction.starts_with("MAKE_STRUCT_NAMED ") {
            assert_eq!(instruction, "MAKE_STRUCT_NAMED 0");
            break;
        }
        if let Some(index) = parse_constant_index(instruction) {
            field_operands.push(program.chunk.constants[index].to_string());
        }
    }

    assert!(
        field_operands.contains(&"name".to_string()),
        "expected field constant 'name' before MAKE_STRUCT_NAMED, saw {field_operands:?}",
    );
    assert!(
        field_operands.contains(&"age".to_string()),
        "expected field constant 'age' before MAKE_STRUCT_NAMED, saw {field_operands:?}",
    );

    let template = program
        .structs
        .first()
        .expect("expected struct template for User");
    let field_names: Vec<&str> = template
        .field_names
        .iter()
        .map(|name| name.as_str())
        .collect();
    assert_eq!(field_names, ["name", "age"]);
}
