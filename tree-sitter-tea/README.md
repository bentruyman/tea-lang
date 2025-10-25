# tree-sitter-tea

Experimental Tree-sitter grammar for the Tea programming language. The goal is to provide fast, incremental parsing so that editors can surface syntax highlighting, folding, and structural navigation for Tea source files.

## Development

Generate parser sources after editing `grammar.js`:

```sh
npm install
npx tree-sitter generate
```

Run the corpus tests to sanity-check the grammar:

```sh
npx tree-sitter test
```

The Rust bindings are published from `bindings/rust`, making the grammar consumable from the Tea language tooling.
