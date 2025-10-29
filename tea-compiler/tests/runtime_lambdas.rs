use std::path::PathBuf;

use tea_compiler::{CompileOptions, Compiler, SourceFile, SourceId, Vm};

#[test]
fn lambda_captures_local_variable() -> anyhow::Result<()> {
    let source = r#"
use debug = "std.debug"

def run() -> Int
  var base = 40
  var add = |value: Int| => base + value
  add(2)
end

debug.print(run())
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

#[test]
fn anonymous_function_definition_works() -> anyhow::Result<()> {
    let source = r#"
use debug = "std.debug"

def make_adder(x: Int) -> Func(Int) -> Int
  return def(y: Int) -> Int
    return x + y
  end
end

def run() -> Int
  var adder = make_adder(42)
  adder(10)
end

debug.print(run())
"#;

    let mut compiler = Compiler::new(CompileOptions::default());
    let source_file = SourceFile::new(
        SourceId(0),
        PathBuf::from("anon_func.tea"),
        source.to_string(),
    );
    let compilation = compiler.compile(&source_file)?;
    assert!(
        compiler.diagnostics().is_empty(),
        "expected no diagnostics, found {:?}",
        compiler.diagnostics()
    );

    let make_adder_function = compilation
        .program
        .functions
        .iter()
        .find(|function| function.name == "make_adder")
        .expect("expected make_adder function to be emitted");
    let instruction_strings: Vec<String> = make_adder_function
        .chunk
        .instructions
        .iter()
        .map(|instruction| instruction.to_string())
        .collect();
    assert!(
        instruction_strings
            .iter()
            .any(|text| text.starts_with("MAKE_CLOSURE")),
        "expected MAKE_CLOSURE instruction in make_adder(), got {instruction_strings:?}"
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
        "expected lambda to load captured x value, got {lambda_instruction_strings:?}"
    );
    assert!(
        lambda_instruction_strings
            .iter()
            .any(|text| text == "GET_LOCAL 1"),
        "expected lambda to load parameter y, got {lambda_instruction_strings:?}"
    );
    assert!(
        lambda_instruction_strings.iter().any(|text| text == "ADD"),
        "expected lambda to add x and y, got {lambda_instruction_strings:?}"
    );

    let mut vm = Vm::new(&compilation.program);
    vm.run()?;
    Ok(())
}
