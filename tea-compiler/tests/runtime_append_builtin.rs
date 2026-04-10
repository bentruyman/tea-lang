use std::path::PathBuf;

mod support;

use tea_compiler::{aot, CompileOptions, Compiler, SourceFile, SourceId};

#[test]
fn append_builtin_executes_at_runtime() -> anyhow::Result<()> {
    let source = r#"
def append_numbers() -> List[Int]
  var numbers = [1, 2]
  @append(numbers, 3)
  return numbers
end

def append_words() -> List[String]
  var words: List[String] = []
  @append(words, "tea")
  return words
end

@println(append_numbers())
@println(append_words())
"#;

    let stdout = support::build_and_run(source, "append_builtin_runtime.tea", &[])?;
    assert_eq!(stdout, "[1, 2, 3]\n[tea]\n");

    Ok(())
}

#[test]
fn append_builtin_lowers_in_llvm_backend() -> anyhow::Result<()> {
    let source = r#"
def run() -> List[Int]
  var numbers = [1, 2]
  @append(numbers, 3)
  return numbers
end
"#;

    let mut compiler = Compiler::new(CompileOptions::default());
    let source_file = SourceFile::new(
        SourceId(0),
        PathBuf::from("append_builtin_aot.tea"),
        source.to_string(),
    );
    let compilation = compiler.compile(&source_file)?;
    assert!(
        compiler.diagnostics().is_empty(),
        "expected no diagnostics, found {:?}",
        compiler.diagnostics()
    );

    aot::compile_compilation_to_llvm_ir(&compilation)?;

    Ok(())
}
