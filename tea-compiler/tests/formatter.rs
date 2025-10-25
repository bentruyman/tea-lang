use tea_compiler::format_source;

fn assert_lines(actual: &str, expected: &[&str]) {
    let actual_lines: Vec<&str> = actual.lines().collect();
    assert_eq!(expected, actual_lines.as_slice());
}

#[test]
fn formats_blocks_and_brackets() {
    let input = r#"
def greet(name: String)
print("hi")
if name == "tea"
print("brew")
else
print("steep")
end
var values = [
1,
2,
]
end
"#;

    let expected = [
        "def greet(name: String)",
        "  print(\"hi\")",
        "",
        "  if name == \"tea\"",
        "    print(\"brew\")",
        "  else",
        "    print(\"steep\")",
        "  end",
        "",
        "  var values = [",
        "    1,",
        "    2,",
        "  ]",
        "end",
    ];

    let output = format_source(input);
    println!("expected_data: {:?}", expected);
    println!("actual_data: {:?}", output.lines().collect::<Vec<_>>());
    assert_lines(&output, &expected);
}

#[test]
fn formats_nested_dicts_and_lists() {
    let input = r#"
var config = {
name: "tea",
values: [
1,
{ nested: true },
],
metadata: {
owners: [
"core",
"contributors",
],
},
}
"#;

    let expected = [
        "var config = {",
        "  name: \"tea\",",
        "  values: [",
        "    1,",
        "    { nested: true },",
        "  ],",
        "  metadata: {",
        "    owners: [",
        "      \"core\",",
        "      \"contributors\",",
        "    ],",
        "  },",
        "}",
    ];

    assert_lines(&format_source(input), &expected);
}

#[test]
fn indents_assignment_continuations_and_hanging_operators() {
    let input = r#"
var data =
[
1,
2,
]

var total =
first
+ second
- third
"#;

    let output = format_source(input);
    let lines: Vec<&str> = output.lines().collect();
    assert_eq!(lines[0], "var data =");
    assert_eq!(lines[1], "  [");
    assert_eq!(lines[2], "  1,");
    assert_eq!(lines[3], "  2,");
    assert_eq!(lines[4], "]");
    assert_eq!(lines[5], "");
    assert_eq!(lines[6], "var total =");
    assert_eq!(lines[7], "  first");
    assert_eq!(lines[8], "  + second");
    assert_eq!(lines[9], "  - third");
}

#[test]
fn preserves_comments_and_blank_lines() {
    let input = r#"
# banner
def run()
var result = 42  # TODO tighten

# trailing comment describing return
return result
end
"#;

    let expected = [
        "# banner",
        "def run()",
        "  var result = 42  # TODO tighten",
        "",
        "  # trailing comment describing return",
        "  return result",
        "end",
    ];

    assert_lines(&format_source(input), &expected);
}

#[test]
fn aligns_closing_brackets_with_inline_comments() {
    let input = r#"
var wrapper = [
{
name: "tea",
}, # trailing struct
]
"#;

    let expected = [
        "var wrapper = [",
        "  {",
        "    name: \"tea\",",
        "  }, # trailing struct",
        "]",
    ];

    assert_lines(&format_source(input), &expected);
}

#[test]
fn aligns_sequential_closing_brackets() {
    let input = r#"
var cubes = [
[
1,
2,
],
[
3,
4,
],
]
"#;

    let output = format_source(input);
    let lines: Vec<&str> = output.lines().collect();
    assert_eq!(lines[0], "var cubes = [");
    assert_eq!(lines[1], "  [");
    assert_eq!(lines[4], "  ],");
    assert_eq!(lines[5], "  [");
    assert_eq!(lines[8], "  ],");
    assert_eq!(lines[9], "]");
}

