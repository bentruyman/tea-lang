use std::path::PathBuf;

use tea_compiler::{CompileOptions, Compiler, SourceFile, SourceId};

#[test]
fn allows_unwrap_after_guard_return() {
    let source = r#"
use debug = "std.debug"

def add(a: Int, b: Int) -> Int
  return a + b
end

def demo(x: Int?) -> Int
  var local = x
  if local == nil
    return 0
  end

  add(local!, 2)
end

debug.print(demo(1))
"#;

    let mut compiler = Compiler::new(CompileOptions::default());
    let source_file = SourceFile::new(
        SourceId(0),
        PathBuf::from("guard_return.tea"),
        source.to_string(),
    );
    let result = compiler.compile(&source_file);
    if result.is_err() {
        let messages: Vec<_> = compiler
            .diagnostics()
            .entries()
            .iter()
            .map(|entry| entry.message.clone())
            .collect();
        panic!(
            "expected program to compile without errors; diagnostics: {:?}",
            messages
        );
    }
}

#[test]
fn allows_guard_assignment_to_prove_non_nil() {
    let source = r#"
use debug = "std.debug"

def greeting(name: String?) -> String
  var local = name
  if local == nil
    local = "tea"
  end

  return local!
end

debug.print(greeting(nil))
"#;

    let mut compiler = Compiler::new(CompileOptions::default());
    let source_file = SourceFile::new(
        SourceId(0),
        PathBuf::from("guard_assign.tea"),
        source.to_string(),
    );
    let result = compiler.compile(&source_file);
    if result.is_err() {
        let messages: Vec<_> = compiler
            .diagnostics()
            .entries()
            .iter()
            .map(|entry| entry.message.clone())
            .collect();
        panic!(
            "expected guard assignment to allow unwrap; diagnostics: {:?}",
            messages
        );
    }
}

#[test]
fn coalesce_flows_into_int_arguments() {
    let source = r#"
use debug = "std.debug"

def consume(value: Int) -> Void
  debug.print(value)
end

var number: Int? = nil
consume(number ?? 42)
"#;

    let mut compiler = Compiler::new(CompileOptions::default());
    let source_file = SourceFile::new(
        SourceId(0),
        PathBuf::from("coalesce.tea"),
        source.to_string(),
    );
    let result = compiler.compile(&source_file);
    if result.is_err() {
        let messages: Vec<_> = compiler
            .diagnostics()
            .entries()
            .iter()
            .map(|entry| entry.message.clone())
            .collect();
        panic!(
            "expected coalesce to produce non-optional type for Int consumer; diagnostics: {:?}",
            messages
        );
    }
}

#[test]
fn rejects_unwrap_without_flow_proof() {
    let source = r#"
use debug = "std.debug"

var maybe_name: String? = nil
debug.print(maybe_name!)
"#;

    let mut compiler = Compiler::new(CompileOptions::default());
    let source_file = SourceFile::new(
        SourceId(0),
        PathBuf::from("unwrap_error.tea"),
        source.to_string(),
    );
    let result = compiler.compile(&source_file);
    assert!(result.is_err(), "expected unwrap without guard to fail");
    let messages: Vec<_> = compiler
        .diagnostics()
        .entries()
        .iter()
        .map(|entry| entry.message.as_str())
        .collect();
    assert!(
        messages.iter().any(|message| message
            .contains("cannot unwrap optional 'maybe_name': value may be nil here")),
        "missing unwrap diagnostic: {:?}",
        messages
    );
}
