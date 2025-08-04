use anyhow::{ensure, Result};
use std::path::PathBuf;

use tea_compiler::{CompileOptions, Compiler, SourceFile, SourceId, Vm};

#[test]
fn json_roundtrip_through_vm() -> Result<()> {
    let source = r#"
use assert = "std.assert"
use json = "std.json"
var payload = { name: "tea", version: "1.0" }
var encoded = json.encode(payload)
assert.assert(encoded != "")

var decoded = json.decode(encoded)
assert.assert(decoded["name"] == "tea")
assert.assert(decoded["version"] == "1.0")
"#;

    let mut compiler = Compiler::new(CompileOptions::default());
    let source_file = SourceFile::new(
        SourceId(0),
        PathBuf::from("json_roundtrip.tea"),
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
fn yaml_decode_and_encode() -> Result<()> {
    let source = r#"
use assert = "std.assert"
use yaml = "std.yaml"

var doc = "title: Tea Guide\nitems:\n  - leaf\n  - pot\n"
var decoded = yaml.decode(doc)
assert.assert(decoded["title"] == "Tea Guide")
assert.assert(decoded["items"][1] == "pot")

var reencoded = yaml.encode(decoded)
assert.assert(reencoded != "")
"#;

    let mut compiler = Compiler::new(CompileOptions::default());
    let source_file = SourceFile::new(
        SourceId(0),
        PathBuf::from("yaml_roundtrip.tea"),
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
