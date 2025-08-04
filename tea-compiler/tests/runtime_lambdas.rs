use std::path::PathBuf;

use tea_compiler::{CompileOptions, Compiler, SourceFile, SourceId, Vm};

#[test]
fn lambda_captures_local_variable() -> anyhow::Result<()> {
    let source = r#"
use print = "std.print"

def run() -> Int
  var base = 40
  var add = |value: Int| => base + value
  add(2)
end

print.print(run())
"#;

    let mut compiler = Compiler::new(CompileOptions::default());
    let source_file = SourceFile::new(SourceId(0), PathBuf::from("lambda.tea"), source.to_string());
    let compilation = compiler.compile(&source_file)?;
    assert!(
        compiler.diagnostics().is_empty(),
        "expected no diagnostics, found {:?}",
        compiler.diagnostics()
    );

    let run_function = compilation
        .program
        .functions
        .iter()
        .find(|function| function.name == "run")
        .expect("expected run function to be emitted");
    let run_instruction_strings: Vec<String> = run_function
        .chunk
        .instructions
        .iter()
        .map(|instruction| instruction.to_string())
        .collect();
    assert!(
        run_instruction_strings
            .iter()
            .any(|text| text.starts_with("MAKE_CLOSURE")),
        "expected MAKE_CLOSURE instruction in run(), got {run_instruction_strings:?}"
    );

    let lambda_function = compilation
        .program
        .functions
        .iter()
        .find(|function| function.name.starts_with("<lambda:"))
        .expect("expected lambda function to be emitted");
    let lambda_instruction_strings: Vec<String> = lambda_function
        .chunk
        .instructions
        .iter()
        .map(|instruction| instruction.to_string())
        .collect();
    assert!(
        lambda_instruction_strings
            .iter()
            .any(|text| text == "GET_LOCAL 0"),
        "expected lambda to load captured base value, got {lambda_instruction_strings:?}"
    );
    assert!(
        lambda_instruction_strings
            .iter()
            .any(|text| text == "GET_LOCAL 1"),
        "expected lambda to load parameter value, got {lambda_instruction_strings:?}"
    );
    assert!(
        lambda_instruction_strings.iter().any(|text| text == "ADD"),
        "expected lambda to add base and parameter, got {lambda_instruction_strings:?}"
    );

    let mut vm = Vm::new(&compilation.program);
    vm.run()?;
    Ok(())
}
