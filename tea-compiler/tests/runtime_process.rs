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

#[test]
fn process_spawn_waits_for_completion() -> Result<()> {
    let source = r#"
use assert = "std.assert"
use process = "std.process"

var handle = process.spawn("sh", ["-c", "printf world"])
var chunk = process.read_stdout(handle)
assert.assert(chunk == "world")
var result = process.wait(handle)
assert.assert(result.exit == 0)
assert.assert(result.stdout == "")
"#;

    let mut compiler = Compiler::new(CompileOptions::default());
    let source_file = SourceFile::new(
        SourceId(0),
        PathBuf::from("process_spawn.tea"),
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

#[test]
fn process_write_and_close_stdin() -> Result<()> {
    let source = r#"
use assert = "std.assert"
use process = "std.process"

var handle = process.spawn("sh", ["-c", "read line; printf \"got:%s\" \"$line\""])
process.write_stdin(handle, "tea\n")
process.close_stdin(handle)
var result = process.wait(handle)
assert.assert(result.stdout == "got:tea")
assert.assert(result.exit == 0)
"#;

    let mut compiler = Compiler::new(CompileOptions::default());
    let source_file = SourceFile::new(
        SourceId(0),
        PathBuf::from("process_stdin.tea"),
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
