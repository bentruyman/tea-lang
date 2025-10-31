use std::env;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use tea_compiler::{CompileOptions, Compiler, SourceFile, SourceId, Vm};

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
fn fs_roundtrip_through_vm() -> anyhow::Result<()> {
    let dir_path = unique_temp_dir();
    let file_path = dir_path.join("sample.txt");
    let backups_path = dir_path.join("backups");
    let copy_path = backups_path.join("copy.bin");

    let dir_str = dir_path.to_string_lossy();
    let file_str = file_path.to_string_lossy();
    let backups_str = backups_path.to_string_lossy();
    let copy_str = copy_path.to_string_lossy();

    let source = format!(
        r#"
use assert = "std.assert"
use fs = "std.fs"

fs.ensure_dir("{dir}")

var before = fs.list_dir("{dir}")
assert.assert_eq(length(before), 0)

fs.write_text_atomic("{file}", "hello fs")
assert.assert(fs.exists("{file}"))
assert.assert(fs.is_dir("{dir}"))
assert.assert(fs.is_symlink("{file}") == false)

var original = fs.read_text("{file}")
assert.assert_eq(original, "hello fs")

var info = fs.metadata("{file}")
assert.assert(info["is_file"])
assert.assert(info["is_dir"] == false)
assert.assert_eq(info["parent"], "{dir}")
assert.assert(info["size"] == fs.size("{file}"))
assert.assert(info["permissions"] > 0)
assert.assert(info["is_symlink"] == false)

var perms = fs.permissions("{file}")
assert.assert(perms > 0)

var after_write = fs.list_dir("{dir}")
assert.assert_eq(length(after_write), 1)
assert.assert_eq(after_write[0], "{file}")

var matches = fs.glob("{dir}/*.txt")
assert.assert_eq(length(matches), 1)
assert.assert_eq(matches[0], "{file}")

var visit_before = fs.walk("{dir}")
assert.assert_eq(length(visit_before), 1)
assert.assert_eq(visit_before[0], "{file}")

fs.ensure_parent("{copy}")
fs.write_text("{copy}", "hello fs")
assert.assert(fs.exists("{copy}"))
assert.assert_eq(fs.size("{copy}"), fs.size("{file}"))
assert.assert(fs.modified("{file}") > 0)

var after_copy = fs.list_dir("{dir}")
assert.assert_eq(length(after_copy), 2)
assert.assert_eq(after_copy[0], "{backups}")
assert.assert_eq(after_copy[1], "{file}")

var visit_after = fs.walk("{dir}")
assert.assert_eq(length(visit_after), 3)

var copy_info = fs.metadata("{copy}")
assert.assert(copy_info["is_file"])

var pattern_all = fs.glob("{dir}/**/*")
assert.assert(length(pattern_all) >= 3)

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
    let compilation = compiler.compile(&source_file)?;
    assert!(
        compiler.diagnostics().is_empty(),
        "expected no diagnostics, found {:?}",
        compiler.diagnostics()
    );

    let mut vm = Vm::new(&compilation.program);
    vm.run()?;

    if dir_path.exists() {
        // Clean up in case the script didn't remove everything.
        let _ = fs::remove_dir_all(&dir_path);
    }

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

fs.read_text("{path}")
"#,
        path = missing_str
    );

    let mut compiler = Compiler::new(CompileOptions::default());
    let source_file = SourceFile::new(SourceId(0), PathBuf::from("fs_missing.tea"), source);
    let compilation = compiler.compile(&source_file)?;
    assert!(compiler.diagnostics().is_empty(), "unexpected diagnostics");

    let mut vm = Vm::new(&compilation.program);
    let error = vm.run().expect_err("expected fs.read_text to fail");
    let message = error.to_string();
    let expected_prefix = format!("std.fs.read_text('{}') failed:", missing_str);
    assert!(
        message.starts_with(&expected_prefix),
        "error message did not start with expected prefix\n  expected: {expected_prefix}\n  actual:   {message}"
    );

    Ok(())
}
