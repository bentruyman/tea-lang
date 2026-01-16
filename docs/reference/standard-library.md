# Standard Library

Tea's standard library provides modules for common tasks: filesystem operations, environment variables, path manipulation, string utilities, JSON handling, and testing assertions.

## Importing Modules

Import stdlib modules using the `use` keyword:

```tea
use fs = "std.fs"
use path = "std.path"
use env = "std.env"
```

The syntax is: `use alias = "module.path"`

## Quick Reference

| Module       | Purpose                                            |
| ------------ | -------------------------------------------------- |
| `std.assert` | Test assertions and runtime checks                 |
| `std.env`    | Environment variables and working directory        |
| `std.fs`     | Filesystem operations (read, write, list)          |
| `std.json`   | JSON encoding and decoding                         |
| `std.path`   | Path manipulation utilities                        |
| `std.string` | String operations (trim, replace, case conversion) |

---

## std.assert

Assertions for testing and runtime validation.

```tea
use assert = "std.assert"
```

### `ok(value: Bool) -> Void`

Assert that a value is true.

```tea
assert.ok(1 + 1 == 2)
assert.ok(x > 0)
```

### `eq(left: T, right: T) -> Void`

Assert that two values are equal.

```tea
assert.eq(1 + 1, 2)
assert.eq("hello", "hello")
assert.eq([1, 2, 3], [1, 2, 3])
```

### `ne(left: T, right: T) -> Void`

Assert that two values are not equal.

```tea
assert.ne(1, 2)
assert.ne("hello", "world")
```

### `gt(left: T, right: T) -> Void`

Assert that left is greater than right.

```tea
assert.gt(10, 5)
assert.gt(3.5, 2.1)
```

### `lt(left: T, right: T) -> Void`

Assert that left is less than right.

```tea
assert.lt(5, 10)
assert.lt(2.1, 3.5)
```

### `snapshot(name: String, value: Any, path: String?) -> Void`

Assert that a value matches a saved snapshot for testing.

```tea
test "output format"
  var result = format_data(input)
  assert.snapshot("format_output", result)
end
```

Snapshots are saved in `__snapshots__/` by default.

**Example Test:**

```tea
use assert = "std.assert"

test "calculator operations"
  assert.eq(add(2, 3), 5)
  assert.ne(multiply(2, 3), 5)
  assert.gt(10, 5)
  assert.lt(5, 10)
end
```

---

## std.env

Environment variable access and working directory management.

```tea
use env = "std.env"
```

### `get(name: String) -> String`

Get an environment variable value.

```tea
var home = env.get("HOME")
var path = env.get("PATH")
```

### `set(name: String, value: String) -> Void`

Set an environment variable for the current process.

```tea
env.set("MY_VAR", "my_value")
env.set("DEBUG", "true")
```

### `cwd() -> String`

Get the current working directory.

```tea
var current_dir = env.cwd()
@println(`Working in: ${current_dir}`)
```

### `vars() -> Dict[String, String]`

Get all environment variables as a dictionary.

```tea
var all_vars = env.vars()
# Iterate over environment variables
```

**Example Usage:**

```tea
use env = "std.env"
use path = "std.path"

var root = env.cwd()
var config_path = path.join([root, "config.json"])

if env.get("DEBUG") == "true"
  @println(`Loading config from: ${config_path}`)
end
```

---

## std.fs

Filesystem operations for reading, writing, and managing files and directories.

```tea
use fs = "std.fs"
```

### `read_file(path: String) -> String`

Read the entire contents of a text file.

```tea
var content = fs.read_file("config.txt")
var script = fs.read_file("script.tea")
```

### `write_file(path: String, contents: String) -> Void`

Write a string to a file, replacing existing contents.

```tea
fs.write_file("output.txt", "Hello, Tea!")
fs.write_file("results.json", json_data)
```

### `read_dir(path: String) -> List[String]`

List all entries in a directory (files and subdirectories).

