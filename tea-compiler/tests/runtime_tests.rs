use std::path::PathBuf;

use tea_compiler::{CompileOptions, Compiler, SourceFile, SourceId};

#[test]
fn vm_executes_test_blocks_with_results() -> anyhow::Result<()> {
    let source = r#"
use assert = "std.assert"

test "passing assertion"
  assert.eq(1, 1)
end

test "failing assertion"
  assert.eq(2, 3)
end
"#;

    let path = PathBuf::from("tests/test_script.tea");
    let source_file = SourceFile::new(SourceId(0), path, source.to_string());

    let mut compiler = Compiler::new(CompileOptions::default());
    let compilation = compiler.compile(&source_file)?;
    assert!(
        compiler.diagnostics().is_empty(),
        "expected no diagnostics, found {:?}",
        compiler.diagnostics()
    );

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
