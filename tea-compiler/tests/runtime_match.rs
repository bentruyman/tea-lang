use std::path::PathBuf;

use tea_compiler::{Compilation, CompileOptions, Compiler, SourceFile, SourceId, Vm};

fn compile_program(source: &str) -> anyhow::Result<Compilation> {
    let mut compiler = Compiler::new(CompileOptions::default());
    let source_file = SourceFile::new(SourceId(0), PathBuf::from("match.tea"), source.to_string());
    let compilation = compiler.compile(&source_file)?;
    assert!(
        compiler.diagnostics().is_empty(),
        "expected no diagnostics, found {:?}",
        compiler.diagnostics()
    );
    Ok(compilation)
}

#[test]
fn match_expression_runtime_behaviour() -> anyhow::Result<()> {
    let source = r#"
use assert = "std.assert"

enum Status {
  Ready
  Pending
}

var status = Status.Pending
var label = match status
  case Status.Ready => "go"
  case Status.Pending => "wait"
  case _ => "unknown"
end
assert.assert_eq(label, "wait")

var code = 302
var message = match code
  case 200 => "ok"
  case 301 | 302 => "redirect"
  case _ => "other"
end
assert.assert_eq(message, "redirect")

var calls = 0

def tick() -> Int
  calls = calls + 1
  calls
end

var observed = match tick()
  case 1 => "once"
  case _ => "many"
end
assert.assert_eq(observed, "once")
assert.assert_eq(calls, 1)

var flag = true
var bool_result = match flag
  case true => "ok"
  case false => "not ok"
end
assert.assert_eq(bool_result, "ok")

enum Color {
  Red
  Green
  Blue
}

var color = Color.Red
var output = match color
  case Color.Red => "red"
  case Color.Green => "green"
  case Color.Blue => "blue"
end
assert.assert_eq(output, "red")
"#;

    let compilation = compile_program(source)?;
    let mut vm = Vm::new(&compilation.program);
    vm.run()?;
    Ok(())
}

#[test]
fn match_expression_emits_expected_jumps() -> anyhow::Result<()> {
    let source = r#"
var code = 302
var message = match code
  case 200 => "ok"
  case 301 | 302 => "redirect"
  case _ => "other"
end
message
"#;

    let compilation = compile_program(source)?;
    let instructions: Vec<String> = compilation
        .program
        .chunk
        .instructions
        .iter()
        .map(|instruction| instruction.to_string())
        .collect();

    let jump_if_false_count = instructions
        .iter()
        .filter(|instruction| instruction.starts_with("JUMP_IF_FALSE"))
        .count();
    assert_eq!(
        jump_if_false_count, 3,
        "expected three JumpIfFalse instructions for match arm evaluation, got {instructions:?}"
    );

    assert!(
        instructions
            .iter()
            .any(|instruction| instruction.starts_with("SET_LOCAL")),
        "expected match expression to store the scrutinee in a local slot"
    );

    Ok(())
}

#[test]
fn match_expression_requires_wildcard_arm() {
    let source = r#"
var message = match 1
  case 1 => "one"
end
"#;

    let mut compiler = Compiler::new(CompileOptions::default());
    let source_file = SourceFile::new(
        SourceId(0),
        PathBuf::from("missing_wildcard.tea"),
        source.to_string(),
    );
    let result = compiler.compile(&source_file);
    assert!(
        result.is_err(),
        "expected compilation to fail without wildcard arm"
    );
    let messages: Vec<String> = compiler
        .diagnostics()
        .entries()
        .iter()
        .map(|diagnostic| diagnostic.message.clone())
        .collect();
    assert!(
        messages
            .iter()
            .any(|message| message.contains("match expression is not exhaustive")),
        "expected non-exhaustive diagnostic, got {messages:?}"
    );
}

#[test]
fn match_reports_missing_bool_case() {
    let source = r#"
var message = match true
  case true => "ok"
end
"#;

    let mut compiler = Compiler::new(CompileOptions::default());
    let source_file = SourceFile::new(
        SourceId(0),
        PathBuf::from("missing_bool.tea"),
        source.to_string(),
    );
    let result = compiler.compile(&source_file);
    assert!(
        result.is_err(),
        "expected compilation to fail when bool cases are missing"
    );
    let messages: Vec<String> = compiler
        .diagnostics()
        .entries()
        .iter()
        .map(|diagnostic| diagnostic.message.clone())
        .collect();
    assert!(
        messages
            .iter()
            .any(|message| message.contains("missing `false`")),
        "expected diagnostic to mention missing bool case, got {messages:?}"
    );
}

#[test]
fn match_warns_on_unreachable_arm() {
    let source = r#"
var value = 1
var result = match value
  case _ => "any"
  case 1 => "one"
end
result
"#;

    let mut compiler = Compiler::new(CompileOptions::default());
    let source_file = SourceFile::new(
        SourceId(0),
        PathBuf::from("unreachable.tea"),
        source.to_string(),
    );
    let _ = compiler
        .compile(&source_file)
        .expect("compilation to succeed");
    let messages: Vec<String> = compiler
        .diagnostics()
        .entries()
        .iter()
        .map(|diagnostic| diagnostic.message.clone())
        .collect();
    assert!(
        messages
            .iter()
            .any(|message| message.contains("match arm is unreachable")),
        "expected unreachable arm warning, got {messages:?}"
    );
}

#[test]
fn match_warns_on_duplicate_bool_pattern() {
    let source = r#"
var flag = true
var result = match flag
  case true => "ok"
  case true => "still ok"
  case false => "no"
end
result
"#;

    let mut compiler = Compiler::new(CompileOptions::default());
    let source_file = SourceFile::new(
        SourceId(0),
        PathBuf::from("duplicate_bool.tea"),
        source.to_string(),
    );
    let _ = compiler
        .compile(&source_file)
        .expect("compilation to succeed");
    let messages: Vec<String> = compiler
        .diagnostics()
        .entries()
        .iter()
        .map(|diagnostic| diagnostic.message.clone())
        .collect();
    assert!(
        messages
            .iter()
            .any(|message| message.contains("pattern `true` is unreachable")),
        "expected duplicate pattern warning, got {messages:?}"
    );
}
