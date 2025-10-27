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
        r#"use io = "std.io"

error DataError {
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
io.write(from_cases)
io.write("\n")

var passthrough = read("notes.txt") catch "fallback"
io.write(passthrough)
io.write("\n")

var fallback = try read("secret") catch "handled"
io.write(fallback)
io.flush()
"#,
    )?;

    let output = Command::new(tea_cli_binary())
        .current_dir(workspace_root())
        .arg(&script_path)
        .output()
        .expect("run tea script with errors-as-values");

    assert!(output.status.success(), "script should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(
        stdout, "missing:missing\ncontent\nhandled",
        "expected script output: {stdout}"
    );
    Ok(())
}