```tea
var entries = fs.read_dir(".")
for entry in entries
  @println(entry)
end
```

Returns only the names, not full paths.

### `create_dir(path: String) -> Void`

Create a directory. Parent directories must already exist.

```tea
fs.create_dir("output")
fs.create_dir("build/artifacts")  # Parent "build" must exist
```

### `remove(path: String) -> Void`

Remove a file or directory recursively.

```tea
fs.remove("temp.txt")
fs.remove("old_folder")  # Removes directory and all contents
```

### `rename(source: String, target: String) -> Void`

Rename or move a file or directory.

```tea
fs.rename("old.txt", "new.txt")
fs.rename("file.txt", "backup/file.txt")
```

**Example: Process Text Files:**

```tea
use fs = "std.fs"
use string = "std.string"
use path = "std.path"

var files = fs.read_dir("docs")

for file in files
  if string.ends_with(file, ".txt")
    var content = fs.read_file(path.join(["docs", file]))
    var lines = @len(string.split(content, "\n"))
    @println(`${file}: ${lines} lines`)
  end
end
```

---

## std.json

JSON encoding and decoding utilities.

```tea
use json = "std.json"
```

### `encode(value: Dict) -> String`

Encode a value to a JSON string.

```tea
var data = { "name": "tea", "version": "1.0", "stable": "true" }
var json_str = json.encode(data)
@println(json_str)  # {"name":"tea","version":"1.0","stable":"true"}
```

### `decode(json_str: String) -> Dict`

Decode a JSON string to a value.

```tea
var json_str = "{\"name\":\"tea\",\"version\":\"1.0\"}"
var data = json.decode(json_str)
@println(data.name)  # tea
```

**Example: Config File:**

```tea
use fs = "std.fs"
use json = "std.json"

# Read config
var config_json = fs.read_file("config.json")
var config = json.decode(config_json)

@println(`Server: ${config.host}:${config.port}`)

# Write config
var new_config = { "host": "localhost", "port": "8080" }
fs.write_file("config.json", json.encode(new_config))
```

---

## std.path

Path manipulation utilities for working with file paths.

```tea
use path = "std.path"
```

### `join(parts: List[String]) -> String`

Join path components using the system path separator.

```tea
var file_path = path.join(["usr", "local", "bin", "tea"])
# Unix: "usr/local/bin/tea"
# Windows: "usr\local\bin\tea"
```

### `split(file_path: String) -> List[String]`

Split a path into its components.

```tea
var parts = path.split("/usr/local/bin")
# Result: ["usr", "local", "bin"]
```

### `dirname(file_path: String) -> String`

Get the directory portion of a path.

```tea
path.dirname("/usr/local/bin/tea")  # "/usr/local/bin"
path.dirname("file.txt")            # ""
path.dirname("/")                   # "/"
```

### `basename(file_path: String) -> String`

Get the filename portion of a path.

```tea
path.basename("/usr/local/bin/tea")  # "tea"
path.basename("file.txt")            # "file.txt"
path.basename("/usr/local/")         # "local"
```

### `extension(file_path: String) -> String`

Get the file extension (without the dot).

```tea
path.extension("file.tea")     # "tea"
path.extension("file.tar.gz")  # "gz"
path.extension(".gitignore")   # ""
path.extension("file")         # ""
```

**Example: Build Output Path:**

```tea
use path = "std.path"
use env = "std.env"

var source_file = "src/main.tea"
var filename = path.basename(source_file)  # "main.tea"
var name_only = path.split(filename)[0]    # "main"

var output_dir = path.join([env.cwd(), "build"])
var output_file = path.join([output_dir, name_only])

@println(`Output: ${output_file}`)  # "path/to/project/build/main"
```

---

## std.string

String manipulation utilities for common text operations.

```tea
use string = "std.string"
```

### `starts_with(text: String, prefix: String) -> Bool`

Check if a string starts with a prefix.

