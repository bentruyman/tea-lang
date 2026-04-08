# echo

A simple implementation of the Unix `echo` command in Tea.

## Description

Prints its arguments to standard output, separated by spaces. Optionally omits the trailing newline with the `-n` flag.

## Usage

```bash
tea examples/echo/main.tea [OPTIONS] [ARGUMENTS...]
```

### Options

- `-n` — Do not output the trailing newline

## Examples

```bash
# Print a simple message
tea examples/echo/main.tea hello world
# Output: hello world

# Print without trailing newline
tea examples/echo/main.tea -n no newline here

# Print nothing (just a newline)
tea examples/echo/main.tea
# Output: (empty line)

# Print special characters
tea examples/echo/main.tea "hello   world"
# Output: hello   world
```

## Tea Features Demonstrated

- **`@args()`** — Built-in function to access command-line arguments
- **`@len()`** — Getting collection length
- **`@print()` / `@println()`** — Output functions
- **`@exit()`** — Program termination with exit code
- **List operations** — Indexing, concatenation, iteration
- **String concatenation** — Using the `+` operator
- **Control flow** — `if/else/end`, `while/end`

## Build

Compile to a native binary:

```bash
tea build examples/echo/main.tea -o echo
./echo hello world
```
