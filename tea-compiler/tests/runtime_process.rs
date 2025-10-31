use anyhow::{ensure, Result};
use std::path::PathBuf;

use tea_compiler::{CompileOptions, Compiler, SourceFile, SourceId, Vm};

#[test]
fn process_run_captures_output() -> Result<()> {
    let source = r#"
use assert = "std.assert"
use process = "std.process"

var result = process.run("sh", ["-c", "printf hello"])
assert.assert(result.success)
assert.assert(result.exit == 0)
assert.assert(result.stdout == "hello")
assert.assert(result.stderr == "")
"#;

    let mut compiler = Compiler::new(CompileOptions::default());
    let source_file = SourceFile::new(
        SourceId(0),
        PathBuf::from("process_run.tea"),
        source.to_string(),
    );
    let compilation = compiler.compile(&source_file)?;
    ensure!(
        compiler.diagnostics().is_empty(),
        "unexpected diagnostics: {:?}",
        compiler.diagnostics()
    );

    let mut vm = Vm::new(&compilation.program);
    vm.run()?;
    Ok(())
}
