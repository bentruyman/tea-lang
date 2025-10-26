use std::path::PathBuf;

use tea_compiler::{CompileOptions, Compiler, SourceFile, SourceId};

#[test]
fn rejects_mismatched_annotation() {
    let source = "use debug = \"std.debug\"\nvar flag: Bool = 1\n";
    let mut compiler = Compiler::new(CompileOptions::default());
    let source_file = SourceFile::new(SourceId(0), PathBuf::from("test.tea"), source.to_string());
    let result = compiler.compile(&source_file);
    assert!(
        result.is_err(),
        "expected type checker to reject mismatched annotations"
    );
}

#[test]
fn rejects_untyped_function_parameters() {
    let source = "def foo(x)\n  x\nend\n";
    let mut compiler = Compiler::new(CompileOptions::default());
    let source_file = SourceFile::new(
        SourceId(0),
        PathBuf::from("test_fn.tea"),
        source.to_string(),
    );
    let result = compiler.compile(&source_file);
    assert!(
        result.is_err(),
        "expected type checker to require parameter annotations"
    );
}

#[test]
fn requires_alias_for_use_statement() {
    let source = "use \"std.debug\"\nprint(\"hi\")\n";
    let mut compiler = Compiler::new(CompileOptions::default());
    let source_file = SourceFile::new(
        SourceId(0),
        PathBuf::from("missing_alias.tea"),
        source.to_string(),
    );
    let result = compiler.compile(&source_file);
    assert!(result.is_err(), "expected parser to reject missing alias");
    let messages: Vec<_> = compiler
        .diagnostics()
        .entries()
        .iter()
        .map(|d| d.message.as_str())
        .collect();
    assert!(
        messages
            .iter()
            .any(|msg| msg.contains("module imports must specify an alias")),
        "expected alias requirement diagnostic, found {:?}",
        messages
    );
}

#[test]
fn rejects_return_type_mismatch() {
    let source = "def foo() -> Int\n  return true\nend\n";
    let mut compiler = Compiler::new(CompileOptions::default());
    let source_file = SourceFile::new(
        SourceId(0),
        PathBuf::from("test_return.tea"),
        source.to_string(),
    );
    let result = compiler.compile(&source_file);
    assert!(
        result.is_err(),
        "expected mismatched return type to be rejected"
    );
}

#[test]
fn rejects_missing_return_value() {
    let source = "def foo() -> Int\n  var x = 1\nend\n";
    let mut compiler = Compiler::new(CompileOptions::default());
    let source_file = SourceFile::new(
        SourceId(0),
        PathBuf::from("test_missing_return.tea"),
        source.to_string(),
    );
    let result = compiler.compile(&source_file);
    assert!(
        result.is_err(),
        "expected function without return to be rejected"
    );
}

#[test]
fn rejects_argument_type_mismatch() {
    let source = "def inc(value: Int) -> Int\n  value + 1\nend\n\ninc(true)\n";
    let mut compiler = Compiler::new(CompileOptions::default());
    let source_file = SourceFile::new(
        SourceId(0),
        PathBuf::from("test_call.tea"),
        source.to_string(),
    );
    let result = compiler.compile(&source_file);
    assert!(
        result.is_err(),
        "expected argument type mismatch to be rejected"
    );
}

#[test]
fn reports_generic_function_type_argument_mismatch() {
    let source = r#"
def pair[T, U](left: T, right: U) -> T
  if right == right
    left
  else
    left
  end
end

pair[Int](1, 2)
"#;
    let mut compiler = Compiler::new(CompileOptions::default());
    let source_file = SourceFile::new(
        SourceId(0),
        PathBuf::from("generic_fn.tea"),
        source.to_string(),
    );
    let result = compiler.compile(&source_file);
    assert!(result.is_err(), "expected compilation to fail");
    let messages: Vec<_> = compiler
        .diagnostics()
        .entries()
        .iter()
        .map(|d| d.message.as_str())
        .collect();
    assert!(
        messages.iter().any(|msg| msg
            .contains("function 'pair' expects 2 type arguments [<T>, <U>] but 1 provided")),
        "missing type argument diagnostic: {:?}",
        messages
    );
}

