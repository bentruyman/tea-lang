use std::path::PathBuf;

use tea_compiler::{CompileOptions, Compiler, SourceFile, SourceId};

#[test]
fn for_loop_dict_key_value() -> anyhow::Result<()> {
    let source = r#"
use assert = "std.assert"

test "for loop iterates over dict with key and value"
  var point = { x: 3, y: 4 }
  var sum = 0
  
  for key, value of point
    sum = sum + value
  end
  
  assert.eq(sum, 7)
end
"#;

    let source_file = SourceFile::new(
        SourceId(0),
        PathBuf::from("dict_iteration.tea"),
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
fn for_loop_dict_empty() -> anyhow::Result<()> {
    let source = r#"
use assert = "std.assert"

test "for loop with empty dict"
  var empty = {}
  var count = 0
  
  for key, value of empty
    count = count + 1
  end
  
  assert.eq(count, 0)
end
"#;

    let source_file = SourceFile::new(
        SourceId(0),
        PathBuf::from("dict_empty.tea"),
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
fn for_loop_dict_keys_accessible() -> anyhow::Result<()> {
    let source = r#"
use assert = "std.assert"

test "for loop can access dict keys"
  var scores = { "alice": 10, "bob": 8 }
  var key_count = 0
  
  for key, value of scores
    key_count = key_count + @len(key)
  end
  
  # "alice" = 5, "bob" = 3, total = 8
  assert.eq(key_count, 8)
end
"#;

    let source_file = SourceFile::new(
        SourceId(0),
        PathBuf::from("dict_keys.tea"),
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
fn for_loop_dict_with_break() -> anyhow::Result<()> {
    let source = r#"
use assert = "std.assert"

test "for loop dict with break"
  var numbers = { "a": 1, "b": 2, "c": 3 }
  var sum = 0
  
  for key, value of numbers
    if value == 2
      break
    end
    sum = sum + value
  end
  
  # Should only sum values before breaking (depends on iteration order)
  # Since HashMap iteration order isn't guaranteed, we just check it's less than full sum
  assert.ok(sum < 6)
end
"#;

    let source_file = SourceFile::new(
        SourceId(0),
        PathBuf::from("dict_break.tea"),
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
fn for_loop_dict_with_continue() -> anyhow::Result<()> {
    let source = r#"
use assert = "std.assert"

test "for loop dict with continue"
  var numbers = { "a": 1, "b": 2, "c": 3 }
  var sum = 0
  
  for key, value of numbers
    if value == 2
      continue
    end
    sum = sum + value
  end
  
  # Should sum 1 + 3 = 4 (skipping 2)
  assert.eq(sum, 4)
end
"#;

    let source_file = SourceFile::new(
        SourceId(0),
        PathBuf::from("dict_continue.tea"),
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
fn for_loop_dict_type_error() -> anyhow::Result<()> {
    let source = r#"
var numbers = [1, 2, 3]

for key, value of numbers
  var x = key
end
"#;

    let source_file = SourceFile::new(
        SourceId(0),
        PathBuf::from("dict_type_error.tea"),
        source.to_string(),
    );

    let mut compiler = Compiler::new(CompileOptions::default());
    let _ = compiler.compile(&source_file);

    // Should have a type error: can't iterate with two variables over a list
    assert!(
        !compiler.diagnostics().is_empty(),
        "expected type error for iterating list with two variables"
    );

    Ok(())
}
