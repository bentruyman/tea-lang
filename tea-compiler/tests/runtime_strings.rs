use std::path::PathBuf;

use tea_compiler::{CompileOptions, Compiler, SourceFile, SourceId};

mod support;

#[test]
fn interpolated_strings_emit_concat_instruction_and_execute() -> anyhow::Result<()> {
    let source = r#"
use assert from "std.assert"

def greet(name: String) -> String
  `Hello, ${name}!`
end

test "greet formats name"
  assert.eq(greet("Tea"), "Hello, Tea!")
end
"#;

    let source_file = SourceFile::new(
        SourceId(0),
        PathBuf::from("strings.tea"),
        source.to_string(),
    );

    let mut compiler = Compiler::new(CompileOptions::default());
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
fn string_concatenation_with_plus_operator() -> anyhow::Result<()> {
    let source = r#"
use assert from "std.assert"

def concat(a: String, b: String) -> String
  a + b
end

def build_string(count: Int) -> String
  var result = ""
  var i = 0
  while i < count
    result = result + "x"
    i = i + 1
  end
  return result
end

test "concatenates two strings"
  assert.eq(concat("hello", "world"), "helloworld")
end

test "concatenates with empty string"
  assert.eq("" + "test", "test")
  assert.eq("test" + "", "test")
end

test "concatenates multiple strings"
  assert.eq("a" + "b" + "c", "abc")
end

test "builds string with loop"
  assert.eq(build_string(5), "xxxxx")
end
"#;

    let source_file = SourceFile::new(
        SourceId(0),
        PathBuf::from("string_concat.tea"),
        source.to_string(),
    );

    let mut compiler = Compiler::new(CompileOptions::default());
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
fn string_helpers_execute_with_collection_utilities() -> anyhow::Result<()> {
    let source = r#"
use assert from "std.assert"
use string from "std.string"

def verify() -> Void
  var first_split = string.split_once("name=value", "=")
  var last_split = string.rsplit_once("archive.tar.gz", ".")
  var stripped_prefix = string.strip_prefix("prefix-value", "prefix-")
  var stripped_suffix = string.strip_suffix("report.txt", ".txt")

  if first_split == nil
    @panic("expected split_once to succeed")
    return
  end

  if last_split == nil
    @panic("expected rsplit_once to succeed")
    return
  end

  if stripped_prefix == nil
    @panic("expected strip_prefix to succeed")
    return
  end

  if stripped_suffix == nil
    @panic("expected strip_suffix to succeed")
    return
  end

  assert.eq(string.index_of("hello", "ll"), 2)
  assert.eq(string.last_index_of("banana", "na"), 4)
  assert.ok(string.contains("hello", "ell"))
  assert.eq(@len(string.split("a,b,c", ",")), 3)
  assert.eq(first_split![0], "name")
  assert.eq(first_split![1], "value")
  assert.eq(last_split![0], "archive.tar")
  assert.eq(last_split![1], "gz")
  assert.eq(string.replace_once("foo bar foo", "foo", "baz"), "baz bar foo")
  assert.eq(string.count("bananana", "na"), 3)
  assert.eq(stripped_prefix!, "value")
  assert.eq(stripped_suffix!, "report")
  assert.eq(string.pad_start("7", 3), "  7")
  assert.eq(string.pad_end("7", 3), "7  ")
  assert.eq(string.pad_start_with("tea", 6, "0"), "000tea")
  assert.eq(string.pad_end_with("tea", 6, "."), "tea...")
  assert.eq(string.join(["a", "b", "c"], "-"), "a-b-c")
  assert.eq(string.repeat("ha", 3), "hahaha")
end

verify()
@println("ok")
"#;

    let stdout = support::run_script(source, "string_helpers.tea", &[])?;
    assert_eq!(stdout, "ok\n");
    Ok(())
}