#[test]
fn reports_generic_struct_inference_hint() {
    let source = r#"
struct Phantom[T] {
}

var phantom = Phantom()
"#;
    let mut compiler = Compiler::new(CompileOptions::default());
    let source_file = SourceFile::new(
        SourceId(0),
        PathBuf::from("generic_struct.tea"),
        source.to_string(),
    );
    let result = compiler.compile(&source_file);
    assert!(result.is_err(), "expected compilation to fail");
    let messages: Vec<_> = compiler
        .diagnostics()
        .entries()
        .iter()
        .map(|d| d.message.as_str())
        .collect();
    assert!(
        messages.iter().any(|msg| msg.contains(
            "could not infer type for parameter 'T' when constructing 'Phantom'; consider spelling the type arguments explicitly like Phantom[T]"
        )),
        "missing inference hint: {:?}",
        messages
    );
}

#[test]
fn rejects_struct_generics_closing_on_newline() {
    let source = "struct Box[T\n]\n{\n}\n";
    let mut compiler = Compiler::new(CompileOptions::default());
    let source_file = SourceFile::new(
        SourceId(0),
        PathBuf::from("struct_generics_newline.tea"),
        source.to_string(),
    );
    let result = compiler.compile(&source_file);
    assert!(
        result.is_err(),
        "expected parser to reject trailing newline before ']' in struct generics"
    );
    let messages: Vec<_> = compiler
        .diagnostics()
        .entries()
        .iter()
        .map(|d| d.message.as_str())
        .collect();
    assert!(
        messages
            .iter()
            .any(|msg| msg.contains("newline before closing ']' in struct 'Box' type parameters")),
        "missing struct generic newline diagnostic: {:?}",
        messages
    );
}

#[test]
fn rejects_union_with_unsupported_member_type() {
    let source = r#"
union Bad {
  List[Int]
}
"#;
    let mut compiler = Compiler::new(CompileOptions::default());
    let source_file = SourceFile::new(
        SourceId(0),
        PathBuf::from("union_bad.tea"),
        source.to_string(),
    );
    let result = compiler.compile(&source_file);
    assert!(
        result.is_err(),
        "expected compiler to reject unsupported union member type"
    );
    let messages: Vec<_> = compiler
        .diagnostics()
        .entries()
        .iter()
        .map(|d| d.message.as_str())
        .collect();
    assert!(
        messages
            .iter()
            .any(|msg| msg.contains("union 'Bad' member type 'List[Int]' is not supported")),
        "expected unsupported union member diagnostic, got {:?}",
        messages
    );
}

#[test]
fn rejects_impossible_type_test() {
    let source = r#"
var value = 5
if value is String
  value
end
"#;
    let mut compiler = Compiler::new(CompileOptions::default());
    let source_file = SourceFile::new(
        SourceId(0),
        PathBuf::from("impossible_type_test.tea"),
        source.to_string(),
    );
    let result = compiler.compile(&source_file);
    assert!(
        result.is_err(),
        "expected type test on incompatible types to fail"
    );
    let messages: Vec<_> = compiler
        .diagnostics()
        .entries()
        .iter()
        .map(|d| d.message.as_str())
        .collect();
    assert!(
        messages
            .iter()
            .any(|msg| msg.contains("type test will always be false")),
        "expected incompatible type test diagnostic, got {:?}",
        messages
    );
}

#[test]
fn rejects_function_generics_closing_on_newline() {
    let source = "def id[T\n](value: T) -> T\n  value\nend\n";
    let mut compiler = Compiler::new(CompileOptions::default());
    let source_file = SourceFile::new(
        SourceId(0),
        PathBuf::from("function_generics_newline.tea"),
        source.to_string(),
    );
    let result = compiler.compile(&source_file);
    assert!(
        result.is_err(),
        "expected parser to reject trailing newline before ']' in function generics"
    );
    let messages: Vec<_> = compiler
        .diagnostics()
        .entries()
        .iter()
        .map(|d| d.message.as_str())
        .collect();
    assert!(
        messages
            .iter()
            .any(|msg| msg.contains("newline before closing ']' in function 'id' type parameters")),
        "missing function generic newline diagnostic: {:?}",
        messages
    );
}

