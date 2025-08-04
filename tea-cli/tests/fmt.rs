use std::fs;
use std::process::Command;

use tempfile::tempdir;

const ASSIGNMENT_SAMPLE: &str = r#"var data =
[
1,
2,
]

var total =
first
+ second
- third
"#;

const ASSIGNMENT_FORMATTED: &str = r#"var data =
  [
  1,
  2,
]

var total =
  first
  + second
  - third
"#;

fn tea_cli_binary() -> &'static str {
    env!("CARGO_BIN_EXE_tea-cli")
}

#[test]
fn fmt_check_flags_unformatted_input() {
    let dir = tempdir().expect("tempdir");
    let file_path = dir.path().join("sample.tea");
    fs::write(&file_path, ASSIGNMENT_SAMPLE).expect("write sample");

    let output = Command::new(tea_cli_binary())
        .arg("fmt")
        .arg("--check")
        .arg(&file_path)
        .output()
        .expect("run fmt --check");

    assert!(!output.status.success(), "expected non-zero exit status");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("needs formatting"),
        "stderr missing needs formatting message:\n{stderr}"
    );
}

#[test]
fn fmt_rewrites_and_is_idempotent() {
    let dir = tempdir().expect("tempdir");
    let file = dir.path().join("sample.tea");
    fs::write(&file, ASSIGNMENT_SAMPLE).expect("write sample");

    let status = Command::new(tea_cli_binary())
        .arg("fmt")
        .arg(&file)
        .status()
        .expect("run fmt");
    assert!(status.success(), "tea fmt should succeed");

    let formatted = fs::read_to_string(&file).expect("read formatted");
    assert_eq!(ASSIGNMENT_FORMATTED, formatted);

    let status = Command::new(tea_cli_binary())
        .arg("fmt")
        .arg("--check")
        .arg(&file)
        .status()
        .expect("run fmt --check");
    assert!(status.success(), "tea fmt --check should be idempotent");
}

#[test]
fn fmt_formats_directories_recursively() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    let nested = root.join("nested");
    fs::create_dir(&nested).expect("create nested");

    let root_file = root.join("root.tea");
    let nested_file = nested.join("inner.tea");

    fs::write(&root_file, ASSIGNMENT_SAMPLE).expect("write root");
    fs::write(&nested_file, "var values = [\n1\n3\n]\n").expect("write nested");

    let status = Command::new(tea_cli_binary())
        .arg("fmt")
        .arg(root)
        .status()
        .expect("run fmt on directory");
    assert!(status.success(), "fmt should succeed on directory");

    let formatted_root = fs::read_to_string(&root_file).expect("read formatted root");
    assert_eq!(ASSIGNMENT_FORMATTED, formatted_root);

    let formatted_nested = fs::read_to_string(&nested_file).expect("read formatted nested");
    assert_eq!("var values = [\n  1\n  3\n]\n", formatted_nested);
}

#[test]
fn fmt_check_supports_multiple_inputs() {
    let dir = tempdir().expect("tempdir");
    let file_one = dir.path().join("one.tea");
    let file_two = dir.path().join("two.tea");

    fs::write(&file_one, ASSIGNMENT_SAMPLE).expect("write one");
    fs::write(&file_two, ASSIGNMENT_SAMPLE).expect("write two");

    let output = Command::new(tea_cli_binary())
        .arg("fmt")
        .arg("--check")
        .arg(&file_one)
        .arg(&file_two)
        .output()
        .expect("run fmt --check on multiple files");

    assert!(
        !output.status.success(),
        "expected failure when files unformatted"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(file_one.file_name().unwrap().to_str().unwrap()),
        "stderr should list first file:\n{stderr}"
    );
    assert!(
        stderr.contains(file_two.file_name().unwrap().to_str().unwrap()),
        "stderr should list second file:\n{stderr}"
    );

    // The files should remain unmodified under --check.
    let check_one = fs::read_to_string(&file_one).expect("read one");
    assert_eq!(ASSIGNMENT_SAMPLE, check_one);
}
