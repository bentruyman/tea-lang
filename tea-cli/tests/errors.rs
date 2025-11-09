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
fn run_script_handles_errors_as_values() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let script_path = tmp.path().join("errors.tea");
    fs::write(
        &script_path,
        r#"error DataError {
  Missing(path: String)
  Permission
}

def read(path: String) -> String ! {
  DataError.Missing,
  DataError.Permission
}
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
print(from_cases)

var passthrough = read("notes.txt") catch "fallback"
print(passthrough)

var fallback = try read("secret") catch "handled"
print(fallback)
"#,
    )?;

    // Build the script using AOT compilation
    let binary_path = tmp.path().join("errors");
    let build_output = Command::new(tea_cli_binary())
        .current_dir(workspace_root())
        .arg("build")
        .arg(&script_path)
        .arg("-o")
        .arg(&binary_path)
        .output()
        .expect("build tea script with errors-as-values");

    assert!(build_output.status.success(), "build should succeed");

    // Run the compiled binary
    let output = Command::new(&binary_path)
        .output()
        .expect("run compiled binary");

    assert!(output.status.success(), "script should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(
        stdout, "missing:missing\ncontent\nhandled\n",
        "expected script output: {stdout}"
    );
    Ok(())
}
