; Keywords
[
  "def"
  "pub"
  "var"
  "const"
  "use"
  "struct"
  "enum"
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

(struct_definition
  name: (identifier) @type)

(enum_definition
  name: (identifier) @type)

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

[
  (string)
  (template_string)
] @string

(number) @number

(boolean) @boolean

(nil) @constant.builtin

(comment) @comment