```tea
string.starts_with("hello world", "hello")  # true
string.starts_with("hello world", "world")  # false
```

### `ends_with(text: String, suffix: String) -> Bool`

Check if a string ends with a suffix.

```tea
string.ends_with("hello world", "world")  # true
string.ends_with("hello world", "hello")  # false
```

### `replace(text: String, pattern: String, replacement: String) -> String`

Replace all occurrences of a pattern.

```tea
string.replace("hello world", "world", "Tea")  # "hello Tea"
string.replace("aaa", "a", "b")                # "bbb"
```

### `to_lower(text: String) -> String`

Convert a string to lowercase.

```tea
string.to_lower("HELLO")       # "hello"
string.to_lower("Hello World") # "hello world"
```

### `to_upper(text: String) -> String`

Convert a string to uppercase.

```tea
string.to_upper("hello")       # "HELLO"
string.to_upper("Hello World") # "HELLO WORLD"
```

### `trim(text: String) -> String`

Trim whitespace from both ends.

```tea
string.trim("  hello  ")     # "hello"
string.trim("\t\nhello\n\t") # "hello"
```

### `trim_start(text: String) -> String`

Trim whitespace from the start.

```tea
string.trim_start("  hello  ")  # "hello  "
```

### `trim_end(text: String) -> String`

Trim whitespace from the end.

```tea
string.trim_end("  hello  ")  # "  hello"
```

### `reverse(text: String) -> String`

Reverse a string.

```tea
string.reverse("hello")  # "olleh"
string.reverse("Tea")    # "aeT"
```

**Example: File Filtering:**

```tea
use fs = "std.fs"
use string = "std.string"

var files = fs.read_dir(".")

for file in files
  if string.ends_with(file, ".tea")
    var content = fs.read_file(file)
    var lines = string.trim(content)
    @println(`${file}: ${@len(lines)} chars`)
  end
end
```

---

## Practical Examples

### Configuration Manager

```tea
use fs = "std.fs"
use json = "std.json"
use path = "std.path"
use env = "std.env"

def load_config() -> Dict
  var config_path = path.join([env.cwd(), "config.json"])

  if !fs.exists(config_path)
    # Create default config
    var default_config = {
      "host": "localhost",
      "port": "8080",
      "debug": "false"
    }
    fs.write_file(config_path, json.encode(default_config))
    return default_config
  end

  var config_json = fs.read_file(config_path)
  json.decode(config_json)
end

var config = load_config()
@println(`Server: ${config.host}:${config.port}`)
```

### File Processor

```tea
use fs = "std.fs"
use path = "std.path"
use string = "std.string"

def process_directory(dir: String)
  var entries = fs.read_dir(dir)

  for entry in entries
    var full_path = path.join([dir, entry])

    if string.ends_with(entry, ".tea")
      var content = fs.read_file(full_path)
      var trimmed = string.trim(content)

      # Save processed file
      var output_name = string.replace(entry, ".tea", ".processed.tea")
      var output_path = path.join([dir, output_name])
      fs.write_file(output_path, trimmed)

      @println(`Processed: ${entry}`)
    end
  end
end

process_directory("src")
```

### Testing Utilities

```tea
use assert = "std.assert"
use string = "std.string"

test "string operations"
  var text = "  Hello, Tea!  "
  var trimmed = string.trim(text)

  assert.eq(trimmed, "Hello, Tea!")
  assert.ok(string.starts_with(trimmed, "Hello"))
  assert.ok(string.ends_with(trimmed, "!"))
end

test "path operations"
  var p = path.join(["usr", "local", "bin"])
  assert.ok(string.ends_with(p, "bin"))

  var ext = path.extension("file.tea")
  assert.eq(ext, "tea")
end
```

## See Also

- **[Built-in Functions](builtins.md)** - Global functions like `@println`, `@len`, etc.
- **[Language Guide](../guide/advanced.md)** - Learn about modules and imports
- **[Examples](../examples.md)** - More practical code examples
