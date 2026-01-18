use std::path::PathBuf;

use tea_compiler::{CompileOptions, Compiler, SourceFile, SourceId};

/// Test List.map transforms each element
#[test]
fn test_list_map() -> anyhow::Result<()> {
    let source = r#"
def run() -> List[Int]
  var numbers = [1, 2, 3]
  numbers.map(|x: Int| => x * 2)
end

print(run())
"#;

    let mut compiler = Compiler::new(CompileOptions::default());
    let source_file = SourceFile::new(
        SourceId(0),
        PathBuf::from("list_map.tea"),
        source.to_string(),
    );
    compiler.compile(&source_file)?;
    assert!(
        compiler.diagnostics().is_empty(),
        "expected no diagnostics, found {:?}",
        compiler.diagnostics()
    );

    Ok(())
}

/// Test List.filter keeps elements matching predicate
#[test]
fn test_list_filter() -> anyhow::Result<()> {
    let source = r#"
def run() -> List[Int]
  var numbers = [1, 2, 3, 4, 5]
  numbers.filter(|x: Int| => x > 2)
end

print(run())
"#;

    let mut compiler = Compiler::new(CompileOptions::default());
    let source_file = SourceFile::new(
        SourceId(0),
        PathBuf::from("list_filter.tea"),
        source.to_string(),
    );
    compiler.compile(&source_file)?;
    assert!(
        compiler.diagnostics().is_empty(),
        "expected no diagnostics, found {:?}",
        compiler.diagnostics()
    );

    Ok(())
}

/// Test List.reduce folds list to single value
#[test]
fn test_list_reduce() -> anyhow::Result<()> {
    let source = r#"
def run() -> Int
  var numbers = [1, 2, 3, 4, 5]
  numbers.reduce(0, |acc: Int, x: Int| => acc + x)
end

print(run())
"#;

    let mut compiler = Compiler::new(CompileOptions::default());
    let source_file = SourceFile::new(
        SourceId(0),
        PathBuf::from("list_reduce.tea"),
        source.to_string(),
    );
    compiler.compile(&source_file)?;
    assert!(
        compiler.diagnostics().is_empty(),
        "expected no diagnostics, found {:?}",
        compiler.diagnostics()
    );

    Ok(())
}

/// Test List.find returns first matching element
#[test]
fn test_list_find() -> anyhow::Result<()> {
    let source = r#"
def run() -> Int?
  var numbers = [1, 2, 3, 4, 5]
  numbers.find(|x: Int| => x > 3)
end

print(run())
"#;

    let mut compiler = Compiler::new(CompileOptions::default());
    let source_file = SourceFile::new(
        SourceId(0),
        PathBuf::from("list_find.tea"),
        source.to_string(),
    );
    compiler.compile(&source_file)?;
    assert!(
        compiler.diagnostics().is_empty(),
        "expected no diagnostics, found {:?}",
        compiler.diagnostics()
    );

    Ok(())
}

/// Test List.any returns true if any element matches
#[test]
fn test_list_any() -> anyhow::Result<()> {
    let source = r#"
def run() -> Bool
  var numbers = [1, 2, 3, 4, 5]
  numbers.any(|x: Int| => x > 3)
end

print(run())
"#;

    let mut compiler = Compiler::new(CompileOptions::default());
    let source_file = SourceFile::new(
        SourceId(0),
        PathBuf::from("list_any.tea"),
        source.to_string(),
    );
    compiler.compile(&source_file)?;
    assert!(
        compiler.diagnostics().is_empty(),
        "expected no diagnostics, found {:?}",
        compiler.diagnostics()
    );

    Ok(())
}

/// Test List.all returns true if all elements match
#[test]
fn test_list_all() -> anyhow::Result<()> {
    let source = r#"
def run() -> Bool
  var numbers = [1, 2, 3, 4, 5]
  numbers.all(|x: Int| => x > 0)
end

print(run())
"#;

    let mut compiler = Compiler::new(CompileOptions::default());
    let source_file = SourceFile::new(
        SourceId(0),
        PathBuf::from("list_all.tea"),
        source.to_string(),
    );
    compiler.compile(&source_file)?;
    assert!(
        compiler.diagnostics().is_empty(),
        "expected no diagnostics, found {:?}",
        compiler.diagnostics()
    );

    Ok(())
}

