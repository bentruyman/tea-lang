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
        "  if name == \"tea\"",
        "    print(\"brew\")",
        "  else",
        "    print(\"steep\")",
        "  end",
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
