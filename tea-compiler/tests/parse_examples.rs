use std::fs;
use std::path::PathBuf;

use tea_compiler::{CompileOptions, Compiler, SourceFile, SourceId};

fn compile_example(relative_path: &str) -> anyhow::Result<()> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir.parent().expect("workspace root");
    let example_path = workspace_root.join(relative_path);
    let contents = fs::read_to_string(&example_path)?;

    let mut compiler = Compiler::new(CompileOptions::default());
    let source = SourceFile::new(SourceId(0), example_path, contents);
    compiler.compile(&source)?;
    assert!(
        compiler.diagnostics().is_empty(),
        "expected no diagnostics, found {:?}",
        compiler.diagnostics()
    );

    // TODO: Execute examples via AOT when implemented
    Ok(())
}

#[test]
fn parse_basics_example() -> anyhow::Result<()> {
    compile_example("examples/language/basics/basics.tea")
}

#[test]
fn parse_const_example() -> anyhow::Result<()> {
    compile_example("examples/language/basics/const.tea")
}

#[test]
fn parse_fib_example() -> anyhow::Result<()> {
    compile_example("examples/language/basics/fib.tea")
}

#[test]
fn parse_loops_example() -> anyhow::Result<()> {
    compile_example("examples/language/control_flow/loops.tea")
}

#[test]
fn parse_lists_example() -> anyhow::Result<()> {
    compile_example("examples/language/collections/lists.tea")
}

#[test]
fn parse_dicts_example() -> anyhow::Result<()> {
    compile_example("examples/language/collections/dicts.tea")
}

#[test]
fn parse_string_interpolation_example() -> anyhow::Result<()> {
    compile_example("examples/language/strings/interpolation.tea")
}

#[test]
fn parse_structs_example() -> anyhow::Result<()> {
    compile_example("examples/language/types/structs.tea")
}

#[test]
fn parse_generics_example() -> anyhow::Result<()> {
    compile_example("examples/language/types/generics.tea")
}

#[test]
fn parse_lambdas_example() -> anyhow::Result<()> {
    compile_example("examples/language/functions/lambdas.tea")
}

#[test]
fn parse_basic_test_example() -> anyhow::Result<()> {
    compile_example("examples/language/basics/basic_test.tea")
}
