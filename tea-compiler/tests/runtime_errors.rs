use std::path::PathBuf;

use anyhow::Result;
use tea_compiler::{Compilation, CompileOptions, Compiler, SourceFile, SourceId, Vm};

fn compile_program(source: &str) -> Result<Compilation> {
    let mut compiler = Compiler::new(CompileOptions::default());
    let source_file = SourceFile::new(SourceId(0), PathBuf::from("errors.tea"), source.to_string());
    match compiler.compile(&source_file) {
        Ok(compilation) => {
            assert!(
                compiler.diagnostics().is_empty(),
                "expected no diagnostics, found {:?}",
                compiler.diagnostics()
            );
            Ok(compilation)
        }
        Err(error) => {
            eprintln!("diagnostics: {:?}", compiler.diagnostics());
            Err(error)
        }
    }
}

#[test]
fn try_catch_fallback_handles_errors() -> Result<()> {
    let source = r#"
use assert = "std.assert"

error ExampleError {
  Failure(message: String)
}

def maybe_fail(flag: Bool) -> Int ! ExampleError.Failure
  if flag
    throw ExampleError.Failure("broken")
  end
  return 7
end

var handled = try maybe_fail(true) catch 42
assert.assert_eq(handled, 42)

var passthrough = try maybe_fail(false) catch 13
assert.assert_eq(passthrough, 7)
"#;

    let compilation = compile_program(source)?;
    let mut vm = Vm::new(&compilation.program);
    vm.run()?;
    Ok(())
}

#[test]
fn try_expressions_merge_error_unions() -> Result<()> {
    let source = r#"
use assert = "std.assert"

error NetError {
  Timeout
}

error ParseError {
  Invalid
}

def fetch(fail: Bool) -> Int ! NetError.Timeout
  if fail
    throw NetError.Timeout()
  end
  return 3
end

def parse(fail: Bool) -> Int ! ParseError.Invalid
  if fail
    throw ParseError.Invalid()
  end
  return 4
end

def combined(fetch_fail: Bool, parse_fail: Bool) -> Int ! {
  NetError.Timeout,
  ParseError.Invalid
}
  var total = 0
  total = total + try fetch(fetch_fail)
  total = total + try parse(parse_fail)
  return total
end

var timeout = try combined(true, false) catch err
  case is NetError.Timeout => -1
  case is ParseError.Invalid => -2
  case _ => -3
end
assert.assert_eq(timeout, -1)

var invalid = try combined(false, true) catch err
  case is NetError.Timeout => -1
  case is ParseError.Invalid => -2
  case _ => -3
end
assert.assert_eq(invalid, -2)

var success = try combined(false, false) catch err
  case is NetError.Timeout => -1
  case is ParseError.Invalid => -2
  case _ => -3
end
assert.assert_eq(success, 7)
"#;

    let compilation = compile_program(source)?;
    let mut vm = Vm::new(&compilation.program);
    vm.run()?;
    Ok(())
}

#[test]
fn try_catch_arms_match_variants() -> Result<()> {
    let source = r#"
use assert = "std.assert"

error FsError {
  NotFound(path: String)
  Permission
}

def read(path: String) -> String ! {
  FsError.NotFound,
  FsError.Permission
}
  if path == "missing"
    throw FsError.NotFound(path)
  end
  if path == "forbidden"
    throw FsError.Permission()
  end
  return "ok"
end

var missing = try read("missing") catch err
  case is FsError.NotFound => `Missing: ${err.path}`
  case is FsError.Permission => "Permission denied"
  case _ => "unknown"
end
assert.assert_eq(missing, "Missing: missing")

var forbidden = try read("forbidden") catch err
  case is FsError.NotFound => `Missing: ${err.path}`
  case is FsError.Permission => "Permission denied"
  case _ => "unknown"
end
assert.assert_eq(forbidden, "Permission denied")

var success = try read("file.txt") catch err
  case is FsError.NotFound => `Missing: ${err.path}`
  case is FsError.Permission => "Permission denied"
  case _ => "unknown"
end
assert.assert_eq(success, "ok")
"#;

    let compilation = compile_program(source)?;
    let mut vm = Vm::new(&compilation.program);
    vm.run()?;
    Ok(())
}

#[test]
fn catch_arms_rethrow_unhandled_errors() -> Result<()> {
    let source = r#"
error NetError {
  Timeout
  Disconnected
}

def fail() -> Nil ! NetError.Timeout
  throw NetError.Timeout()
end

var status = try fail() catch err
  case is NetError.Disconnected => nil
end
status
"#;

    let compilation = compile_program(source)?;
    let mut vm = Vm::new(&compilation.program);
    let error = match vm.run() {
        Ok(_) => anyhow::bail!("expected runtime error but execution succeeded"),
        Err(error) => error,
    };
    assert!(
        error.to_string().contains("NetError.Timeout"),
        "expected error to mention NetError.Timeout, got {}",
        error
    );
    Ok(())
}