#[test]
fn removes_all_spaces_before_parameter_list() {
    let input = r#"
def print_user          (user: User) -> Nil
print(user.name)
end
"#;

    let expected = [
        "def print_user(user: User) -> Nil",
        "  print(user.name)",
        "end",
    ];

    assert_lines(&format_source(input), &expected);
}

#[test]
fn removes_single_space_before_parameter_list() {
    let input = r#"
def run (count: Int) -> Int
return count
end
"#;

    let expected = ["def run(count: Int) -> Int", "  return count", "end"];

    assert_lines(&format_source(input), &expected);
}

#[test]
fn tightens_call_whitespace() {
    let input = r#"
def run()
debug.print( fib(30) )
debug .print(fib(30))
fib(n-2)
const multiplier=3
debug.print (ANSWER)
end
"#;

    let expected = [
        "def run()",
        "  debug.print(fib(30))",
        "  debug.print(fib(30))",
        "  fib(n - 2)",
        "  const multiplier = 3",
        "  debug.print(ANSWER)",
        "end",
    ];

    assert_lines(&format_source(input), &expected);
}

#[test]
fn spaces_dict_entries() {
    let input = r#"
var scores = {"alice":10,"bob":8}
"#;

    let expected = ["var scores = { \"alice\": 10, \"bob\": 8 }"];

    assert_lines(&format_source(input), &expected);
}

#[test]
fn space_before_dict_literal_assignment() {
    let input = r#"
var point ={ x : 3, y : 4 }
"#;

    let expected = ["var point = { x: 3, y: 4 }"];

    assert_lines(&format_source(input), &expected);
}

#[test]
fn tightens_inline_lists() {
    let input = r#"
var nested = [ [1], [2] , [ 3 ] ]
"#;

    let expected = ["var nested = [[1], [2], [3]]"];

    assert_lines(&format_source(input), &expected);
}

#[test]
fn space_before_list_literal_assignment() {
    let input = r#"
var words =["alpha", "beta", "gamma"]
"#;

    let expected = ["var words = [\"alpha\", \"beta\", \"gamma\"]"];

    assert_lines(&format_source(input), &expected);
}

#[test]
fn tightens_index_spacing() {
    let input = r#"
var value = numbers[ 0 ]
"#;

    let expected = ["var value = numbers[0]"];

    assert_lines(&format_source(input), &expected);
}

#[test]
fn spaces_arrow_in_signatures() {
    let input = r#"
def scale(value: Int)->Int
value * multiplier
end
"#;

    let expected = [
        "def scale(value: Int) -> Int",
        "  value * multiplier",
        "end",
    ];

    assert_lines(&format_source(input), &expected);
}

#[test]
fn pads_functions_with_blank_lines() {
    let input = r#"
debug.print("Start")
def func()
debug.print("BODY")
end
debug.print("End")
"#;

    let expected = [
        "debug.print(\"Start\")",
        "",
        "def func()",
        "  debug.print(\"BODY\")",
        "end",
        "",
        "debug.print(\"End\")",
    ];

    assert_lines(&format_source(input), &expected);
}

#[test]
fn collapses_if_keyword_spacing() {
    let input = r#"
if     true
debug.print("ok")
end
"#;

    let expected = ["if true", "  debug.print(\"ok\")", "end"];

    assert_lines(&format_source(input), &expected);
}

#[test]
fn normalizes_lambda_bars() {
    let input = r#"
var mapper = | value: Int | => base + value
"#;

    let expected = ["var mapper = |value: Int| => base + value"];

    assert_lines(&format_source(input), &expected);
}

#[test]
fn indents_test_blocks() {
    let input = r#"
test "fib"
assert.assert_eq(fib_module.fib(0), 0)
assert.assert_eq(fib_module.fib(1), 1)
end
"#;

    let expected = [
        "test \"fib\"",
        "  assert.assert_eq(fib_module.fib(0), 0)",
        "  assert.assert_eq(fib_module.fib(1), 1)",
        "end",
    ];

    assert_lines(&format_source(input), &expected);
}

