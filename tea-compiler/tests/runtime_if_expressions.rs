use std::path::PathBuf;
use tea_compiler::{CompileOptions, Compiler, SourceFile, SourceId};

#[test]
fn test_basic_if_expression() -> anyhow::Result<()> {
    let source = r#"


const x = if (true) 1 else 2
print(x)

const y = if (false) 10 else 20
print(y)
"#;

    let mut compiler = Compiler::new(CompileOptions::default());
    let source_file = SourceFile::new(SourceId(0), PathBuf::from("test.tea"), source.to_string());
    compiler.compile(&source_file)?;

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
fn test_if_expression_with_strings() -> anyhow::Result<()> {
    let source = r#"


const greeting = if (true) "hello" else "goodbye"
print(greeting)
"#;

    let mut compiler = Compiler::new(CompileOptions::default());
    let source_file = SourceFile::new(SourceId(0), PathBuf::from("test.tea"), source.to_string());
    compiler.compile(&source_file)?;

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
fn test_nested_if_expressions() -> anyhow::Result<()> {
    let source = r#"


const x = if (true) if (false) 1 else 2 else 3
print(x)
"#;

    let mut compiler = Compiler::new(CompileOptions::default());
    let source_file = SourceFile::new(SourceId(0), PathBuf::from("test.tea"), source.to_string());
    compiler.compile(&source_file)?;

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
fn test_if_expression_in_compound_assignment() -> anyhow::Result<()> {
    let source = r#"


var x = 10
x += if (true) 5 else 1
print(x)
"#;

    let mut compiler = Compiler::new(CompileOptions::default());
    let source_file = SourceFile::new(SourceId(0), PathBuf::from("test.tea"), source.to_string());
    compiler.compile(&source_file)?;

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
fn test_if_expression_with_function_calls() -> anyhow::Result<()> {
    let source = r#"


def add(a: Int, b: Int) -> Int
  return a + b
end

const result = if (true) add(2, 3) else add(10, 20)
print(result)
"#;

    let mut compiler = Compiler::new(CompileOptions::default());
    let source_file = SourceFile::new(SourceId(0), PathBuf::from("test.tea"), source.to_string());
    compiler.compile(&source_file)?;

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
fn test_if_expression_type_error() {
    let source = r#"
const x = if (true) 1 else "string"
"#;

    let mut compiler = Compiler::new(CompileOptions::default());
    let source_file = SourceFile::new(SourceId(0), PathBuf::from("test.tea"), source.to_string());
    let result = compiler.compile(&source_file);
    // Should fail due to incompatible types
    assert!(result.is_err());
}

#[test]
fn test_if_expression_requires_else() {
    // This should fail to parse since if-expressions require else
    let source = r#"
const x = if (true) 1
"#;

    let mut compiler = Compiler::new(CompileOptions::default());
    let source_file = SourceFile::new(SourceId(0), PathBuf::from("test.tea"), source.to_string());
    let result = compiler.compile(&source_file);
    assert!(result.is_err());
}
