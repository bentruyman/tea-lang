use std::path::PathBuf;

use tea_compiler::{CompileOptions, Compiler, SourceFile, SourceId, Statement};

#[test]
fn parses_function_docstring() -> anyhow::Result<()> {
    let source_text = r#"## Remove a file or directory.
## @param path String — Path to remove.
def remove(path: String) -> String
  path
end
"#;

    let source = SourceFile::new(
        SourceId(0),
        PathBuf::from("docstring.tea"),
        source_text.to_string(),
    );
    let mut compiler = Compiler::new(CompileOptions::default());
    let compilation = compiler.compile(&source)?;

    assert!(
        !compiler.diagnostics().has_errors(),
        "unexpected diagnostics: {:?}",
        compiler.diagnostics()
    );

    let module = compilation.module;
    match module.statements.first() {
        Some(Statement::Function(function)) => {
            assert_eq!(
                function.docstring.as_deref(),
                Some("Remove a file or directory.\n@param path String — Path to remove.")
            );
        }
        other => panic!("expected function statement, found {:?}", other),
    }

    Ok(())
}

#[test]
fn builtin_print_works_without_import() -> anyhow::Result<()> {
    let source_text = r#"

def main() -> Void
  print("hello")
end
"#;

    let source = SourceFile::new(
        SourceId(0),
        PathBuf::from("builtin_print.tea"),
        source_text.to_string(),
    );
    let mut compiler = Compiler::new(CompileOptions::default());
    compiler.compile(&source)?;

    assert!(
        !compiler.diagnostics().has_errors(),
        "unexpected diagnostics: {:?}",
        compiler.diagnostics()
    );

    Ok(())
}

#[test]
fn parses_variable_docstring() -> anyhow::Result<()> {
    let source_text = r#"## Enable this to see the flag
var flag: Bool = false

def main() -> Bool
  flag
end
"#;

    let source = SourceFile::new(
        SourceId(0),
        PathBuf::from("flags.tea"),
        source_text.to_string(),
    );
    let mut compiler = Compiler::new(CompileOptions::default());
    let compilation = compiler.compile(&source)?;

    assert!(
        !compiler.diagnostics().has_errors(),
        "unexpected diagnostics: {:?}",
        compiler.diagnostics()
    );

    match compilation.module.statements.first() {
        Some(Statement::Var(var_stmt)) => {
            assert_eq!(
                var_stmt.docstring.as_deref(),
                Some("Enable this to see the flag")
            );
        }
        other => panic!("expected var statement, found {:?}", other),
    }

    Ok(())
}
