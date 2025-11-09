use std::path::PathBuf;

use tea_compiler::{CompileOptions, Compiler, SourceFile, SourceId};

#[test]
fn for_loop_iterates_list() -> anyhow::Result<()> {
    let source = r#"
use assert = "std.assert"

test "for loop iterates over list"
  var sum = 0
  var numbers = [1, 2, 3]
  
  for num of numbers
    sum = sum + num
  end
  
  assert.eq(sum, 6)
end
"#;

    let source_file = SourceFile::new(
        SourceId(0),
        PathBuf::from("for_loop.tea"),
        source.to_string(),
    );

    let mut compiler = Compiler::new(CompileOptions::default());
    let compilation = compiler.compile(&source_file)?;
    assert!(
        compiler.diagnostics().is_empty(),
        "expected no diagnostics, found {:?}",
        compiler.diagnostics()
    );

    // Note: This test was converted from VM-based execution to AOT compilation-only
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
fn for_loop_with_empty_list() -> anyhow::Result<()> {
    let source = r#"
use assert = "std.assert"

test "for loop with empty list"
  var count = 0
  var empty = []
  
  for item of empty
    count = count + 1
  end
  
  assert.eq(count, 0)
end
"#;

    let source_file = SourceFile::new(
        SourceId(0),
        PathBuf::from("for_loop_empty.tea"),
        source.to_string(),
    );

    let mut compiler = Compiler::new(CompileOptions::default());
    let compilation = compiler.compile(&source_file)?;
    assert!(
        compiler.diagnostics().is_empty(),
        "expected no diagnostics, found {:?}",
        compiler.diagnostics()
    );

    // Note: This test was converted from VM-based execution to AOT compilation-only
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
fn for_loop_with_strings() -> anyhow::Result<()> {
    let source = r#"
use assert = "std.assert"

test "for loop with string list"
  var count = 0
  var words = ["Hello", "World", "!"]
  
  for word of words
    count = count + length(word)
  end
  
  assert.eq(count, 11)
end
"#;

    let source_file = SourceFile::new(
        SourceId(0),
        PathBuf::from("for_loop_strings.tea"),
        source.to_string(),
    );

    let mut compiler = Compiler::new(CompileOptions::default());
    let compilation = compiler.compile(&source_file)?;
    assert!(
        compiler.diagnostics().is_empty(),
        "expected no diagnostics, found {:?}",
        compiler.diagnostics()
    );

    // Note: This test was converted from VM-based execution to AOT compilation-only
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
fn for_loop_nested() -> anyhow::Result<()> {
    let source = r#"
use assert = "std.assert"

test "nested for loops"
  var result = 0
  var outer = [1, 2]
  var inner = [10, 20]
  
  for i of outer
    for j of inner
      result = result + (i * j)
    end
  end
  
  # (1*10 + 1*20 + 2*10 + 2*20) = 10 + 20 + 20 + 40 = 90
  assert.eq(result, 90)
end
"#;

    let source_file = SourceFile::new(
        SourceId(0),
        PathBuf::from("for_loop_nested.tea"),
        source.to_string(),
    );

    let mut compiler = Compiler::new(CompileOptions::default());
    let compilation = compiler.compile(&source_file)?;
    assert!(
        compiler.diagnostics().is_empty(),
        "expected no diagnostics, found {:?}",
        compiler.diagnostics()
    );

    // Note: This test was converted from VM-based execution to AOT compilation-only
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
fn for_loop_with_break() -> anyhow::Result<()> {
    let source = r#"
use assert = "std.assert"

test "for loop with break"
  var sum = 0
  var numbers = [1, 2, 3, 4, 5]
  
  for num of numbers
    if num == 4
      break
    end
    sum = sum + num
  end
  
  assert.eq(sum, 6)  # 1 + 2 + 3 = 6
end
"#;

    let source_file = SourceFile::new(
        SourceId(0),
        PathBuf::from("for_loop_break.tea"),
        source.to_string(),
    );

    let mut compiler = Compiler::new(CompileOptions::default());
    let compilation = compiler.compile(&source_file)?;
    assert!(
        compiler.diagnostics().is_empty(),
        "expected no diagnostics, found {:?}",
        compiler.diagnostics()
    );

    // Note: This test was converted from VM-based execution to AOT compilation-only
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
fn for_loop_with_continue() -> anyhow::Result<()> {
    let source = r#"
use assert = "std.assert"

test "for loop with continue"
  var sum = 0
  var numbers = [1, 2, 3, 4, 5]
  
  for num of numbers
    if num == 3
      continue
    end
    sum = sum + num
  end
  
  assert.eq(sum, 12)  # 1 + 2 + 4 + 5 = 12
end
"#;

    let source_file = SourceFile::new(
        SourceId(0),
        PathBuf::from("for_loop_continue.tea"),
        source.to_string(),
    );

    let mut compiler = Compiler::new(CompileOptions::default());
    let compilation = compiler.compile(&source_file)?;
    assert!(
        compiler.diagnostics().is_empty(),
        "expected no diagnostics, found {:?}",
        compiler.diagnostics()
    );

    // Note: This test was converted from VM-based execution to AOT compilation-only
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
