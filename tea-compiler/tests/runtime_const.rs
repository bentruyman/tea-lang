use std::path::PathBuf;

use tea_compiler::{CompileOptions, Compiler, SourceFile, SourceId, TestStatus, Vm};

#[test]
fn const_bindings_are_immutable_and_accessible() -> anyhow::Result<()> {
    let source = r#"
use assert = "std.assert"

const SCALE = 5

def apply(value: Int) -> Int
  const offset = 2
  value * SCALE + offset
end

test "const bindings"
  assert.assert_eq(apply(3), 17)
end
"#;

    let source_file = SourceFile::new(
        SourceId(0),
        PathBuf::from("const_bindings.tea"),
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
    assert_eq!(outcomes.len(), 1, "expected a single test outcome");
    assert!(
        matches!(outcomes[0].status, TestStatus::Passed),
        "const test should pass: {:?}",
        outcomes[0]
    );

    Ok(())
}
