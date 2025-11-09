use std::fs;

use anyhow::Result;
use tea_compiler::{CompileOptions, Compiler, SourceFile, SourceId};
use tempfile::tempdir;

#[test]
fn relative_module_exports_require_qualified_access() -> Result<()> {
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
use helper = "./helper"

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
use helper = "./helper"

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