#[test]
fn conditional_type_error_includes_span() {
    let source = "if 1\n  nil\nend\n";
    let mut compiler = Compiler::new(CompileOptions::default());
    let source_file = SourceFile::new(
        SourceId(0),
        PathBuf::from("conditional.tea"),
        source.to_string(),
    );
    let result = compiler.compile(&source_file);
    assert!(result.is_err(), "expected conditional type error");

    let diagnostics = compiler.diagnostics().entries();
    let diagnostic = diagnostics
        .iter()
        .find(|d| d.message.contains("conditional expression"))
        .expect("conditional diagnostic to be present");
    let span = diagnostic
        .span
        .expect("conditional diagnostic should have a span");
    assert_eq!(span.line, 1);
    assert_eq!(span.column, 4);
}

#[test]
fn rejects_non_int_index() {
    let source = "var numbers = [1, 2, 3]\nvar value = numbers[true]\n";
    let mut compiler = Compiler::new(CompileOptions::default());
    let source_file = SourceFile::new(
        SourceId(0),
        PathBuf::from("test_index.tea"),
        source.to_string(),
    );
    let result = compiler.compile(&source_file);
    assert!(result.is_err(), "expected non-Int index to be rejected");
}

#[test]
fn rejects_unknown_module_import() {
    let source = "use \"std.unknown\"\n";
    let mut compiler = Compiler::new(CompileOptions::default());
    let source_file = SourceFile::new(
        SourceId(0),
        PathBuf::from("test_module.tea"),
        source.to_string(),
    );
    let result = compiler.compile(&source_file);
    assert!(
        result.is_err(),
        "expected unknown module import to be rejected"
    );
}

#[test]
fn rejects_mixed_list_elements() {
    let source = "var values = [1, \"two\"]\n";
    let mut compiler = Compiler::new(CompileOptions::default());
    let source_file = SourceFile::new(
        SourceId(0),
        PathBuf::from("test_list_elements.tea"),
        source.to_string(),
    );
    let result = compiler.compile(&source_file);
    assert!(
        result.is_err(),
        "expected list with mixed element types to be rejected"
    );
}

#[test]
fn rejects_incompatible_list_assignment() {
    let source = "var values = [1, 2]\nvalues = [true]\n";
    let mut compiler = Compiler::new(CompileOptions::default());
    let source_file = SourceFile::new(
        SourceId(0),
        PathBuf::from("test_list_assignment.tea"),
        source.to_string(),
    );
    let result = compiler.compile(&source_file);
    assert!(
        result.is_err(),
        "expected list reassignment with different element types to be rejected"
    );
}

#[test]
fn loop_condition_type_error_includes_span() {
    let source = "while 1\n  nil\nend\n";
    let mut compiler = Compiler::new(CompileOptions::default());
    let source_file = SourceFile::new(SourceId(0), PathBuf::from("loop.tea"), source.to_string());
    let result = compiler.compile(&source_file);
    assert!(result.is_err(), "expected loop condition type error");

    let diagnostics = compiler.diagnostics().entries();
    let diagnostic = diagnostics
        .iter()
        .find(|d| d.message.contains("loop condition"))
        .expect("loop condition diagnostic to be present");
    let span = diagnostic
        .span
        .expect("loop condition diagnostic should have a span");
    assert_eq!(span.line, 1);
    assert_eq!(span.column, 7);
}

#[test]
fn rejects_dict_annotation_with_non_string_key() {
    let source = "var mapping: Dict[Int, Int] = { \"a\": 1 }\n";
    let mut compiler = Compiler::new(CompileOptions::default());
    let source_file = SourceFile::new(
        SourceId(0),
        PathBuf::from("test_dict_annotation.tea"),
        source.to_string(),
    );
    let result = compiler.compile(&source_file);
    assert!(
        result.is_err(),
        "expected dict annotation with non-string key to be rejected"
    );
}

#[test]
fn rejects_function_annotation_mismatch() {
    let source = r#"
def id(value: Int) -> Int
  value
end

var identity: Func(Int) -> Bool = id
"#;
    let mut compiler = Compiler::new(CompileOptions::default());
    let source_file = SourceFile::new(
        SourceId(0),
        PathBuf::from("test_func_annotation.tea"),
        source.to_string(),
    );
    let result = compiler.compile(&source_file);
    assert!(
        result.is_err(),
        "expected function annotation mismatch to be rejected"
    );
}

