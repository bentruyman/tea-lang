; Keywords
[
  "def"
  "pub"
  "var"
  "const"
  "use"
  "struct"
  "enum"
  "error"
  "throw"
  "try"
  "catch"
  "case"
  "if"
  "else"
  "unless"
  "while"
  "until"
  "return"
  "test"
] @keyword

(function_definition
  name: (identifier) @function)

(function_definition
  return_type: (_) @type)

(error_type
  name: (identifier) @type)

(error_type
  variant: (identifier) @constructor)

(struct_definition
  name: (identifier) @type)

(enum_definition
  name: (identifier) @type)

(error_definition
  name: (identifier) @type)

(error_variant
  name: (identifier) @constructor)

(error_field
  name: (identifier) @property)

(parameter
  name: (identifier) @variable.parameter)

(parameter
  type: (_) @type)

(type_annotation
  (identifier) @type)

(call_expression
  function: (identifier) @function.call)

(member_expression
  property: (identifier) @property)

(var_declaration
  name: (identifier) @variable)

(const_declaration
  name: (identifier) @constant)

(use_statement
  alias: (identifier) @namespace)

(catch_clause
  binding: (identifier) @variable)

[
  (string)
  (template_string)
] @string

(number) @number

(boolean) @boolean

(nil) @constant.builtin

(comment) @comment
