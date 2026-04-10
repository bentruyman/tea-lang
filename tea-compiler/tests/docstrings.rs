use std::path::PathBuf;
use std::{fs, path::Path};

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

#[test]
fn public_stdlib_functions_have_docstrings() -> anyhow::Result<()> {
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root");

    for path in [
        "stdlib/args/mod.tea",
        "stdlib/assert/mod.tea",
        "stdlib/env/mod.tea",
        "stdlib/fs/mod.tea",
        "stdlib/json/mod.tea",
        "stdlib/path/mod.tea",
        "stdlib/process/mod.tea",
        "stdlib/regex/mod.tea",
        "stdlib/string/mod.tea",
    ] {
        let absolute_path = workspace_root.join(path);
        let source_text = fs::read_to_string(&absolute_path)?;
        let source = SourceFile::new(SourceId(0), absolute_path.clone(), source_text);
        let mut compiler = Compiler::new(CompileOptions::default());
        let parsed = compiler.parse_source(&source)?;

        assert!(
            !compiler.diagnostics().has_errors(),
            "unexpected diagnostics for {}: {:?}",
            path,
            compiler.diagnostics()
        );

        for statement in parsed.into_module().statements {
            let Statement::Function(function) = statement else {
                continue;
            };
            if function.is_public {
                let doc = function.docstring.as_deref().unwrap_or("").trim();
                assert!(
                    !doc.is_empty(),
                    "public stdlib function {} in {} is missing a docstring",
                    function.name,
                    path
                );
            }
        }
    }

    Ok(())
}
