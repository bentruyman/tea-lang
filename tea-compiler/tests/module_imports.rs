use std::fs;

use anyhow::Result;
#[cfg(feature = "llvm-backend")]
use tea_compiler::aot;
use tea_compiler::{CompileOptions, Compiler, SourceFile, SourceId};
use tempfile::tempdir;

#[test]
fn relative_module_exports_require_qualified_access() -> Result<()> {
    let dir = tempdir()?;
    let helper_path = dir.path().join("helper.tea");
    fs::write(
        &helper_path,
        r#"
pub const SCALE: Int = 3

pub struct Box {
  value: Int
}

pub def wrap(value: Int) -> Box
  Box(value: value * SCALE)
end
"#,
    )?;

    let main_source = r#"
use helper from "./helper"

def build_box(value: Int) -> helper.Box
  helper.wrap(value)
end

var box = build_box(5)
box.value + helper.SCALE
"#;

    let main_path = dir.path().join("main.tea");
    fs::write(&main_path, main_source)?;

    let source_file = SourceFile::new(SourceId(0), main_path.clone(), main_source.to_string());
    let mut compiler = Compiler::new(CompileOptions::default());
    compiler.compile(&source_file)?;
    assert!(
        compiler.diagnostics().is_empty(),
        "expected no diagnostics, found {:?}",
        compiler.diagnostics()
    );

    Ok(())
}

#[test]
fn module_imports_respect_public_visibility() -> Result<()> {
    let dir = tempdir()?;
    let helper_path = dir.path().join("helper.tea");
    fs::write(
        &helper_path,
        r#"
pub def greet(name: String) -> String
  secret(name)
end

def secret(name: String) -> String
  "psst #{name}"
end
"#,
    )?;

    let main_source = r#"
use helper from "./helper"

var greeting = helper.greet("tea")
helper.secret("tea")
"#;

    let main_path = dir.path().join("main.tea");
    fs::write(&main_path, main_source)?;

    let source_file = SourceFile::new(SourceId(0), main_path.clone(), main_source.to_string());
    let mut compiler = Compiler::new(CompileOptions::default());
    let compilation = compiler.compile(&source_file);
    assert!(
        compilation.is_err(),
        "expected compilation to fail due to private function access"
    );

    let diagnostics = compiler.diagnostics().entries();
    assert!(
        diagnostics.iter().any(|diagnostic| diagnostic
            .message
            .contains("module 'helper' has no export named 'secret'")),
        "expected missing export diagnostic, found {:?}",
        diagnostics
    );

    Ok(())
}

#[test]
fn private_module_constants_and_types_are_not_exported() -> Result<()> {
    let dir = tempdir()?;
    let helper_path = dir.path().join("helper.tea");
    fs::write(
        &helper_path,
        r#"
const SCALE: Int = 3

struct Box {
  value: Int
}

pub def wrap(value: Int) -> Box
  Box(value: value * SCALE)
end
"#,
    )?;

    let main_source = r#"
use helper from "./helper"

var box = helper.wrap(5)
helper.SCALE
"#;

    let main_path = dir.path().join("main.tea");
    fs::write(&main_path, main_source)?;

    let source_file = SourceFile::new(SourceId(0), main_path.clone(), main_source.to_string());
    let mut compiler = Compiler::new(CompileOptions::default());
    let compilation = compiler.compile(&source_file);
    assert!(
        compilation.is_err(),
        "expected compilation to fail due to private constant access"
    );

    let diagnostics = compiler.diagnostics().entries();
    assert!(
        diagnostics.iter().any(|diagnostic| diagnostic
            .message
            .contains("module 'helper' has no export named 'SCALE'")),
        "expected missing export diagnostic, found {:?}",
        diagnostics
    );

    Ok(())
}

#[test]
fn private_module_types_are_not_exported_in_annotations() -> Result<()> {
    let dir = tempdir()?;
    let helper_path = dir.path().join("helper.tea");
    fs::write(
        &helper_path,
        r#"
struct Box {
  value: Int
}

pub def wrap(value: Int) -> Box
  Box(value: value)
end
"#,
    )?;

    let main_source = r#"
use helper from "./helper"

def take_box(box: helper.Box) -> Int
  box.value
end
"#;

    let main_path = dir.path().join("main.tea");
    fs::write(&main_path, main_source)?;

    let source_file = SourceFile::new(SourceId(0), main_path.clone(), main_source.to_string());
    let mut compiler = Compiler::new(CompileOptions::default());
    let compilation = compiler.compile(&source_file);
    assert!(
        compilation.is_err(),
        "expected compilation to fail due to private type access"
    );

    let diagnostics = compiler.diagnostics().entries();
    assert!(
        diagnostics.iter().any(|diagnostic| diagnostic
            .message
            .contains("module 'helper' has no export named 'Box'")),
        "expected missing export diagnostic, found {:?}",
        diagnostics
    );

    Ok(())
}

