use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use tea_compiler::{
    CompileOptions, CompileTarget, Compiler, InMemoryModuleLoader, SourceFile, SourceId,
};
use tea_eval::{evaluate, EvalOptions};

fn compile_browser_source(source: &str) -> tea_compiler::Compilation {
    let entry_path = PathBuf::from("/main.tea");
    let loader =
        InMemoryModuleLoader::new(HashMap::from([(entry_path.clone(), source.to_string())]))
            .with_browser_stdlib();
    let source = SourceFile::new(SourceId(0), entry_path, source.to_string());
    let mut compiler = Compiler::new(CompileOptions {
        target: CompileTarget::Browser,
        module_loader: Some(Arc::new(loader)),
        ..CompileOptions::default()
    });

    compiler.compile(&source).unwrap_or_else(|error| {
        panic!(
            "browser compilation to succeed: {error}; diagnostics={:?}",
            compiler.diagnostics().entries()
        )
    })
}

#[test]
fn browser_eval_runs_structs_loops_and_stdlib() {
    let compilation = compile_browser_source(
        r#"
use string = "std.string"

struct User {
  name: String
  age: Int
}

var user = User(name: "Ada", age: 37)
var total = 0

for value in [1, 2, 3]
  total = total + value
end

@println(string.to_upper(user.name))
@println(total)
"#,
    );

    let output = evaluate(&compilation, EvalOptions::default());
    assert_eq!(output.runtime_error, None);
    assert_eq!(output.stdout, vec!["ADA\n".to_string(), "6\n".to_string()]);
}

#[test]
fn browser_eval_handles_json_decode_and_fuel_limits() {
    let compilation = compile_browser_source(
        r#"
use json = "std.json"

@println(json.encode([1, 2, 3]))
"#,
    );

    let output = evaluate(&compilation, EvalOptions { fuel: 10_000 });
    assert_eq!(output.runtime_error, None);
    assert_eq!(output.stdout, vec!["[1,2,3]\n".to_string()]);

    let loop_compilation = compile_browser_source(
        r#"
while true
end
"#,
    );
    let loop_output = evaluate(&loop_compilation, EvalOptions { fuel: 32 });
    assert_eq!(
        loop_output.runtime_error,
        Some("execution limit reached".to_string())
    );
}
