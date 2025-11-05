use std::env;
use std::path::PathBuf;

use tea_compiler::{CompileOptions, Compiler, SourceFile, SourceId, Vm};

#[test]
fn env_helpers_operate_via_vm() -> anyhow::Result<()> {
    let source = r#"
use assert = "std.assert"
use env = "std.env"

assert.assert(env.has("TEA_LANG_TEST_VAR") == false)
assert.eq(env.get("TEA_LANG_TEST_VAR"), "")

env.set("TEA_LANG_TEST_VAR", "configured")
assert.assert(env.has("TEA_LANG_TEST_VAR"))
assert.eq(env.get("TEA_LANG_TEST_VAR"), "configured")

var vars = env.vars()
assert.assert(vars["TEA_LANG_TEST_VAR"] == "configured")

env.unset("TEA_LANG_TEST_VAR")
assert.assert(env.has("TEA_LANG_TEST_VAR") == false)

var cwd = env.cwd()
assert.assert(cwd != "")
"#;

    let mut compiler = Compiler::new(CompileOptions::default());
    let source_file = SourceFile::new(SourceId(0), PathBuf::from("env.tea"), source.to_string());
    let compilation = compiler.compile(&source_file)?;
    assert!(
        compiler.diagnostics().is_empty(),
        "unexpected diagnostics: {:?}",
        compiler.diagnostics()
    );

    let mut vm = Vm::new(&compilation.program);
    vm.run()?;

    env::remove_var("TEA_LANG_TEST_VAR");

    Ok(())
}
