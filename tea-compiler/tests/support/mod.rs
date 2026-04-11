use std::fs;
use std::path::PathBuf;
use std::process::Command;

use tempfile::tempdir;

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

#[allow(dead_code)]
pub fn build_and_run(source: &str, file_name: &str, args: &[&str]) -> anyhow::Result<String> {
    let tmp = tempdir()?;
    let script_path = tmp.path().join(file_name);
    let binary_path = tmp.path().join(file_name.trim_end_matches(".tea"));
    fs::write(&script_path, source)?;

    let build_output = Command::new(cargo_bin())
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
        .args(args)
        .output()
        .expect("run compiled tea binary");
    assert!(
        output.status.success(),
        "binary should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    Ok(String::from_utf8(output.stdout)?)
}

#[allow(dead_code)]
pub fn run_script(source: &str, file_name: &str, args: &[&str]) -> anyhow::Result<String> {
    let tmp = tempdir()?;
    let script_path = tmp.path().join(file_name);
    fs::write(&script_path, source)?;

    let mut command = Command::new(cargo_bin());
    command
        .current_dir(workspace_root())
        .args(["run", "-p", "tea-cli", "--"])
        .arg(&script_path);
    if !args.is_empty() {
        command.arg("--");
        command.args(args);
    }
    let output = command.output().expect("run tea script");
    assert!(
        output.status.success(),
        "script should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    Ok(String::from_utf8(output.stdout)?)
}
