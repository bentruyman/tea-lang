use std::path::PathBuf;

use tea_compiler::{CompileOptions, Compiler, SourceFile, SourceId, TestStatus, Vm};

#[test]
fn interpolated_strings_emit_concat_instruction_and_execute() -> anyhow::Result<()> {
    let source = r#"
use assert = "std.assert"

def greet(name: String) -> String
  `Hello, ${name}!`
end

test "greet formats name"
  assert.assert_eq(greet("Tea"), "Hello, Tea!")
end
"#;

    let source_file = SourceFile::new(
        SourceId(0),
        PathBuf::from("strings.tea"),
        source.to_string(),
    );

    let mut compiler = Compiler::new(CompileOptions::default());
    let compilation = compiler.compile(&source_file)?;
    assert!(
        compiler.diagnostics().is_empty(),
        "expected no diagnostics, found {:?}",
        compiler.diagnostics()
    );

    let greet_function = compilation
        .program
        .functions
        .iter()
        .find(|function| function.name == "greet")
        .expect("expected greet function to be emitted");
    let instruction_text: Vec<String> = greet_function
        .chunk
        .instructions
        .iter()
        .map(|instruction| instruction.to_string())
        .collect();
    assert!(
        instruction_text
            .iter()
            .any(|text| text.starts_with("CONCAT_STRINGS")),
        "expected CONCAT_STRINGS instruction in greet(), got {instruction_text:?}"
    );

    let mut vm = Vm::new(&compilation.program);
    let outcomes = vm.run_tests(None, None)?;
    assert_eq!(outcomes.len(), 1, "expected a single test outcome");
    assert!(
        matches!(outcomes[0].status, TestStatus::Passed),
        "interpolation test should pass: {:?}",
        outcomes[0]
    );

    Ok(())
}

#[test]
fn string_concatenation_with_plus_operator() -> anyhow::Result<()> {
    let source = r#"
use assert = "std.assert"

def concat(a: String, b: String) -> String
  a + b
end

def build_string(count: Int) -> String
  var result = ""
  var i = 0
  while i < count
    result = result + "x"
    i = i + 1
  end
  return result
end

test "concatenates two strings"
  assert.assert_eq(concat("hello", "world"), "helloworld")
end

test "concatenates with empty string"
  assert.assert_eq("" + "test", "test")
  assert.assert_eq("test" + "", "test")
end

test "concatenates multiple strings"
  assert.assert_eq("a" + "b" + "c", "abc")
end

test "builds string with loop"
  assert.assert_eq(build_string(5), "xxxxx")
end
"#;

    let source_file = SourceFile::new(
        SourceId(0),
        PathBuf::from("string_concat.tea"),
        source.to_string(),
    );

    let mut compiler = Compiler::new(CompileOptions::default());
    let compilation = compiler.compile(&source_file)?;
    assert!(
        compiler.diagnostics().is_empty(),
        "expected no diagnostics, found {:?}",
        compiler.diagnostics()
    );

    let mut vm = Vm::new(&compilation.program);
    let outcomes = vm.run_tests(None, None)?;
    assert_eq!(outcomes.len(), 4, "expected four test outcomes");

    for outcome in &outcomes {
        assert!(
            matches!(outcome.status, TestStatus::Passed),
            "all string concatenation tests should pass: {:?}",
            outcome
        );
    }

    Ok(())
}
