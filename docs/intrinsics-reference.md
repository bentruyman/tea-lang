# Tea Intrinsics Reference

This document provides a comprehensive reference for all intrinsic functions in Tea. Intrinsics are low-level built-in functions that provide core functionality for the language runtime. They are typically accessed through the `std.intrinsics` module or used internally by the standard library.

**Note:** Most users should use the higher-level standard library modules (`std.env`, `std.fs`, `std.path`, etc.) instead of calling intrinsics directly. The standard library provides cleaner APIs and better error handling.

## Table of Contents

- [Conversion](#conversion)
- [String Utilities](#string-utilities)
- [Assertions](#assertions)
- [Environment](#environment)
- [Filesystem](#filesystem)
- [Path](#path)

---

## Conversion

### `to_string(value: Any) -> String`

Convert any value to its string representation.

**Usage:**

```tea
use intrinsics = "std.intrinsics"

var num = 42
var str = intrinsics.to_string(num)  # => "42"
```

**Note:** This is primarily used internally for string interpolation.

---

## String Utilities

### `string_index_of(text: String, search: String) -> Int`

Find the first occurrence of a substring. Returns the index (0-based) or -1 if not found.

**Examples:**

```tea
use intrinsics = "std.intrinsics"

intrinsics.string_index_of("hello world", "world")  # => 6
intrinsics.string_index_of("hello", "x")            # => -1
```

### `string_split(text: String, separator: String) -> List[String]`

Split a string by a separator into a list of substrings.

**Examples:**

```tea
use intrinsics = "std.intrinsics"

intrinsics.string_split("a,b,c", ",")    # => ["a", "b", "c"]
intrinsics.string_split("hello", "l")    # => ["he", "", "o"]
```

### `string_contains(text: String, search: String) -> Bool`

Check if a string contains a substring.

**Examples:**

```tea
use intrinsics = "std.intrinsics"

intrinsics.string_contains("hello world", "world")  # => true
intrinsics.string_contains("hello", "x")            # => false
```

### `string_replace(text: String, search: String, replacement: String) -> String`

Replace all occurrences of a substring with another string.

**Examples:**

```tea
use intrinsics = "std.intrinsics"

intrinsics.string_replace("hello world", "world", "Tea")  # => "hello Tea"
intrinsics.string_replace("aaa", "a", "b")                # => "bbb"
```

---

## Assertions

### `__intrinsic_fail(message: String) -> Never`

Immediately fail with an error message. This function never returns.

**Usage:**

```tea
# Typically used through assert.fail()
__intrinsic_fail("Something went wrong")
```

### `__intrinsic_assert_snapshot(name: String, value: Any, path: String) -> Void`

Assert that a value matches a saved snapshot for testing.

**Parameters:**

- `name`: The name of the snapshot
- `value`: The value to compare
- `path`: Directory path for snapshots (typically `__snapshots__`)

**Usage:**

```tea
# Typically used through assert.snapshot()
__intrinsic_assert_snapshot("test_name", result, "__snapshots__")
```

---

## Environment

### `env_get(name: String) -> String`

Get the value of an environment variable.

**Examples:**

```tea
use intrinsics = "std.intrinsics"

var path = intrinsics.env_get("PATH")
```

**Note:** Prefer using `std.env.get()` which provides better error handling.

### `env_set(name: String, value: String) -> Void`

Set an environment variable.

**Examples:**

```tea
use intrinsics = "std.intrinsics"

intrinsics.env_set("MY_VAR", "my_value")
```

### `env_unset(name: String) -> Void`

Remove an environment variable.

**Examples:**

```tea
use intrinsics = "std.intrinsics"

intrinsics.env_unset("MY_VAR")
```

### `env_has(name: String) -> Bool`

Check if an environment variable exists.

**Examples:**

```tea
use intrinsics = "std.intrinsics"

if intrinsics.env_has("HOME")
  print("HOME is set")
end
```

### `env_vars() -> Dict[String, String]`

Get all environment variables as a dictionary.

**Examples:**

```tea
use intrinsics = "std.intrinsics"

var all_vars = intrinsics.env_vars()
for key, value in all_vars
  print(`{key} = {value}`)
end
```

### `env_cwd() -> String`

Get the current working directory.

**Examples:**

```tea
use intrinsics = "std.intrinsics"

var cwd = intrinsics.env_cwd()
print(`Current directory: {cwd}`)
```

---

## Filesystem

### `fs_read_text(file_path: String) -> String`

Read the contents of a text file.

**Examples:**

```tea
use intrinsics = "std.intrinsics"

var content = intrinsics.fs_read_text("config.txt")
```

**Note:** Prefer using `std.fs.read_text()` which provides better error messages.

### `fs_write_text(file_path: String, content: String) -> Void`

Write text content to a file, creating or overwriting it.

**Examples:**

```tea
use intrinsics = "std.intrinsics"

intrinsics.fs_write_text("output.txt", "Hello, Tea!")
```

### `fs_create_dir(dir_path: String) -> Void`

Create a directory. Parent directories must already exist.

**Examples:**

```tea
use intrinsics = "std.intrinsics"

intrinsics.fs_create_dir("my_folder")
```

**Note:** Use `std.fs.ensure_dir()` to create parent directories automatically.

### `fs_remove(file_path: String) -> Void`

Remove a file or directory (recursively if it's a directory).

**Examples:**

```tea
use intrinsics = "std.intrinsics"

intrinsics.fs_remove("temp.txt")
intrinsics.fs_remove("old_folder")  # Removes directory and contents
```

### `fs_exists(file_path: String) -> Bool`

Check if a file or directory exists.

**Examples:**

```tea
use intrinsics = "std.intrinsics"

if intrinsics.fs_exists("config.json")
  print("Config file found")
end
```

### `fs_list_dir(dir_path: String) -> List[String]`

List all entries in a directory (files and subdirectories).

**Examples:**

```tea
use intrinsics = "std.intrinsics"

var entries = intrinsics.fs_list_dir(".")
for entry in entries
  print(entry)
end
```

**Note:** Returns just the names, not full paths.

### `fs_walk(dir_path: String) -> List[String]`

Recursively walk a directory tree and return all file paths.

**Examples:**

```tea
use intrinsics = "std.intrinsics"

var all_files = intrinsics.fs_walk("src")
for file in all_files
  print(file)
end
```

**Note:** Returns full paths relative to the starting directory.

---

## Path

### `path_join(parts: List[String]) -> String`

Join path components using the system path separator.

**Examples:**

```tea
use intrinsics = "std.intrinsics"

var path = intrinsics.path_join(["usr", "local", "bin"])  # => "usr/local/bin"
```

### `path_components(file_path: String) -> List[String]`

Split a path into its component parts.

**Examples:**

```tea
use intrinsics = "std.intrinsics"

var parts = intrinsics.path_components("/usr/local/bin")  # => ["usr", "local", "bin"]
```

**Note:** Leading separators are removed from components.

### `path_dirname(file_path: String) -> String`

Get the directory portion of a path.

**Examples:**

```tea
use intrinsics = "std.intrinsics"

intrinsics.path_dirname("/usr/local/bin/tea")  # => "/usr/local/bin"
intrinsics.path_dirname("file.txt")            # => ""
```

### `path_basename(file_path: String) -> String`

Get the filename portion of a path.

**Examples:**

```tea
use intrinsics = "std.intrinsics"

intrinsics.path_basename("/usr/local/bin/tea")  # => "tea"
intrinsics.path_basename("file.txt")            # => "file.txt"
```

### `path_extension(file_path: String) -> String`

Get the file extension (without the dot).

**Examples:**

```tea
use intrinsics = "std.intrinsics"

intrinsics.path_extension("file.tea")     # => "tea"
intrinsics.path_extension("file.tar.gz")  # => "gz"
intrinsics.path_extension(".gitignore")   # => ""
```

### `path_normalize(file_path: String) -> String`

Normalize a path by resolving `.` and `..` components.

**Examples:**

```tea
use intrinsics = "std.intrinsics"

intrinsics.path_normalize("./foo/../bar")  # => "bar"
intrinsics.path_normalize("a/./b/../c")    # => "a/c"
```

### `path_absolute(file_path: String) -> String`

Convert a relative path to an absolute path.

**Examples:**

```tea
use intrinsics = "std.intrinsics"

var abs = intrinsics.path_absolute("file.txt")  # => "/current/working/dir/file.txt"
```

### `path_relative(from: String, to: String) -> String`

Get the relative path from one location to another.

**Examples:**

```tea
use intrinsics = "std.intrinsics"

var rel = intrinsics.path_relative("/usr/local", "/usr/local/bin")  # => "bin"
```

### `path_separator() -> String`

Get the system path separator (`/` on Unix, `\` on Windows).

**Examples:**

```tea
use intrinsics = "std.intrinsics"

var sep = intrinsics.path_separator()  # => "/" on Unix
```

---

## Best Practices

1. **Use Standard Library**: Prefer `std.*` modules over direct intrinsic calls
   - Standard library provides better error messages
   - Standard library may add validation and convenience features
   - Intrinsics are lower-level and may change

2. **Internal Use Only**: Some intrinsics (like `__intrinsic_fail`) are prefixed with `__` to indicate they're meant for internal use by the standard library

3. **Pure Tea Implementations**: Some standard library functions are implemented in pure Tea and call intrinsics internally. This provides better error handling and documentation.

## See Also

- [Standard Library Reference](stdlib-reference.md) - Higher-level APIs built on intrinsics
- [Language Guide](../README.md) - General Tea language documentation
