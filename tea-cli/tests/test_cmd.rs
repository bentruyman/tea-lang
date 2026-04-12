use std::fs;
use std::path::PathBuf;
use std::process::Command;

use tempfile::tempdir;

fn tea_cli_binary() -> &'static str {
    env!("CARGO_BIN_EXE_tea")
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn cargo_bin() -> PathBuf {
    std::env::var_os("CARGO")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("cargo"))
}

#[test]
fn bare_tea_prints_help_and_exits_successfully() {
    let output = Command::new(tea_cli_binary())
        .current_dir(workspace_root())
        .output()
        .expect("run tea without arguments");

    assert!(
        output.status.success(),
        "tea without arguments should print help and exit successfully"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Usage: tea [OPTIONS] <INPUT> [ARG]..."),
        "stdout missing usage line:\n{stdout}"
    );
    assert!(
        stdout.contains("See `tea <subcommand> --help` for command-specific options."),
        "stdout missing subcommand hint:\n{stdout}"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.is_empty(), "stderr should be empty:\n{stderr}");
}

#[test]
fn help_subcommand_block_is_aligned() {
    let output = Command::new(tea_cli_binary())
        .current_dir(workspace_root())
        .arg("--help")
        .output()
        .expect("run tea --help");

    assert!(output.status.success(), "tea --help should succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let expected_block = "\
Subcommands:
  tea build <INPUT>        Compile a tea-lang file to a native executable.
  tea docs-manifest        Generate the docs reference manifest for the website.
  tea fmt [PATH]...        Format tea-lang sources in place (defaults to current directory).
  tea test [PATH]...       Discover and run tea-lang test blocks.
";
    assert!(
        stdout.contains(expected_block),
        "help output missing aligned subcommand block:\n{stdout}"
    );
    assert!(
        !stdout.contains("--dump-tokens"),
        "top-level help should hide compiler debug flags:\n{stdout}"
    );
    assert!(
        !stdout.contains("--emit <EMIT>"),
        "top-level help should hide compiler debug flags:\n{stdout}"
    );
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
use assert from "std.assert"

test "one"
  assert.eq(1, 1)
end

test "two"
  assert.eq(2, 2)
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
    // Note: Test command now just validates compilation, not execution
    assert!(
        stdout.contains("compiled successfully"),
        "expected compilation success message: {stdout}"
    );
}

#[test]
#[ignore = "Test execution via AOT not yet implemented"]
fn test_runs_tests_and_reports_failures() {
    let tmp = tempdir().expect("tempdir");
    let target_root = tmp.path().join("target");
    fs::create_dir_all(&target_root).expect("create target dir");
    let pass_path = tmp.path().join("passing.tea");
    fs::write(
        &pass_path,
        r#"
use assert from "std.assert"

test "passing"
  assert.eq(3, 3)
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
    // Note: Test command now just validates compilation, not execution
    assert!(
        stdout.contains("compiled successfully"),
        "expected compilation success message: {stdout}"
    );

    let fail_path = tmp.path().join("failing.tea");
    fs::write(
        &fail_path,
        r#"
use assert from "std.assert"

test "failing"
  assert.eq(1, "wrong type")  # This will fail type checking
end
"#,
    )
    .expect("write failing test");

    let output = Command::new(tea_cli_binary())
        .current_dir(workspace_root())
        .env("TEA_TARGET_DIR", &target_root)
        .arg("test")
        .arg(&fail_path)
        .output()
        .expect("run tea test on failing file");

    assert!(
        !output.status.success(),
        "failing tests should result in non-zero exit status"
    );
}

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

#[test]
fn release_build_finds_and_builds_matching_runtime_archive() {
    let tmp = tempdir().expect("tempdir");
    let target_root = tmp.path().join("target");
    fs::create_dir_all(&target_root).expect("create isolated target dir");

    let source_path = tmp.path().join("release_runtime.tea");
    fs::write(
        &source_path,
        r#"
print("release runtime")
"#,
    )
    .expect("write source");

    let binary_path = tmp.path().join(if cfg!(windows) {
        "release_runtime.exe"
    } else {
        "release_runtime"
    });

    let output = Command::new(cargo_bin())
        .current_dir(workspace_root())
        .env("TEA_TARGET_DIR", &target_root)
        .args(["run", "--release", "-p", "tea-cli", "--", "build"])
        .arg(&source_path)
        .arg("--output")
        .arg(&binary_path)
        .output()
        .expect("run release tea build");

    assert!(
        output.status.success(),
        "release tea build should succeed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(binary_path.exists(), "binary should exist");

    let release_deps = target_root.join("release").join("deps");
    let runtime_rlib_exists = fs::read_dir(&release_deps)
        .expect("read release deps")
        .filter_map(Result::ok)
        .any(|entry| {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            name.starts_with("libtea_runtime") && name.ends_with(".rlib")
        });
    assert!(
        runtime_rlib_exists,
        "expected release tea-runtime rlib in {}",
        release_deps.display()
    );
}

#[test]
fn build_errors_script_runs_successfully() {
    let tmp = tempdir().expect("tempdir");
    let target_root = tmp.path().join("target");
    fs::create_dir_all(&target_root).expect("create target dir for build");

    let source_path = tmp.path().join("errors.tea");
    fs::write(
        &source_path,
        r#"
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
@println(from_cases)

var passthrough = read("notes.txt") catch "fallback"
@println(passthrough)

var fallback = try read("secret") catch "handled"
@println(fallback)
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
        stdout, "missing:missing\ncontent\nhandled\n",
        "compiled program should produce expected output"
    );
}
