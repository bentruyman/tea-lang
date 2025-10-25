# tea-lang Semantics

This document captures the first working cut of tea-lang. It is the contract for the compiler, runtime, and standard library. Revisit and version it as the language evolves. The current prototype only implements the subset called out in **Prototype Scope**; other sections describe the intended design.

## Source Form
- Files use UTF-8 and `.tea` suffix.
- Line breaks terminate statements. A newline is significant unless the parser is inside `()`, `[]`, `{}`, a `| |` lambda header, or the line ends with a trailing operator (`+`, `-`, `*`, `/`, `.`).
- Comments start with `#` and extend to the end of the line.

## Lexical Elements
- Identifiers: `[A-Za-z_][A-Za-z0-9_]*`. Snake_case for variables/functions, PascalCase for types.
- Reserved keywords: `var`, `const`, `def`, `struct`, `if`, `unless`, `else`, `end`, `for`, `of`, `while`, `until`, `return`, `use`, `and`, `or`, `not`, `in`, `nil`.
- Literals: integers (`42`), floats (`3.14`, `1_000.0`), strings (`"Hello"` and `` `Hello, ${name}` ``), booleans (`true`, `false`), lists (`[1, 2, 3]`), dictionaries (`{ "name": "tea" }`), ranges (`0..10`, `0...10`).

## Types
- Scalar: `Bool`, `Int`, `Float`, `String`.
- Compound: `List[T]`, `Dict[K, V]`, `Struct`, `Func`.
- `Nil` represents absence; only `nil` and `false` are falsy.
- Type annotations use postfix colon: `var count: Int = 0`. Omitted annotations trigger inference. Containers use brackets (`List[Int]`, `Dict[String, Int]`) and function types use `Func(Int) -> Int`.
- **Generics:** functions and structs can be parameterised with square-bracket type parameters (e.g. `def identity[T](value: T) -> T`). Call sites may pass explicit type arguments (`identity[Int](42)`), and both the VM and LLVM backends monomorphise the concrete instantiations that the type checker discovers—even when the generic is defined in a separate module that is pulled in via `use`.
- Structs introduce nominal record types:
  ```
  struct User
    name: String
    age: Int
  end
  ```
  Instances require all declared fields and can be created with positional or named arguments (`User("Ada", 37)` or `User(name: "Ada", age: 37)`). Fields are immutable; use helper functions for updates.
- The compiler currently validates annotated variables against `Bool`, `Int`, `Float`, `String`, and `Nil` literals, and infers element types for list/dict literals so mixed containers surface diagnostics.
- Struct definitions: `struct App { server: http.Server }`. Fields are immutable by default; setter functions must handle mutations.

## Expressions
- Everything is an expression; the last expression in a block is the implicit return.
- Supported operators (precedence high → low):
  1. `()` call, `[]` index, `.` member.
  2. unary `+`, `-`, `not`.
  3. `*`, `/`, `%`.
  4. `+`, `-`.
  5. comparisons: `==`, `!=`, `>`, `>=`, `<`, `<=`, `in`.
  6. `and`.
  7. `or`.
- Assignment uses `=`. Desugaring for `+=`, `++`, etc. is a future addition.
- Function calls currently require parentheses: `alias.function(args)` (e.g. `debug.print(message)`). A shorthand without parentheses may be added later.
- List literals evaluate their elements left-to-right (`[expr, ...]`) and produce a boxed list value. Indexing (`list[index]`) evaluates the receiver followed by the index expression; indices must be `Int` at runtime.
- Dictionary literals use braces (`{ key: value, ... }`), where keys may be identifiers or string literals. Member access (`record.field`) reads struct fields or dictionary entries (the property is lowered to a string key). Direct indexing (`dict["field"]`) is also supported. Keys must be strings at runtime.
- Function literals: `|args| => expr` (single-expression) or `|args| => { ... }` (block body). Arguments follow the same annotation/default syntax as `def`.
- Lambda parameters require explicit type annotations so the compiler can check function literals before execution.
- String interpolation uses backtick-delimited strings with `${expr}` placeholders.

## Statements
- `use alias = "module"` loads std modules or relative paths (`use helpers = "./math"`). Dot access resolves exported constants/functions/structs under the chosen alias. The builtin library currently ships:
  - `"std.debug"` — exposes `print` for console output.
  - `"std.assert"` — provides `assert`, `assert_eq`, `assert_ne`, and `fail` helpers for runtime checks.
  - `"std.util"` — type guards (`is_nil`/`is_int`/... ), `len`, `to_string`, and `clamp_int` utilities.
  - `"std.fs"` — file helpers for reading/writing text or bytes, creating/removing directories, querying metadata, and streaming chunked reads via handles.
  - `"std.path"` — join/split helpers, normalization, relative/absolute conversion, and separator queries.
  - `"std.env"` — environment helpers for reading/updating variables, cwd changes, and locating temp/home/config directories.
  - `"std.io"` — pipeline-friendly stdin/stdout helpers (`read_line`, `read_all`, `write`, `flush`, etc.).
  - `"std.json"`, `"std.yaml"` — encode/decode structured data to and from Tea values.
    - When the argument to `decode` is a string literal, the compiler parses it at compile time and infers the resulting list/dict element types so downstream indexing gets stricter checking. Dynamic inputs still fall back to a generic dictionary value.
