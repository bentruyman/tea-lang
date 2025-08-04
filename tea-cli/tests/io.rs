use std::fs;
use std::io::Write as _;
use std::path::PathBuf;
use std::process::{Command, Stdio};

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
fn io_write_emits_without_newline() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let script_path = tmp.path().join("write.tea");
    fs::write(
        &script_path,
        r#"use io = "std.io"

io.write("hello")
io.flush()
"#,
    )?;

    let output = Command::new(tea_cli_binary())
        .current_dir(workspace_root())
        .arg(&script_path)
        .output()
        .expect("run tea-cli write script");

    assert!(output.status.success(), "script should succeed");
    assert_eq!(String::from_utf8_lossy(&output.stdout), "hello");
    Ok(())
}

#[test]
fn io_read_line_round_trip() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let script_path = tmp.path().join("read.tea");
    fs::write(
        &script_path,
        r#"use io = "std.io"

var line = io.read_line()
if line != nil
  io.write(line)
  io.flush()
end
"#,
    )?;

    let mut child = Command::new(tea_cli_binary())
        .current_dir(workspace_root())
        .arg(&script_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn tea-cli read script");

    {
        let stdin = child.stdin.as_mut().expect("stdin available");
        stdin.write_all(b"hello\n")?;
    }

    let output = child.wait_with_output().expect("collect output");
    assert!(output.status.success(), "script should succeed");
    assert_eq!(String::from_utf8_lossy(&output.stdout), "hello");
    Ok(())
}
