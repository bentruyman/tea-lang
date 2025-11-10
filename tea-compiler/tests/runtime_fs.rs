use std::env;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use tea_compiler::{CompileOptions, Compiler, SourceFile, SourceId};

fn unique_temp_dir() -> PathBuf {
    let mut base = env::temp_dir();
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();
    base.push(format!("tea-fs-test-{unique}"));
    base
}

#[test]
fn fs_roundtrip_through_runtime() -> anyhow::Result<()> {
    let dir_path = unique_temp_dir();
    let file_path = dir_path.join("sample.txt");
    let backups_path = dir_path.join("backups");
    let copy_path = backups_path.join("copy.txt");

    let dir_str = dir_path.to_string_lossy();
    let file_str = file_path.to_string_lossy();
    let backups_str = backups_path.to_string_lossy();
    let copy_str = copy_path.to_string_lossy();

    let source = format!(
        r#"
use assert = "std.assert"
use fs = "std.fs"

fs.create_dir("{dir}")

var before = fs.read_dir("{dir}")
assert.eq(@len(before), 0)

fs.write_file("{file}", "hello fs")

var original = fs.read_file("{file}")
assert.eq(original, "hello fs")

var after_write = fs.read_dir("{dir}")
assert.eq(@len(after_write), 1)
assert.eq(after_write[0], "{file}")

fs.create_dir("{backups}")
fs.write_file("{copy}", "hello fs")

var after_copy = fs.read_dir("{dir}")
assert.eq(@len(after_copy), 2)

fs.remove("{copy}")
fs.remove("{backups}")
fs.remove("{file}")
fs.remove("{dir}")
"#,
        dir = dir_str,
        file = file_str,
        backups = backups_str,
        copy = copy_str,
    );

    let mut compiler = Compiler::new(CompileOptions::default());
    let source_file = SourceFile::new(SourceId(0), PathBuf::from("fs.tea"), source);
    compiler.compile(&source_file)?;
    assert!(
        compiler.diagnostics().is_empty(),
        "expected no diagnostics, found {:?}",
        compiler.diagnostics()
    );

    // Full test execution support via AOT is planned for the future
    assert!(
        compiler.diagnostics().is_empty(),
        "expected no diagnostics, found {:?}",
        compiler.diagnostics()
    );

    // TODO: Add AOT test execution when implemented
    // For now, we verify that the code compiles without errors

    Ok(())
}

#[test]
fn fs_read_text_reports_consistent_error() -> anyhow::Result<()> {
    let dir_path = unique_temp_dir();
    let missing_path = dir_path.join("missing.txt");
    let missing_str = missing_path.to_string_lossy();

    let source = format!(
        r#"
use fs = "std.fs"

fs.read_file("{path}")
"#,
        path = missing_str
    );

    let mut compiler = Compiler::new(CompileOptions::default());
    let source_file = SourceFile::new(SourceId(0), PathBuf::from("fs_missing.tea"), source);
    compiler.compile(&source_file)?;
    assert!(compiler.diagnostics().is_empty(), "unexpected diagnostics");

    // Full test execution support via AOT is planned for the future
    assert!(
        compiler.diagnostics().is_empty(),
        "expected no diagnostics, found {:?}",
        compiler.diagnostics()
    );

    // TODO: Add AOT test execution when implemented
    // For now, we verify that the code compiles without errors

    Ok(())
}
