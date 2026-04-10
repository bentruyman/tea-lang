use std::fs;
use std::path::PathBuf;
use std::process::Command;

use tea_compiler::{aot, CompileOptions, Compiler, SourceFile, SourceId};
use tempfile::tempdir;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn build_and_run(source: &str, file_name: &str) -> anyhow::Result<String> {
    let tmp = tempdir()?;
    let script_path = tmp.path().join(file_name);
    let binary_path = tmp.path().join(file_name.trim_end_matches(".tea"));
    fs::write(&script_path, source)?;

    let build_output = Command::new("cargo")
        .current_dir(workspace_root())
        .args(["run", "-p", "tea-cli", "--", "build"])
        .arg(&script_path)
        .arg("-o")
        .arg(&binary_path)
        .output()
        .expect("build tea script");

    assert!(
        build_output.status.success(),
        "build should succeed: {}",
        String::from_utf8_lossy(&build_output.stderr)
    );

    let output = Command::new(&binary_path)
        .output()
        .expect("run compiled tea binary");
    assert!(
        output.status.success(),
        "binary should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    Ok(String::from_utf8(output.stdout)?)
}

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

    let stdout = build_and_run(source, "append_builtin_runtime.tea")?;
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
