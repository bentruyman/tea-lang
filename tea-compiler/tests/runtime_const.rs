use std::path::PathBuf;

use tea_compiler::{CompileOptions, Compiler, SourceFile, SourceId};

// Full test execution support via AOT is planned for the future
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
  assert.eq(apply(3), 17)
end
"#;

    let source_file = SourceFile::new(
        SourceId(0),
        PathBuf::from("const_bindings.tea"),
        source.to_string(),
    );

    let mut compiler = Compiler::new(CompileOptions::default());
    compiler.compile(&source_file)?;
    assert!(
        compiler.diagnostics().is_empty(),
        "expected no diagnostics, found {:?}",
        compiler.diagnostics()
    );

    // TODO: Add AOT test execution when implemented
    // For now, we verify that the code compiles without errors

    Ok(())
}
