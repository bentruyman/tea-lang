# Examples

Tea programs under this directory double as executable documentation for the
language and its standard library. The layout mirrors the major learning paths:

- `language/` — core language features grouped by topic:
  - `basics/` introduces syntax, recursion, and lightweight tests.
  - `control_flow/` covers loops, conditionals, and other branching tools.
  - `collections/`, `functions/`, `numeric/`, `types/`, and `modules/` demonstrate
    data structures, higher-order helpers, generics, and cross-file imports.
- `stdlib/` — focused tours of runtime helpers (`cli/`, `http/`, `io/`, `testing/`, etc.).

Run any example with the CLI:

```
cargo run -p tea-cli -- examples/language/basics/basics.tea
```

Examples include inline comments that note their expected output. Keep those
up to date when behaviour changes so other contributors can quickly confirm
the scenario still behaves as advertised.
