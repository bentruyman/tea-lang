# Type System

Goal: make type annotations meaningful and catch obvious mismatches before bytecode generation.

## Scope

- Support primitive types: `Int`, `Bool`, `Float`, `String`, `Nil`, `Void` plus container shells (`List`, `Dict[String, T]`).
- Recognise annotated variables (`var x: Bool = ...`) and verify initialisers.
- Infer types for literals, lists/dicts, unary/binary expressions, simple assignments, and loop/conditional conditions.
- Track globals so later uses of a binding see the most precise type we have inferred.

## Steps

1. **Type Representation**: Introduce a `Type` enum (Bool/Int/Float/String/Nil/Void/List[T]/Dict[T]/Unknown) plus helpers for comparison/pretty-printing.
2. **TypeChecker Pass**:
   - Traverse statements before code generation.
   - Maintain an environment mapping variable names to resolved types.
   - Parse `TypeExpression` tokens into `Type` values; report unknown types or malformed annotations.
   - Infer expression types (literals, unary/binary ops, assignments, list/dict literals, identifiers, call sites).
   - Merge assignment types so rebinding a variable tightens or validates its element types (e.g. `List[Int]`).
   - Verify annotated `var` bindings and push diagnostics on mismatches.
   - Ensure `if`, `unless`, `while`, and `until` conditions are boolean.
3. **Compiler Integration**: Run the type checker after module expansion; abort compilation on diagnostics and surface messages via `Diagnostics`.
4. **Tests & Examples**: Add regression tests covering annotations, container inference/mismatches, and update examples to reflect the type-checked surface area.

## Current Behaviour

- Lists and dictionaries carry element types. Mixed-element literals produce diagnostics (`list element 2: expected Int, found String`).
- Assignments refine existing bindings; incompatible updates (e.g. assigning `List[Bool]` into a `List[Int]`) emit errors instead of silently widening to `Unknown`.
- Indexing honours container element types, so using a non-`Int` list index or reading from a dict yields informative diagnostics.
- Function calls check argument compatibility against declared parameter types, including nested container shapes.
- Function annotations use `Func(args...) -> Return` and are validated when values are assigned or invoked.
- Lambda parameters must include annotations; lambda literals produce concrete `Func(...) -> ...` types so assignments and calls are checked just like named functions.

Follow-up (later): surface structured annotations (`List[Int]`, `Dict[String, T]`), model function/closure types, and propagate precise information across modules.