/// Test Dict.keys returns all keys
#[test]
fn test_dict_keys() -> anyhow::Result<()> {
    let source = r#"
def run() -> List[String]
  var data = {"a": 1, "b": 2, "c": 3}
  data.keys()
end

print(run())
"#;

    let mut compiler = Compiler::new(CompileOptions::default());
    let source_file = SourceFile::new(
        SourceId(0),
        PathBuf::from("dict_keys.tea"),
        source.to_string(),
    );
    compiler.compile(&source_file)?;
    assert!(
        compiler.diagnostics().is_empty(),
        "expected no diagnostics, found {:?}",
        compiler.diagnostics()
    );

    Ok(())
}

/// Test Dict.values returns all values
#[test]
fn test_dict_values() -> anyhow::Result<()> {
    let source = r#"
def run() -> List[Int]
  var data = {"a": 1, "b": 2, "c": 3}
  data.values()
end

print(run())
"#;

    let mut compiler = Compiler::new(CompileOptions::default());
    let source_file = SourceFile::new(
        SourceId(0),
        PathBuf::from("dict_values.tea"),
        source.to_string(),
    );
    compiler.compile(&source_file)?;
    assert!(
        compiler.diagnostics().is_empty(),
        "expected no diagnostics, found {:?}",
        compiler.diagnostics()
    );

    Ok(())
}

/// Test Dict.entries returns all entries
#[test]
fn test_dict_entries() -> anyhow::Result<()> {
    let source = r#"
def run() -> List[Dict[String, Int]]
  var data = {"a": 1, "b": 2, "c": 3}
  data.entries()
end

print(run())
"#;

    let mut compiler = Compiler::new(CompileOptions::default());
    let source_file = SourceFile::new(
        SourceId(0),
        PathBuf::from("dict_entries.tea"),
        source.to_string(),
    );
    compiler.compile(&source_file)?;
    assert!(
        compiler.diagnostics().is_empty(),
        "expected no diagnostics, found {:?}",
        compiler.diagnostics()
    );

    Ok(())
}

/// Test chained method calls
#[test]
fn test_chained_methods() -> anyhow::Result<()> {
    let source = r#"
def run() -> Int
  var numbers = [1, 2, 3, 4, 5]
  numbers.filter(|x: Int| => x > 1).map(|x: Int| => x * 2).reduce(0, |acc: Int, x: Int| => acc + x)
end

print(run())
"#;

    let mut compiler = Compiler::new(CompileOptions::default());
    let source_file = SourceFile::new(
        SourceId(0),
        PathBuf::from("chained_methods.tea"),
        source.to_string(),
    );
    compiler.compile(&source_file)?;
    assert!(
        compiler.diagnostics().is_empty(),
        "expected no diagnostics, found {:?}",
        compiler.diagnostics()
    );

    Ok(())
}

/// Test error on wrong argument count for map
#[test]
fn test_map_wrong_arg_count() {
    let source = r#"
def run() -> List[Int]
  var numbers = [1, 2, 3]
  numbers.map()
end

print(run())
"#;

    let mut compiler = Compiler::new(CompileOptions::default());
    let source_file = SourceFile::new(
        SourceId(0),
        PathBuf::from("map_wrong_args.tea"),
        source.to_string(),
    );
    // Expect compile to fail or produce diagnostics
    let result = compiler.compile(&source_file);
    assert!(
        result.is_err() || !compiler.diagnostics().is_empty(),
        "expected error or diagnostics for wrong argument count"
    );
}

/// Test error on non-boolean return from filter
#[test]
fn test_filter_non_bool_return() {
    let source = r#"
def run() -> List[Int]
  var numbers = [1, 2, 3]
  numbers.filter(|x: Int| => x * 2)
end

print(run())
"#;

    let mut compiler = Compiler::new(CompileOptions::default());
    let source_file = SourceFile::new(
        SourceId(0),
        PathBuf::from("filter_non_bool.tea"),
        source.to_string(),
    );
    // Expect compile to fail or produce diagnostics
    let result = compiler.compile(&source_file);
    assert!(
        result.is_err() || !compiler.diagnostics().is_empty(),
        "expected error or diagnostics for non-boolean filter return"
    );
}

/// Test error on unknown List method
#[test]
fn test_unknown_list_method() {
    let source = r#"
def run() -> List[Int]
  var numbers = [1, 2, 3]
  numbers.unknown_method(|x: Int| => x)
end

print(run())
"#;

    let mut compiler = Compiler::new(CompileOptions::default());
    let source_file = SourceFile::new(
        SourceId(0),
        PathBuf::from("unknown_method.tea"),
        source.to_string(),
    );
    // Expect compile to fail or produce diagnostics
    let result = compiler.compile(&source_file);
    assert!(
        result.is_err() || !compiler.diagnostics().is_empty(),
        "expected error or diagnostics for unknown method"
    );
}
