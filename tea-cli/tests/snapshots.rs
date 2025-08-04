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

fn escape_for_tea(path: &PathBuf) -> String {
    path.to_string_lossy()
        .replace('\\', "/")
        .replace('"', "\\\"")
}

#[test]
fn cli_capture_runs_command() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let emit_path = tmp.path().join("emit.tea");
    fs::write(
        &emit_path,
        r#"use print = "std.print"
print.print("cli capture output")
"#,
    )?;

    let capture_path = tmp.path().join("capture.tea");
    let script = format!(
        r#"use assert = "std.assert"
use cli = "support.cli"

test "capture tea"
  var result = cli.capture(["{}", "{}"])
  assert.assert_eq(result.exit, 0)
  assert.assert_eq(result.stdout, "cli capture output\n")
  assert.assert_empty(result.stderr)
end
"#,
        escape_for_tea(&PathBuf::from(tea_cli_binary())),
        escape_for_tea(&emit_path),
    );
    fs::write(&capture_path, script)?;

    let output = Command::new(tea_cli_binary())
        .current_dir(workspace_root())
        .arg("test")
        .arg(&capture_path)
        .output()
        .expect("run tea test with capture script");

    assert!(
        output.status.success(),
        "tea test should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    Ok(())
}
