use std::path::PathBuf;

use tea_compiler::{CompileOptions, Compiler, SourceFile, SourceId, TestStatus, Vm};

#[test]
fn vm_executes_test_blocks_with_results() -> anyhow::Result<()> {
    let source = r#"
use assert = "std.assert"

test "passing assertion"
  assert.assert_eq(1, 1)
end

test "failing assertion"
  assert.assert_eq(2, 3)
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

    let mut vm = Vm::new(&compilation.program);
    let outcomes = vm.run_tests(None, None)?;
    assert_eq!(outcomes.len(), 2, "expected two test outcomes");

    assert!(
        matches!(outcomes[0].status, TestStatus::Passed),
        "first test should pass: {:?}",
        outcomes[0]
    );

    match &outcomes[1].status {
        TestStatus::Failed { message } => {
            assert!(
                message.contains("assert_eq failed"),
                "expected failure message to mention assert_eq, got {message}"
            );
        }
        other => panic!("second test should fail, found {:?}", other),
    }

    Ok(())
}
