use std::fs;
use std::process::Command;

use tempfile::tempdir;

fn tea_cli_binary() -> &'static str {
    env!("CARGO_BIN_EXE_tea-cli")
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
    fs::write(&script_path, "use missing = \"./not_there.tea\"\n").expect("write script");

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
        stderr.contains("use missing = \"./not_there.tea\""),
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
use assert = "std.assert"

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