- Relative modules are expanded in place during compilation, so generic functions/structs defined in `./helpers.tea` can be specialised from the importer just like local definitions. Built-in std modules continue to provide their helpers through the resolver without textual expansion.
- Variable declaration: `var name = expr`. Multiple bindings share one line: `var x = 1, y = 2`.
- Const declaration: `const name = expr`. Every const requires an initializer and may not be reassigned after creation, e.g.
  ```
  const retries = 3
  # retries = 1  # error: cannot reassign const 'retries'
  ```
- Functions:
  ```
  def fib(n: Int) -> Int
    if n <= 1
      return n
    end
    fib(n - 1) + fib(n - 2)
  end
  ```
- Structs:
  ```
  struct User
    name: String
    age: Int
  end
  ```
  Field order matters for positional construction; named arguments (`User(name: "Ada", age: 37)`) are also accepted.
- Conditionals: `if`, `unless`; optional `else`. No implicit `else if`; chain with `elsif`? (future) or nested `if`.
- Loops:
  - `while condition ... end`
  - `until condition ... end`
  - `for item of iterable ... end` *(planned)*
- `return` exits current function; bare `return` returns `nil`.
- `break` and `next` (skip) are future additions; flag TODO.

## Scopes & Variables
- Lexical scoping: new scopes for functions, structs, blocks, loops.
- `var` creates mutable binding in the current scope. `const` creates an immutable binding; any assignment to that name after initialization raises a resolver/type-checker diagnostic.
- Closures capture by reference. Mutation inside closures affects outer binding.
- Shadowing is disallowed; redeclarations across scopes produce resolver diagnostics.
- The resolver rejects duplicate declarations in the same scope and flags attempts to shadow existing bindings or reference undefined names before bytecode is emitted.
- The resolver also reports unused local variables and parameters so you can prune dead bindings early.

## Functions & Modules
- Functions are first-class values. `def` assigns the function to the current scope binding (an implicit `var`). Function parameters must include explicit type annotations (e.g. `def add(a: Int, b: Int)`).
- Default arguments evaluated at call-time.
- Keyword arguments: `def greet(name: String, punctuation: String = "!")` call as `greet(name: "tea")`. Order-insensitive when passed by keyword.
- Variadic functions (future): `def log(*messages)` (not in initial prototype).
- Modules expose names declared at top-level via the alias you choose (`use fs = "std.fs"` then `fs.read_text("path")`). Relative modules continue to inline their declarations at compile time, but future revisions may surface them under aliases as well.

## Runtime & Execution Model
- Programs execute top-to-bottom. Each file compiles to a module containing a top-level block invoked when loaded.
- Running the CLI executes the main file's module block, leaving exported bindings in the module namespace.
- Arithmetic honours numeric types: operations on two `Int`s remain integral, but if either operand is a `Float` the VM promotes the result to `Float`; division or modulo by zero raises a runtime error.
- Standard library provides `print`, math helpers, HTTP stub (future). For prototype, embed minimal host functions (`print`, `len`).

## Errors & Diagnostics
- Compilation errors:
  - Lexical: invalid character, unterminated string.
  - Syntactic: unexpected token, unterminated block.
  - Semantic: undefined variable, type mismatch, invalid call arity.
- Name resolution runs immediately after module expansion and reports undefined bindings, duplicate declarations, and disallowed shadowing so later stages operate on a consistent scope graph.
- Type errors point to involved expressions/identifiers and include expected vs actual types.
- Runtime errors provide stack trace (function names, location).

## Prototype Scope
- Implement: integers, booleans, strings, lists; `if/unless`, `for/while/until`, functions, `return`, lambda literals (`|x| => expr`, `|| => expr`).
- Omit (mark TODO):
  - Dictionaries, modules beyond `use` stub.
  - Keyword args and default params beyond simple expressions.
  - Pattern matching, HTTP runtime.
- Ensure CLI supports `tea file.tea`, `--dump-tokens`, `--dump-ast`, `--emit=ir` (stub).

## Example Execution Flow
1. CLI reads file, forms `SourceFile`.
2. Lexer emits tokens.
3. Parser builds AST honoring newline-delimited statements.
4. Resolver builds symbol table, resolves identifiers and modules.
5. Type checker annotates nodes, accumulates diagnostics.
6. Lowering produces bytecode instructions.
7. VM executes instructions, calling host functions for built-ins.

Document revisions should align with implementation milestones so tooling and docs stay truthful.
