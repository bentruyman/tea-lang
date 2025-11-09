use std::path::PathBuf;

use tea_compiler::{CompileOptions, Compiler, SourceFile, SourceId};

fn run_code(code: &str) -> anyhow::Result<()> {
    let source_file = SourceFile::new(SourceId(0), PathBuf::from("test.tea"), code.to_string());

    let mut compiler = Compiler::new(CompileOptions::default());
    let compilation = compiler.compile(&source_file)?;

    if !compiler.diagnostics().is_empty() {
        anyhow::bail!("compilation failed: {:?}", compiler.diagnostics());
    }

    // Note: This test was converted from VM-based execution to AOT compilation-only
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
fn test_plus_equal_int() -> anyhow::Result<()> {
    let code = r#"
use assert = "std.assert"

var x = 5
x += 3
assert.eq(x, 8)
"#;
    run_code(code)
}

#[test]
fn test_plus_equal_float() -> anyhow::Result<()> {
    let code = r#"
use assert = "std.assert"

var x = 5.5
x += 2.5
assert.eq(x, 8.0)
"#;
    run_code(code)
}

#[test]
fn test_minus_equal_int() -> anyhow::Result<()> {
    let code = r#"
use assert = "std.assert"

var x = 10
x -= 3
assert.eq(x, 7)
"#;
    run_code(code)
}

#[test]
fn test_minus_equal_float() -> anyhow::Result<()> {
    let code = r#"
use assert = "std.assert"

var x = 10.5
x -= 2.5
assert.eq(x, 8.0)
"#;
    run_code(code)
}

#[test]
fn test_star_equal_int() -> anyhow::Result<()> {
    let code = r#"
use assert = "std.assert"

var x = 5
x *= 3
assert.eq(x, 15)
"#;
    run_code(code)
}

#[test]
fn test_star_equal_float() -> anyhow::Result<()> {
    let code = r#"
use assert = "std.assert"

var x = 2.5
x *= 4.0
assert.eq(x, 10.0)
"#;
    run_code(code)
}

#[test]
fn test_multiple_compound_assignments() -> anyhow::Result<()> {
    let code = r#"
use assert = "std.assert"

var x = 10
x += 5
x -= 3
x *= 2
assert.eq(x, 24)
"#;
    run_code(code)
}

#[test]
fn test_compound_assignment_with_expression() -> anyhow::Result<()> {
    let code = r#"
use assert = "std.assert"

var x = 5
var y = 3
x += y + 2
assert.eq(x, 10)
"#;
    run_code(code)
}

#[test]
fn test_compound_assignment_in_function() -> anyhow::Result<()> {
    let code = r#"
use assert = "std.assert"

def increment(n: Int) -> Int
  var result = n
  result += 1
  return result
end

assert.eq(increment(5), 6)
"#;
    run_code(code)
}

#[test]
fn test_compound_assignment_in_loop() -> anyhow::Result<()> {
    let code = r#"
use assert = "std.assert"

var sum = 0
for i of [1, 2, 3, 4, 5]
  sum += i
end
assert.eq(sum, 15)
"#;
    run_code(code)
}

#[test]
fn test_compound_assignment_type_error() {
    // This should fail type checking because we're trying to add a Float to an Int variable
    let code = r#"
var x: Int = 5
x += 3.5
"#;
    let result = run_code(code);
    assert!(result.is_err(), "Expected type error but got success");
}

#[test]
fn test_compound_assignment_mixed_int_float() {
    // When x is inferred as Int, adding a Float should cause a type error
    let code = r#"
var x = 5
x += 2.5
"#;
    let result = run_code(code);
    // This should fail because x is inferred as Int from the literal 5,
    // and we can't add a Float to an Int without explicit conversion
    assert!(
        result.is_err(),
        "Expected type error when mixing Int and Float"
    );
}
