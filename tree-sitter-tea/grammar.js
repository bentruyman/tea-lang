const PREC = {
  call: 11,
  member: 10,
  unary: 9,
  multiplicative: 8,
  additive: 7,
  comparative: 6,
  equality: 5,
  and: 4,
  or: 3,
  assignment: 2,
  lambda: 1,
};

module.exports = grammar({
  name: "tea",

  extras: $ => [
    /[ \t\r]/,
    $.comment,
  ],

  conflicts: $ => [
    [$.return_statement],
    [$.argument, $.parenthesized_expression],
    [$.braced_block, $.dict_literal],
    [$._expression, $.match_pattern],
    [$.match_case_block, $.match_case_expression],
  ],

  word: $ => $.identifier,

  rules: {
    source_file: $ => repeat($._statement),

    _statement: $ => choice(
      $.use_statement,
      $.const_declaration,
      $.var_declaration,
      $.function_definition,
      $.struct_definition,
      $.enum_definition,
      $.if_statement,
      $.unless_statement,
      $.while_statement,
      $.until_statement,
      $.test_block,
      $.match_statement,
      $.return_statement,
      $.expression_statement,
    ),

    use_statement: $ => seq(
      "use",
      field("alias", $.identifier),
      "=",
      field("module", choice($.string, $.template_string))
    ),

    const_declaration: $ => seq(
      optional("pub"),
      "const",
      field("name", $.identifier),
      optional(seq(":", field("type", $.type_annotation))),
      "=",
      field("value", $._expression),
    ),

    var_declaration: $ => seq(
      "var",
      field("name", $.identifier),
      optional(seq(":", field("type", $.type_annotation))),
      "=",
      field("value", $._expression),
    ),

    function_definition: $ => seq(
      optional("pub"),
      "def",
      field("name", $.identifier),
      optional(field("type_parameters", $.type_parameters)),
      field("parameters", $.parameter_list),
      optional(seq("->", field("return_type", $.type_annotation))),
      field("body", $.block),
      "end"
    ),

    struct_definition: $ => seq(
      optional("pub"),
      "struct",
      field("name", $.identifier),
      optional(field("type_parameters", $.type_parameters)),
      "{",
      field("body", repeat1($.struct_field)),
      "}"
    ),

    struct_field: $ => seq(
      field("name", $.identifier),
      ":",
      field("type", $.type_annotation)
    ),

    enum_definition: $ => seq(
      optional("pub"),
      "enum",
      field("name", $.identifier),
      optional(field("type_parameters", $.type_parameters)),
      "{",
      field("variants", repeat1($.enum_variant)),
      "}"
    ),

    enum_variant: $ => seq(
      field("name", $.identifier)
    ),

    parameter_list: $ => seq(
      "(",
      optional(commaSep1($.parameter)),
      ")"
    ),

    parameter: $ => seq(
      field("name", $.identifier),
      ":",
      field("type", $.type_annotation)
    ),

    type_parameters: $ => seq(
      "[",
      commaSep1($.identifier),
      "]"
    ),

    type_annotation: $ => prec.right(choice(
      $.identifier,
      seq($.identifier, "[", commaSep1($.type_annotation), "]"),
      seq("Func", "(", optional(commaSep1($.type_annotation)), ")", "->", $.type_annotation),
      seq("List", "[", $.type_annotation, "]"),
      seq("Dict", "[", $.type_annotation, ",", $.type_annotation, "]")
    )),

    if_statement: $ => seq(
      "if",
      field("condition", $._expression),
      field("consequence", $.block),
      optional(seq("else", field("alternative", $.block))),
      "end"
    ),

    unless_statement: $ => seq(
      "unless",
      field("condition", $._expression),
      field("body", $.block),
      "end"
    ),

    while_statement: $ => seq(
      "while",
      field("condition", $._expression),
      field("body", $.block),
      "end"
    ),

    until_statement: $ => seq(
      "until",
      field("condition", $._expression),
      field("body", $.block),
      "end"
    ),

    match_statement: $ => seq(
      "match",
      field("value", $._expression),
      repeat1($.match_case_block),
      "end"
    ),

    match_case_block: $ => seq(
      "case",
      field("patterns", $.match_patterns),
      choice(
        seq("=>", field("value", $._expression)),
        field("body", $.block)
      )
    ),

    test_block: $ => seq(
      "test",
      field("name", $.string),
      field("body", $.block),
      "end"
    ),

    block: $ => repeat1($._statement),

    braced_block: $ => seq(
      "{",
      repeat($._statement),
      "}"
    ),

    return_statement: $ => choice(
      seq("return", field("value", $._expression)),
      "return"
    ),

    expression_statement: $ => $._expression,

    _expression: $ => choice(
      $.assignment,
      $.binary_expression,
      $.lambda_expression,
      $.match_expression,
      $.call_expression,
      $.member_expression,
      $.index_expression,
      $.list_literal,
      $.dict_literal,
      $.template_string,
      $.string,
      $.number,
      $.boolean,
      $.nil,
      $.identifier,
      $.parenthesized_expression,
    ),

    assignment: $ => prec.right(PREC.assignment, seq(
      field("left", choice($.identifier, $.member_expression, $.index_expression)),
      "=",
      field("right", $._expression),
    )),

    match_expression: $ => seq(
      "match",
      field("value", $._expression),
      repeat1($.match_case_expression),
      "end"
    ),

    match_case_expression: $ => seq(
      "case",
      field("patterns", $.match_patterns),
      "=>",
      field("value", $._expression)
    ),

    binary_expression: $ => choice(
      ...[
        ["||", PREC.or],
        ["&&", PREC.and],
        ["==", PREC.equality],
        ["!=", PREC.equality],
        ["<", PREC.comparative],
        ["<=", PREC.comparative],
        [">", PREC.comparative],
        [">=", PREC.comparative],
        ["+", PREC.additive],
        ["-", PREC.additive],
        ["*", PREC.multiplicative],
        ["/", PREC.multiplicative],
        ["%", PREC.multiplicative],
      ].map(([operator, precedence]) =>
        prec.left(precedence, seq(
          field("left", $._expression),
          field("operator", operator),
          field("right", $._expression)
        ))
      )
    ),

    unary_expression: $ => prec.left(PREC.unary, seq(
      field("operator", choice("-", "not")),
      field("argument", $._expression)
    )),

    call_expression: $ => prec(PREC.call, seq(
      field("function", $._expression),
      field("arguments", $.argument_list)
    )),

    argument_list: $ => seq(
      "(",
      optional(commaSep1($.argument)),
      ")"
    ),

    argument: $ => choice(
      $._expression,
      $.named_argument
    ),

    named_argument: $ => seq(
      field("name", $.identifier),
      ":",
      field("value", $._expression)
    ),

    member_expression: $ => prec.left(PREC.member, seq(
      field("object", $._expression),
      ".",
      field("property", $.identifier)
    )),

    index_expression: $ => prec.left(PREC.member, seq(
      field("collection", $._expression),
      "[",
      field("index", $._expression),
      "]"
    )),

    lambda_expression: $ => prec.right(PREC.lambda, seq(
      field("parameters", choice(
        seq("|", optional(commaSep1($.lambda_parameter)), "|"),
        "||"
      )),
      "=>",
      field("body", choice($.braced_block, $._expression))
    )),

    lambda_parameter: $ => seq(
      field("name", $.identifier),
      optional(seq(":", field("type", $.type_annotation)))
    ),

    list_literal: $ => seq(
      "[",
      optional(commaSep1($._expression)),
      "]"
    ),

    dict_literal: $ => seq(
      "{",
      optional(commaSep1($.dict_entry)),
      "}"
    ),

    dict_entry: $ => seq(
      field("key", choice($.identifier, $.string, $.template_string)),
      ":",
      field("value", $._expression)
    ),

    match_patterns: $ => prec.left(seq(
      field("pattern", $.match_pattern),
      repeat(seq("|", field("pattern", $.match_pattern)))
    )),

    match_pattern: $ => choice(
      "_",
      $.identifier,
      $.string,
      $.number,
      $.boolean,
      $.member_expression
    ),

    parenthesized_expression: $ => seq("(", $._expression, ")"),

    template_string: $ => seq(
      "`",
      repeat(choice($.template_interpolation, $.string_fragment)),
      "`"
    ),

    template_interpolation: $ => seq(
      "${",
      field("expression", $._expression),
      "}"
    ),

    string_fragment: $ => token.immediate(/([^`$\\]|\\.)+/),

    identifier: $ => /[A-Za-z_][A-Za-z0-9_]*/,

    number: $ => token(choice(
      /\d+(_\d+)*(\.\d+(_\d+)*)?/,
      /\.\d+(_\d+)*/
    )),

    string: $ => token(seq(
      '"',
      repeat(choice(/[^"\\]/, /\\./)),
      '"'
    )),

    boolean: $ => choice("true", "false"),

    nil: $ => "nil",

    comment: $ => token(seq("#", /.*/)),
  },
});

function commaSep1(rule) {
  return seq(rule, repeat(seq(",", rule)), optional(","));
}