#[test]
fn public_module_types_work_in_statement_match_patterns() -> Result<()> {
    let dir = tempdir()?;
    let helper_path = dir.path().join("helper.tea");
    fs::write(
        &helper_path,
        r#"
pub struct Box {
  value: Int
}

pub def wrap(value: Int) -> Box
  Box(value: value)
end
"#,
    )?;

    let main_source = r#"
use helper from "./helper"

match helper.wrap(5)
  case is helper.Box
    @println("box")
  case _
    @println("other")
end
"#;

    let main_path = dir.path().join("main.tea");
    fs::write(&main_path, main_source)?;

    let source_file = SourceFile::new(SourceId(0), main_path.clone(), main_source.to_string());
    let mut compiler = Compiler::new(CompileOptions::default());
    compiler.compile(&source_file)?;
    assert!(
        compiler.diagnostics().is_empty(),
        "expected no diagnostics, found {:?}",
        compiler.diagnostics()
    );

    Ok(())
}

#[test]
fn public_module_errors_work_in_catch_type_patterns() -> Result<()> {
    let dir = tempdir()?;
    let helper_path = dir.path().join("helper.tea");
    fs::write(
        &helper_path,
        r#"
pub error MyError {
  Failure
}

pub def fail(flag: Bool) -> Int ! MyError.Failure
  if flag
    throw MyError.Failure()
  end

  return 0
end
"#,
    )?;

    let main_source = r#"
use helper from "./helper"

var recovered = try helper.fail(true) catch err
  case is helper.MyError.Failure => 0
end

recovered
"#;

    let main_path = dir.path().join("main.tea");
    fs::write(&main_path, main_source)?;

    let source_file = SourceFile::new(SourceId(0), main_path.clone(), main_source.to_string());
    let mut compiler = Compiler::new(CompileOptions::default());
    compiler.compile(&source_file)?;
    assert!(
        compiler.diagnostics().is_empty(),
        "expected no diagnostics, found {:?}",
        compiler.diagnostics()
    );

    Ok(())
}

#[test]
fn public_module_errors_work_in_error_annotations() -> Result<()> {
    let dir = tempdir()?;
    let helper_path = dir.path().join("helper.tea");
    fs::write(
        &helper_path,
        r#"
pub error MyError
"#,
    )?;

    let main_source = r#"
use helper from "./helper"

def fail() -> Void !helper.MyError
  return
end
"#;

    let main_path = dir.path().join("main.tea");
    fs::write(&main_path, main_source)?;

    let source_file = SourceFile::new(SourceId(0), main_path.clone(), main_source.to_string());
    let mut compiler = Compiler::new(CompileOptions::default());
    compiler.compile(&source_file)?;
    assert!(
        compiler.diagnostics().is_empty(),
        "expected no diagnostics, found {:?}",
        compiler.diagnostics()
    );

    Ok(())
}

#[test]
fn public_functions_can_expose_imported_public_errors() -> Result<()> {
    let dir = tempdir()?;
    let child_path = dir.path().join("child.tea");
    fs::write(
        &child_path,
        r#"
pub error MyError
"#,
    )?;

    let helper_path = dir.path().join("helper.tea");
    fs::write(
        &helper_path,
        r#"
use child from "./child"

pub def fail() -> Void !child.MyError
  return
end
"#,
    )?;

    let main_source = r#"
use helper from "./helper"
use child from "./child"

def fail_again() -> Void !child.MyError
  helper.fail()
  return
end
"#;

    let main_path = dir.path().join("main.tea");
    fs::write(&main_path, main_source)?;

    let source_file = SourceFile::new(SourceId(0), main_path.clone(), main_source.to_string());
    let mut compiler = Compiler::new(CompileOptions::default());
    compiler.compile(&source_file)?;
    assert!(
        compiler.diagnostics().is_empty(),
        "expected no diagnostics, found {:?}",
        compiler.diagnostics()
    );

    Ok(())
}

