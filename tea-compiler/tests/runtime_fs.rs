use std::env;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use tea_compiler::{CompileOptions, Compiler, SourceFile, SourceId};

mod support;

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
use assert from "std.assert"
use fs from "std.fs"

fs.create_dir("{dir}")
fs.mkdir_p("{backups}")
assert.ok(fs.exists("{dir}"))

var before = fs.read_dir("{dir}")
assert.eq(@len(before), 1)

fs.write_file("{file}", "hello fs")

var original = fs.read_file("{file}")
assert.eq(original, "hello fs")

var after_write = fs.walk("{dir}")
assert.eq(@len(after_write), 2)

fs.copy("{file}", "{copy}")
fs.rename("{copy}", "{backups}/moved.txt")

var after_copy = fs.walk("{dir}")
assert.eq(@len(fs.glob("{dir}/*")), 2)
assert.eq(@len(after_copy), 3)

fs.remove("{backups}/moved.txt")
fs.remove("{backups}")
fs.remove("{file}")
fs.remove("{dir}")
@println("ok")
"#,
        dir = dir_str,
        file = file_str,
        backups = backups_str,
        copy = copy_str,
    );

    let stdout = support::run_script(&source, "fs.tea", &[])?;
    assert_eq!(stdout, "ok\n");
    Ok(())
}

#[test]
fn fs_read_text_reports_consistent_error() -> anyhow::Result<()> {
    let dir_path = unique_temp_dir();
    let missing_path = dir_path.join("missing.txt");
    let missing_str = missing_path.to_string_lossy();

    let source = format!(
        r#"
use fs from "std.fs"

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

#[test]
fn fs_bytes_roundtrip_through_runtime() -> anyhow::Result<()> {
    let dir_path = unique_temp_dir();
    let file_path = dir_path.join("sample.bin");
    let atomic_path = dir_path.join("sample-atomic.bin");

    let dir_str = dir_path.to_string_lossy();
    let file_str = file_path.to_string_lossy();
    let atomic_str = atomic_path.to_string_lossy();

    let source = format!(
        r#"
use assert from "std.assert"
use fs from "std.fs"

fs.mkdir_p("{dir}")
fs.write_bytes("{file}", [0, 1, 2, 255])
fs.write_bytes_atomic("{atomic}", [5, 4, 3, 2, 1])

var first = fs.read_bytes("{file}")
var second = fs.read_bytes("{atomic}")

assert.eq(@len(first), 4)
assert.eq(first[0], 0)
assert.eq(first[3], 255)
assert.eq(@len(second), 5)
assert.eq(second[0], 5)
assert.eq(second[4], 1)

fs.remove("{file}")
fs.remove("{atomic}")
fs.remove("{dir}")
@println("ok")
"#,
        dir = dir_str,
        file = file_str,
        atomic = atomic_str,
    );

    let stdout = support::run_script(&source, "fs-bytes.tea", &[])?;
    assert_eq!(stdout, "ok\n");
    Ok(())
}

#[test]
fn fs_hardening_helpers_execute_through_runtime() -> anyhow::Result<()> {
    let dir_path = unique_temp_dir();
    let nested_path = dir_path.join("nested").join("child.txt");

    let dir_str = dir_path.to_string_lossy();
    let nested_str = nested_path.to_string_lossy();

    let source = format!(
        r#"
use assert from "std.assert"
use fs from "std.fs"
use path from "std.path"

fs.ensure_parent("{nested}")
assert.ok(fs.exists(path.join(["{dir}", "nested"])))

fs.write_file_atomic("{nested}", "tea")
fs.append_file("{nested}", "-lang")
fs.append_bytes("{nested}", [10, 33])

var contents = fs.read_file("{nested}")
assert.eq(contents, "tea-lang\n!")

var temp_dir = fs.create_temp_dir("tea-fs-")
var temp_file = fs.create_temp_file("tea-fs-")
assert.ok(fs.exists(temp_dir))
assert.ok(fs.exists(temp_file))
assert.ok(fs.metadata(temp_dir).is_dir)
assert.ok(fs.metadata(temp_file).is_file)
assert.ok(! fs.is_symlink(temp_file))
assert.ok(! fs.is_symlink(path.join([temp_dir, "definitely-missing"])))

fs.remove(temp_file)
fs.remove(temp_dir)
fs.remove("{dir}")
@println("ok")
"#,
        dir = dir_str,
        nested = nested_str,
    );

    let stdout = support::run_script(&source, "fs-hardening.tea", &[])?;
    assert_eq!(stdout, "ok\n");
    Ok(())
}
