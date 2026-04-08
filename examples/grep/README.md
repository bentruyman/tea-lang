# grep

A simplified implementation of the Unix `grep` command in Tea.

## Description

Searches for a regular expression pattern in one or more files and prints matching lines. Supports line numbers, case-insensitive matching, and filename prefixes.

## Usage

```bash
tea examples/grep/main.tea [OPTIONS] PATTERN FILE...
```

### Options

- `-n`, `--line-number` — Print line numbers with output lines
- `-i`, `--ignore-case` — Ignore case distinctions in patterns and data
- `-H`, `--with-filename` — Print filename for each match (default when multiple files)
- `-h`, `--help` — Display help message

### Exit Status

- `0` — One or more matches found
- `1` — No matches found
- `2` — Error (missing arguments, file not found, etc.)

## Examples

```bash
# Search for "TODO" in a file
tea examples/grep/main.tea "TODO" src/main.tea

# Case-insensitive search with line numbers
tea examples/grep/main.tea -n -i "error" logs.txt

# Search multiple files (filename shown automatically)
tea examples/grep/main.tea "def " lib/*.tea

# Use regex patterns
tea examples/grep/main.tea "fn.*->.*String" src/*.tea

# Search for email addresses
tea examples/grep/main.tea "[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}" contacts.txt
```

## Tea Features Demonstrated

- **`std.args`** — Command-line argument parsing with flags and positional args
- **`std.fs`** — Reading file contents
- **`std.regex`** — Pattern compilation and matching
- **String interpolation** — Using backtick templates: `` `${var}` ``
- **Error handling** — Exit codes for different states
- **Functions** — Multiple helper functions with type annotations
- **Control flow** — `if/else`, `while`, `for` loops

## Build

Compile to a native binary:

```bash
tea build examples/grep/main.tea -o grep
./grep -n "pattern" file.txt
```

## Differences from GNU grep

This is a simplified implementation. Notable differences:

- No recursive directory search (`-r`)
- No inverted matching (`-v`)
- No context lines (`-A`, `-B`, `-C`)
- No count mode (`-c`)
- No quiet mode (`-q`)
- Pattern must be a valid Rust regex (uses PCRE-like syntax)