#[test]
fn public_functions_cannot_expose_private_types() -> Result<()> {
    let dir = tempdir()?;
    let helper_path = dir.path().join("helper.tea");
    fs::write(
        &helper_path,
        r#"
struct Box {
  value: Int
}

pub def wrap(value: Int) -> Box
  Box(value: value)
end
"#,
    )?;

    let main_source = r#"
use helper from "./helper"

var box = helper.wrap(5)
box.value
"#;

    let main_path = dir.path().join("main.tea");
    fs::write(&main_path, main_source)?;

    let source_file = SourceFile::new(SourceId(0), main_path.clone(), main_source.to_string());
    let mut compiler = Compiler::new(CompileOptions::default());
    let compilation = compiler.compile(&source_file);
    assert!(
        compilation.is_err(),
        "expected compilation to fail due to private type leakage"
    );

    let diagnostics = compiler.diagnostics().entries();
    assert!(
        diagnostics.iter().any(|diagnostic| diagnostic
            .message
            .contains("cannot expose private type 'helper.Box'")),
        "expected private type leakage diagnostic, found {:?}",
        diagnostics
    );

    Ok(())
}

#[test]
fn public_functions_can_expose_imported_public_types() -> Result<()> {
    let dir = tempdir()?;
    let child_path = dir.path().join("child.tea");
    fs::write(
        &child_path,
        r#"
pub struct Box {
  value: Int
}

pub def wrap(value: Int) -> Box
  Box(value: value)
end
"#,
    )?;

    let helper_path = dir.path().join("helper.tea");
    fs::write(
        &helper_path,
        r#"
use child from "./child"

pub def wrap(value: Int) -> child.Box
  child.wrap(value)
end
"#,
    )?;

    let main_source = r#"
use helper from "./helper"
use child from "./child"

def take_box(box: child.Box) -> Int
  box.value
end

take_box(helper.wrap(5))
"#;

    let main_path = dir.path().join("main.tea");
    fs::write(&main_path, main_source)?;

    let source_file = SourceFile::new(SourceId(0), main_path.clone(), main_source.to_string());
    let mut compiler = Compiler::new(CompileOptions::default());
    let compilation = compiler.compile(&source_file)?;
    assert!(
        compiler.diagnostics().is_empty(),
        "expected no diagnostics, found {:?}",
        compiler.diagnostics()
    );

    drop(compilation);

    Ok(())
}

#[test]
fn modules_can_use_imported_typed_apis_privately_without_reexporting_them() -> Result<()> {
    let dir = tempdir()?;
    let child_path = dir.path().join("child.tea");
    fs::write(
        &child_path,
        r#"
pub struct Box {
  value: Int
}

pub def wrap(value: Int) -> Box
  Box(value: value)
end
"#,
    )?;

    let helper_path = dir.path().join("helper.tea");
    fs::write(
        &helper_path,
        r#"
use child from "./child"

pub def run(value: Int) -> Int
  var box = child.wrap(value)
  box.value
end
"#,
    )?;

    let main_source = r#"
use helper from "./helper"

helper.run(5)
"#;

    let main_path = dir.path().join("main.tea");
    fs::write(&main_path, main_source)?;

    let source_file = SourceFile::new(SourceId(0), main_path.clone(), main_source.to_string());
    let mut compiler = Compiler::new(CompileOptions::default());
    compiler.compile(&source_file)?;
    assert!(
        compiler.diagnostics().is_empty(),
        "expected no diagnostics, found {:?}",
        compiler.diagnostics()
    );

    Ok(())
}

#[cfg(feature = "llvm-backend")]
#[test]
fn relative_module_imports_compile_in_aot() -> Result<()> {
    let dir = tempdir()?;
    let helper_path = dir.path().join("helper.tea");
    fs::write(
        &helper_path,
        r#"
const SCALE: Int = 3

pub def wrap(value: Int) -> Int
  value * SCALE
end
"#,
    )?;

    let main_source = r#"
use helper from "./helper"

@println(helper.wrap(5))
"#;

    let main_path = dir.path().join("main.tea");
    fs::write(&main_path, main_source)?;

    let source_file = SourceFile::new(SourceId(0), main_path.clone(), main_source.to_string());
    let mut compiler = Compiler::new(CompileOptions::default());
    let compilation = compiler.compile(&source_file)?;

    let ir = aot::compile_compilation_to_llvm_ir(&compilation)?;
    assert!(
        ir.contains("define i32 @main"),
        "expected AOT IR to contain a main entry point:\n{ir}"
    );

    Ok(())
}
