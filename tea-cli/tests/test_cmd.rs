use std::fs;
use std::path::PathBuf;
use std::process::Command;

use tempfile::tempdir;

fn tea_cli_binary() -> &'static str {
    env!("CARGO_BIN_EXE_tea-cli")
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

#[test]
fn test_lists_discovered_tests() {
    let tmp = tempdir().expect("tempdir");
    let target_root = tmp.path().join("target");
    fs::create_dir_all(&target_root).expect("create target dir");
    let test_path = tmp.path().join("sample.tea");
    fs::write(
        &test_path,
        r#"
use assert = "std.assert"

test "one"
  assert.assert_eq(1, 1)
end

test "two"
  assert.assert_eq(2, 2)
end
"#,
    )
    .expect("write test file");

    let output = Command::new(tea_cli_binary())
        .current_dir(workspace_root())
        .env("TEA_TARGET_DIR", &target_root)
        .arg("test")
        .arg("--list")
        .arg(&test_path)
        .output()
        .expect("run tea test --list");

    assert!(output.status.success(), "tea test --list should succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("one"),
        "expected to list first test: {stdout}"
    );
    assert!(
        stdout.contains("two"),
        "expected to list second test: {stdout}"
    );
}

#[test]
fn test_runs_tests_and_reports_failures() {
    let tmp = tempdir().expect("tempdir");
    let target_root = tmp.path().join("target");
    fs::create_dir_all(&target_root).expect("create target dir");
    let pass_path = tmp.path().join("passing.tea");
    fs::write(
        &pass_path,
        r#"
use assert = "std.assert"

test "passing"
  assert.assert_eq(3, 3)
end
"#,
    )
    .expect("write passing test");

    let output = Command::new(tea_cli_binary())
        .current_dir(workspace_root())
        .env("TEA_TARGET_DIR", &target_root)
        .arg("test")
        .arg(&pass_path)
        .output()
        .expect("run tea test on passing file");

    assert!(output.status.success(), "passing tests should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("PASS passing"),
        "expected PASS line: {stdout}"
    );
    assert!(
        stdout.contains("Summary: 1 passed"),
        "expected summary line: {stdout}"
    );

    let fail_path = tmp.path().join("failing.tea");
    fs::write(
        &fail_path,
        r#"
use assert = "std.assert"

test "failing"
  assert.assert_eq(1, 2)
end
"#,
    )
    .expect("write failing test");

    let status = Command::new(tea_cli_binary())
        .current_dir(workspace_root())
        .env("TEA_TARGET_DIR", &target_root)
        .arg("test")
        .arg(&fail_path)
        .status()
        .expect("run tea test on failing file");

    assert!(
        !status.success(),
        "failing tests should result in non-zero exit status"
    );
}

#[cfg(feature = "llvm-aot")]
#[test]
fn build_creates_bundle_and_checksum() {
    let tmp = tempdir().expect("tempdir");
    let target_root = tmp.path().join("target");
    fs::create_dir_all(&target_root).expect("create target dir for build");
    let source_path = tmp.path().join("app.tea");
    fs::write(
        &source_path,
        r#"


def main() -> Int
  print("hello from bundle")
  0
end

main()
"#,
    )
    .expect("write source");

    let binary_path = tmp.path().join("app");
    let bundle_path = tmp.path().join("app.tar.gz");
    let checksum_path = tmp.path().join("app.sha256");

    let status = Command::new(tea_cli_binary())
        .current_dir(workspace_root())
        .env("TEA_TARGET_DIR", &target_root)
        .arg("build")
        .arg(&source_path)
        .arg("--backend")
        .arg("llvm")
        .arg("--output")
        .arg(&binary_path)
        .arg("--bundle")
        .arg("--bundle-output")
        .arg(&bundle_path)
        .arg("--checksum")
        .arg("--checksum-output")
        .arg(&checksum_path)
        .status()
        .expect("run tea build");

    assert!(status.success(), "tea build should succeed");
    assert!(binary_path.exists(), "binary should exist");
    assert!(bundle_path.exists(), "bundle should exist");
    assert!(checksum_path.exists(), "checksum should exist");
}

#[cfg(feature = "llvm-aot")]
#[test]
fn build_errors_script_runs_successfully() {
    let tmp = tempdir().expect("tempdir");
    let target_root = tmp.path().join("target");
    fs::create_dir_all(&target_root).expect("create target dir for build");

    let source_path = tmp.path().join("errors.tea");
    fs::write(
        &source_path,
        r#"
use io = "std.io"

error DataError {
  Missing(path: String)
  Permission
}

def read(path: String) -> String ! { DataError.Missing, DataError.Permission }
  if path == "missing"
    throw DataError.Missing(path)
  end
  if path == "secret"
    throw DataError.Permission()
  end
  return "content"
end

def describe(path: String) -> String
  try read(path) catch err
    case is DataError.Missing => `missing:${err.path}`
    case is DataError.Permission => "denied"
    case _ => "unexpected"
  end
end

var from_cases = describe("missing")
io.write(from_cases)
io.write("\n")

var passthrough = read("notes.txt") catch "fallback"
io.write(passthrough)
io.write("\n")

var fallback = try read("secret") catch "handled"
io.write(fallback)
io.flush()
"#,
    )
    .expect("write errors script");

    let binary_name = if cfg!(windows) {
        "errors.exe"
    } else {
        "errors"
    };
    let binary_path = tmp.path().join(binary_name);

    let status = Command::new(tea_cli_binary())
        .current_dir(workspace_root())
        .env("TEA_TARGET_DIR", &target_root)
        .arg("build")
        .arg(&source_path)
        .arg("--backend")
        .arg("llvm")
        .arg("--output")
        .arg(&binary_path)
        .status()
        .expect("run tea build");

    assert!(status.success(), "tea build should succeed");
    assert!(binary_path.exists(), "binary should exist");

    let output = Command::new(&binary_path)
        .current_dir(tmp.path())
        .output()
        .expect("execute compiled binary");

    assert!(
        output.status.success(),
        "compiled program should exit successfully"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(
        stdout, "missing:missing\ncontent\nhandled",
        "compiled program should produce expected output"
    );
}
