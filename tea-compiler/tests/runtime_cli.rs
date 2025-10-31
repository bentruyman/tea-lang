use anyhow::{ensure, Result};
use std::path::PathBuf;

use tea_compiler::{CompileOptions, Compiler, SourceFile, SourceId, Vm};

#[test]
fn cli_parse_handles_flags_options_and_positionals() -> Result<()> {
    let source = r#"
use assert = "std.assert"
use util = "std.util"
use cli = "support.cli"

var spec = {
  name: "demo",
  description: "Demo program",
  options: [
    { name: "verbose", aliases: ["-v", "--verbose"], kind: "flag" },
    { name: "count", aliases: ["-c", "--count"], kind: "option", type: "Int", default: 1 }
  ],
  positionals: [
    { name: "input", type: "String", required: true }
  ],
  subcommands: []
}

var result = cli.parse(spec, ["--count", "3", "--verbose", "input.txt"])
assert.assert(result.ok == true)
assert.assert(result.exit == 0)
assert.assert(result.command == "demo")
assert.assert(result.options["count"] == 3)
assert.assert(result.options["verbose"] == true)
assert.assert(result.positionals["input"] == "input.txt")
assert.assert(length(result.rest) == 0)
assert.assert(result.scopes[0]["name"] == "demo")
assert.assert(result.scopes[0]["options"]["count"] == 3)
assert.assert(result.help != "")
"#;

    let mut compiler = Compiler::new(CompileOptions::default());
    let source_file = SourceFile::new(
        SourceId(0),
        PathBuf::from("cli_parse.tea"),
        source.to_string(),
    );
    let compilation = match compiler.compile(&source_file) {
        Ok(comp) => comp,
        Err(err) => {
            if !compiler.diagnostics().is_empty() {
                for diagnostic in compiler.diagnostics().entries() {
                    eprintln!("diagnostic: {:?}", diagnostic);
                }
            }
            return Err(err);
        }
    };
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
fn cli_args_reflect_vm_context() -> Result<()> {
    let source = r#"
use assert = "std.assert"
use cli = "support.cli"

var argv = cli.args()
assert.assert(length(argv) == 2)
assert.assert(argv[0] == "one")
assert.assert(argv[1] == "two")
"#;

    let mut compiler = Compiler::new(CompileOptions::default());
    let source_file = SourceFile::new(
        SourceId(0),
        PathBuf::from("cli_args.tea"),
        source.to_string(),
    );
    let compilation = match compiler.compile(&source_file) {
        Ok(comp) => comp,
        Err(err) => {
            if !compiler.diagnostics().is_empty() {
                for diagnostic in compiler.diagnostics().entries() {
                    eprintln!("diagnostic: {:?}", diagnostic);
                }
            }
            return Err(err);
        }
    };
    ensure!(
        compiler.diagnostics().is_empty(),
        "unexpected diagnostics: {:?}",
        compiler.diagnostics()
    );

    let mut vm = Vm::new(&compilation.program);
    vm.set_cli_args(vec!["one".to_string(), "two".to_string()]);
    vm.run()?;
    Ok(())
}

#[test]
fn cli_capture_reports_process_error() -> Result<()> {
    let source = r#"
use cli = "support.cli"

cli.capture(["this-command-should-not-exist"]) 
"#;

    let mut compiler = Compiler::new(CompileOptions::default());
    let source_file = SourceFile::new(
        SourceId(0),
        PathBuf::from("cli_capture_error.tea"),
        source.to_string(),
    );
    let compilation = compiler.compile(&source_file)?;
    ensure!(
        compiler.diagnostics().is_empty(),
        "unexpected diagnostics: {:?}",
        compiler.diagnostics()
    );

    let mut vm = Vm::new(&compilation.program);
    let error = vm.run().expect_err("expected cli.capture to fail");
    let message = error.to_string();
    let expected_prefix = "support.cli.capture('this-command-should-not-exist') failed:";
    assert!(
        message.starts_with(expected_prefix),
        "error message did not start with expected prefix\n  expected: {expected_prefix}\n  actual:   {message}"
    );

    Ok(())
}