#[test]
fn enforces_generic_bracket_spacing() {
    let input = r#"
struct Box [ T ] {
value: T
}

def identity [ T ] (value: T) -> T
value
end

var int_box = make_box [ Int ] (identity [ Int ] (42))
"#;

    let expected = [
        "struct Box[T] {",
        "  value: T",
        "}",
        "",
        "def identity[T](value: T) -> T",
        "  value",
        "end",
        "",
        "var int_box = make_box[Int](identity[Int](42))",
    ];

    assert_lines(&format_source(input), &expected);
}

#[test]
fn pads_conditionals_with_blank_lines() {
    let input = r#"
debug.print("Start")
if ready
debug.print("BODY")
end
debug.print("End")
"#;

    let expected = [
        "debug.print(\"Start\")",
        "",
        "if ready",
        "  debug.print(\"BODY\")",
        "end",
        "",
        "debug.print(\"End\")",
    ];

    assert_lines(&format_source(input), &expected);
}

#[test]
fn keeps_else_within_conditional_block() {
    let input = r#"
if ready
debug.print("one")
else
debug.print("two")
end
debug.print("done")
"#;

    let expected = [
        "if ready",
        "  debug.print(\"one\")",
        "else",
        "  debug.print(\"two\")",
        "end",
        "",
        "debug.print(\"done\")",
    ];

    assert_lines(&format_source(input), &expected);
}

#[test]
fn no_padding_when_conditional_is_block_edge() {
    let input = r#"
def demo()
if ready
debug.print("one")
end
end
"#;

    let expected = [
        "def demo()",
        "  if ready",
        "    debug.print(\"one\")",
        "  end",
        "end",
    ];

    assert_lines(&format_source(input), &expected);
}

#[test]
fn indents_public_function_body() {
    let input = r#"
pub def hello(name: String) -> String
`Hello ${name}!`
end
"#;

    let expected = [
        "pub def hello(name: String) -> String",
        "  `Hello ${name}!`",
        "end",
    ];

    assert_lines(&format_source(input), &expected);
}

#[test]
fn indents_enum_variants() {
    let input = r#"
enum Status
Pending
Running
Done
end
"#;

    let expected = ["enum Status", "  Pending", "  Running", "  Done", "end"];

    assert_lines(&format_source(input), &expected);
}

#[test]
fn indents_match_cases() {
    let input = r#"
var output = match color
case Color.Red => "red"
case Color.Green => "green"
case Color.Blue => "blue"
end
"#;

    let expected = [
        "var output = match color",
        "  case Color.Red => \"red\"",
        "  case Color.Green => \"green\"",
        "  case Color.Blue => \"blue\"",
        "end",
    ];

    assert_lines(&format_source(input), &expected);
}

#[test]
fn enforces_comparison_spacing() {
    let input = r#"
while count<max
if count>=1 and count<=10
unless value!=expected
if name=="tea"
debug.print(count)
end
end
end
end
"#;

    let expected = [
        "while count < max",
        "  if count >= 1 and count <= 10",
        "    unless value != expected",
        "      if name == \"tea\"",
        "        debug.print(count)",
        "      end",
        "    end",
        "  end",
        "end",
    ];

    assert_lines(&format_source(input), &expected);
}

#[test]
fn normalizes_postfix_coalescing() {
    let input = r#"
debug.print(maybe_name !)
var value = optional  !
"#;

    let expected = ["debug.print(maybe_name!)", "var value = optional!"];

    assert_lines(&format_source(input), &expected);
}

#[test]
fn pads_top_level_conditionals() {
    let input = r#"
var count = 0
unless count == 1
count = 1
end
debug.print(count)
"#;

    let expected = [
        "var count = 0",
        "",
        "unless count == 1",
        "  count = 1",
        "end",
        "",
        "debug.print(count)",
    ];

    assert_lines(&format_source(input), &expected);
}