#[test]
fn rejects_lambda_return_mismatch() {
    let source = "var double: Func(Int) -> Int = |x: Int| => x == 0\n";
    let mut compiler = Compiler::new(CompileOptions::default());
    let source_file = SourceFile::new(
        SourceId(0),
        PathBuf::from("test_lambda_return.tea"),
        source.to_string(),
    );
    let result = compiler.compile(&source_file);
    assert!(
        result.is_err(),
        "expected lambda with mismatched return type to be rejected"
    );
}

#[test]
fn accepts_lambda_annotations() {
    let source = "var double: Func(Int) -> Int = |x: Int| => x + 1\n";
    let mut compiler = Compiler::new(CompileOptions::default());
    let source_file = SourceFile::new(
        SourceId(0),
        PathBuf::from("test_lambda_annotation.tea"),
        source.to_string(),
    );
    let result = compiler.compile(&source_file);
    assert!(result.is_ok(), "expected lambda compilation to succeed");
    assert!(
        compiler.diagnostics().is_empty(),
        "expected no type diagnostics, found {:?}",
        compiler.diagnostics()
    );
}

#[test]
fn accepts_container_annotations() {
    let source = r#"
use debug = "std.debug"

var values: List[Int] = [1, 2, 3]
var lookup: Dict[String, Int] = { foo: 42 }

def apply(value: Int) -> Int
  value + 1
end

var transformer: Func(Int) -> Int = apply

debug.print(transformer(values[0]))
"#;
    let mut compiler = Compiler::new(CompileOptions::default());
    let source_file = SourceFile::new(
        SourceId(0),
        PathBuf::from("test_container_annotations.tea"),
        source.to_string(),
    );
    let result = compiler.compile(&source_file);
    assert!(
        result.is_ok(),
        "expected container annotations to compile, err: {:?}, diagnostics {:?}",
        result.err(),
        compiler.diagnostics()
    );
    assert!(
        compiler.diagnostics().is_empty(),
        "expected no diagnostics, found {:?}",
        compiler.diagnostics()
    );
}

#[test]
fn rejects_const_reassignment() {
    let source = "const answer = 42\nanswer = 0\n";
    let mut compiler = Compiler::new(CompileOptions::default());
    let source_file = SourceFile::new(
        SourceId(0),
        PathBuf::from("const_reassign.tea"),
        source.to_string(),
    );
    let result = compiler.compile(&source_file);
    assert!(
        result.is_err(),
        "expected const reassignment to be rejected"
    );
}

#[test]
fn rejects_const_without_initializer() {
    let source = "const name\n";
    let mut compiler = Compiler::new(CompileOptions::default());
    let source_file = SourceFile::new(
        SourceId(0),
        PathBuf::from("const_init.tea"),
        source.to_string(),
    );
    let result = compiler.compile(&source_file);
    assert!(
        result.is_err(),
        "expected const without initializer to be rejected"
    );
}

#[test]
fn rejects_const_member_assignment() {
    let source = "const foo = { x: 0 }\nfoo.x = 1\n";
    let mut compiler = Compiler::new(CompileOptions::default());
    let source_file = SourceFile::new(
        SourceId(0),
        PathBuf::from("const_member.tea"),
        source.to_string(),
    );
    let result = compiler.compile(&source_file);
    assert!(
        result.is_err(),
        "expected const member assignment to be rejected"
    );
    let diagnostics = compiler.diagnostics();
    assert!(
        diagnostics
            .entries()
            .iter()
            .any(|diag| diag.message.contains("cannot mutate const 'foo'")),
        "expected const mutation diagnostic, found {:?}",
        diagnostics.entries()
    );
}

#[test]
fn rejects_duplicate_enum_variants() {
    let source = r#"
enum Color {
  Red
  Red
}
"#;
    let mut compiler = Compiler::new(CompileOptions::default());
    let source_file = SourceFile::new(
        SourceId(0),
        PathBuf::from("duplicate_enum.tea"),
        source.to_string(),
    );
    let result = compiler.compile(&source_file);
    assert!(result.is_err(), "expected duplicate enum variants to fail");
    let messages: Vec<_> = compiler
        .diagnostics()
        .entries()
        .iter()
        .map(|d| d.message.as_str())
        .collect();
    assert!(
        messages
            .iter()
            .any(|msg| msg.contains("duplicate variant 'Red' in enum 'Color'")),
        "expected duplicate variant diagnostic, found {:?}",
        messages
    );
}
