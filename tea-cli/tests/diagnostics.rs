use std::fs;
use std::process::Command;

use tempfile::tempdir;

fn tea_cli_binary() -> &'static str {
    env!("CARGO_BIN_EXE_tea")
}

fn workspace_root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

#[test]
fn reports_missing_module_with_span() {
    let tmp = tempdir().expect("tempdir");
    let script_path = tmp.path().join("missing_module.tea");
    fs::write(&script_path, "use missing from \"./not_there.tea\"\n").expect("write script");

    let output = Command::new(tea_cli_binary())
        .current_dir(workspace_root())
        .arg(&script_path)
        .output()
        .expect("run tea on missing module script");

    assert!(
        !output.status.success(),
        "expected non-zero exit when module is missing"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("failed to resolve module './not_there.tea':"),
        "expected missing module message, got: {stderr}"
    );
    assert!(
        stderr.contains("-->"),
        "expected span reference in diagnostics: {stderr}"
    );
    assert!(
        stderr.contains("use missing from \"./not_there.tea\""),
        "expected source line in diagnostics: {stderr}"
    );
}

#[test]
fn highlights_argument_type_mismatch() {
    let tmp = tempdir().expect("tempdir");
    let script_path = tmp.path().join("type_error.tea");
    fs::write(
        &script_path,
        r#"
use assert from "std.assert"

def double(x: Int) -> Int
  x + x
end

double("hi")
"#,
    )
    .expect("write script");

    let output = Command::new(tea_cli_binary())
        .current_dir(workspace_root())
        .arg(&script_path)
        .output()
        .expect("run tea on type error script");

    assert!(
        !output.status.success(),
        "expected non-zero exit for type mismatch"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("argument 1 to 'double': expected Int, found String"),
        "expected type mismatch message, got: {stderr}"
    );
    assert!(
        stderr.contains("double(\"hi\")"),
        "expected source line in diagnostics: {stderr}"
    );
    assert!(
        stderr
            .lines()
            .any(|line| line.trim_start().starts_with('^')),
        "expected caret underline pointing at the argument: {stderr}"
    );
}

#[test]
fn runs_script_with_relative_module_import_from_external_directory() {
    let tmp = tempdir().expect("tempdir");
    let helper_path = tmp.path().join("hello.tea");
    fs::write(
        &helper_path,
        r#"
pub def greet() -> String
  "hi"
end
"#,
    )
    .expect("write helper module");

    let script_path = tmp.path().join("todo.tea");
    fs::write(
        &script_path,
        r#"
use hello from "./hello.tea"

@println(hello.greet())
"#,
    )
    .expect("write main script");

    let output = Command::new(tea_cli_binary())
        .current_dir(tmp.path())
        .arg("todo.tea")
        .output()
        .expect("run tea on script with relative module import");

    assert!(
        output.status.success(),
        "expected script to succeed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout, "hi\n", "expected script output: {stdout}");
}
