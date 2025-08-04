use std::path::PathBuf;

use tea_compiler::{CompileOptions, Compiler, SourceFile, SourceId};

fn compile_program(source: &str) -> tea_compiler::Program {
    let mut compiler = Compiler::new(CompileOptions::default());
    let source_file = SourceFile::new(SourceId(0), PathBuf::from("stdlib.tea"), source.to_string());
    compiler
        .compile(&source_file)
        .expect("source should compile")
        .program
}

#[test]
fn vm_emits_builtin_call_for_assert() {
    let program = compile_program(
        r#"
use assert = "std.assert"

assert.assert(true)
"#,
    );

    let has_assert_call = program
        .chunk
        .instructions
        .iter()
        .any(|instruction| instruction.to_string() == "BUILTIN Assert 1");

    assert!(
        has_assert_call,
        "expected BUILTIN Assert 1 in instruction stream"
    );
}

#[test]
fn vm_emits_builtin_call_for_len() {
    let program = compile_program(
        r#"
use util = "std.util"

var count = util.len([1, 2, 3])
"#,
    );

    let has_len_call = program
        .chunk
        .instructions
        .iter()
        .any(|instruction| instruction.to_string() == "BUILTIN UtilLen 1");

    assert!(
        has_len_call,
        "expected BUILTIN UtilLen 1 in instruction stream"
    );
}

#[test]
fn vm_emits_builtin_call_for_fs_read_text() {
    let program = compile_program(
        r#"
use fs = "std.fs"

var contents = fs.read_text("example.txt")
"#,
    );

    let has_fs_call = program
        .chunk
        .instructions
        .iter()
        .any(|instruction| instruction.to_string() == "BUILTIN FsReadText 1");

    assert!(
        has_fs_call,
        "expected BUILTIN FsReadText 1 in instruction stream"
    );
}

#[test]
fn vm_emits_builtin_call_for_fs_glob() {
    let program = compile_program(
        r#"
use fs = "std.fs"

var matches = fs.glob("*.tea")
"#,
    );

    let has_fs_call = program
        .chunk
        .instructions
        .iter()
        .any(|instruction| instruction.to_string() == "BUILTIN FsGlob 1");

    assert!(
        has_fs_call,
        "expected BUILTIN FsGlob 1 in instruction stream"
    );
}

#[test]
fn vm_emits_builtin_call_for_io_write() {
    let program = compile_program(
        r#"
use io = "std.io"

io.write("hello")
"#,
    );

    let has_call = program
        .chunk
        .instructions
        .iter()
        .any(|instruction| instruction.to_string() == "BUILTIN IoWrite 1");

    assert!(has_call, "expected BUILTIN IoWrite 1 in instruction stream");
}

#[test]
fn vm_emits_builtin_call_for_json_encode() {
    let program = compile_program(
        r#"
use json = "std.json"

json.encode({ value: 1 })
"#,
    );

    let has_call = program
        .chunk
        .instructions
        .iter()
        .any(|instruction| instruction.to_string() == "BUILTIN JsonEncode 1");

    assert!(
        has_call,
        "expected BUILTIN JsonEncode 1 in instruction stream"
    );
}

#[test]
fn vm_emits_builtin_call_for_yaml_decode() {
    let program = compile_program(
        r#"
use yaml = "std.yaml"

yaml.decode("value: 42\n")
"#,
    );

    let has_call = program
        .chunk
        .instructions
        .iter()
        .any(|instruction| instruction.to_string() == "BUILTIN YamlDecode 1");

    assert!(
        has_call,
        "expected BUILTIN YamlDecode 1 in instruction stream"
    );
}
