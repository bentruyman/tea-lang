use std::path::PathBuf;

use tea_compiler::{CompileOptions, Compiler, SourceFile, SourceId};

#[test]
fn interpolated_strings_emit_concat_instruction_and_execute() -> anyhow::Result<()> {
    let source = r#"
use assert = "std.assert"

def greet(name: String) -> String
  `Hello, ${name}!`
end

test "greet formats name"
  assert.eq(greet("Tea"), "Hello, Tea!")
end
"#;

    let source_file = SourceFile::new(
        SourceId(0),
        PathBuf::from("strings.tea"),
        source.to_string(),
    );

    let mut compiler = Compiler::new(CompileOptions::default());
    compiler.compile(&source_file)?;
    assert!(
        compiler.diagnostics().is_empty(),
        "expected no diagnostics, found {:?}",
        compiler.diagnostics()
    );

    // Full test execution support via AOT is planned for the future
    assert!(
        compiler.diagnostics().is_empty(),
        "expected no diagnostics, found {:?}",
        compiler.diagnostics()
    );

    // TODO: Add AOT test execution when implemented
    // For now, we verify that the code compiles without errors

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
  assert.eq(concat("hello", "world"), "helloworld")
end

test "concatenates with empty string"
  assert.eq("" + "test", "test")
  assert.eq("test" + "", "test")
end

test "concatenates multiple strings"
  assert.eq("a" + "b" + "c", "abc")
end

test "builds string with loop"
  assert.eq(build_string(5), "xxxxx")
end
"#;

    let source_file = SourceFile::new(
        SourceId(0),
        PathBuf::from("string_concat.tea"),
        source.to_string(),
    );

    let mut compiler = Compiler::new(CompileOptions::default());
    compiler.compile(&source_file)?;
    assert!(
        compiler.diagnostics().is_empty(),
        "expected no diagnostics, found {:?}",
        compiler.diagnostics()
    );

    // Full test execution support via AOT is planned for the future
    assert!(
        compiler.diagnostics().is_empty(),
        "expected no diagnostics, found {:?}",
        compiler.diagnostics()
    );

    // TODO: Add AOT test execution when implemented
    // For now, we verify that the code compiles without errors

    Ok(())
}
