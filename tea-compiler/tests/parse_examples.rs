use std::fs;
use std::path::PathBuf;

use tea_compiler::{CompileOptions, Compiler, SourceFile, SourceId, Vm};

fn compile_example(relative_path: &str) -> anyhow::Result<()> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir.parent().expect("workspace root");
    let example_path = workspace_root.join(relative_path);
    let contents = fs::read_to_string(&example_path)?;

    let mut compiler = Compiler::new(CompileOptions::default());
    let source = SourceFile::new(SourceId(0), example_path, contents);
    let compilation = compiler.compile(&source)?;
    assert!(
        compiler.diagnostics().is_empty(),
        "expected no diagnostics, found {:?}",
        compiler.diagnostics()
    );
    let mut vm = Vm::new(&compilation.program);
    vm.run()?;
    Ok(())
}

#[test]
fn parse_basics_example() -> anyhow::Result<()> {
    compile_example("examples/basics.tea")
}

#[test]
fn parse_fib_example() -> anyhow::Result<()> {
    compile_example("examples/fib.tea")
}

#[test]
fn parse_loops_example() -> anyhow::Result<()> {
    compile_example("examples/loops.tea")
}

#[test]
fn parse_lists_example() -> anyhow::Result<()> {
    compile_example("examples/lists.tea")
}

#[test]
fn parse_dicts_example() -> anyhow::Result<()> {
    compile_example("examples/dicts.tea")
}

#[test]
fn parse_structs_example() -> anyhow::Result<()> {
    compile_example("examples/structs.tea")
}

#[test]
fn parse_generics_example() -> anyhow::Result<()> {
    compile_example("examples/generics.tea")
}

#[test]
fn parse_lambdas_example() -> anyhow::Result<()> {
    compile_example("examples/lambdas.tea")
}

#[test]
fn parse_basic_test_example() -> anyhow::Result<()> {
    compile_example("examples/basic_test.tea")
}

#[test]
fn parse_cli_parse_example() -> anyhow::Result<()> {
    compile_example("examples/cli/parse.tea")
}

#[test]
fn parse_cli_fs_example() -> anyhow::Result<()> {
    compile_example("examples/cli/fs.tea")
}

#[test]
fn parse_cli_process_example() -> anyhow::Result<()> {
    compile_example("examples/cli/process.tea")
}

#[test]
fn parse_cli_env_example() -> anyhow::Result<()> {
    compile_example("examples/cli/env.tea")
}
