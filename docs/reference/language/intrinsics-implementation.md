# Intrinsics Implementation Guide

## Overview

This document describes the intrinsics system in Tea - a set of low-level native functions that provide the foundation for the standard library.

## Architecture

### What are Intrinsics?

Intrinsics are native functions implemented in Rust (via tea-runtime) that provide fundamental operations like:

- Type checking (`is_int`, `is_string`, etc.)
- Environment access (`env_get`, `env_cwd`, etc.)
- Filesystem operations (`fs_read_text`, `fs_exists`, etc.)
- Path manipulation (`path_join`, `path_basename`, etc.)
- I/O operations (`io_write`, `io_read_line`, etc.)
- Codecs (`json_encode`, `yaml_decode`, etc.)

### Design Principles

1. **Module-based access**: Intrinsics are accessed via `use intrinsics = "std.intrinsics"` rather than as global functions
2. **AOT compilation support**: All intrinsic calls are detected and compiled to direct native function calls in LLVM backend
3. **Type safety**: Each intrinsic has a proper type signature enforced by the compiler
4. **Minimal surface**: Only essential operations are exposed as intrinsics

## Usage

### Accessing Intrinsics

```tea
use intrinsics = "std.intrinsics"

pub def example() -> Void
  # Type checking
  if intrinsics.is_string("hello")
    print("It's a string!")
  end

  # Environment access
  var home = intrinsics.env_home_dir()
  print(home)

  # Path operations
  var parts = ["usr", "local", "bin"]
  var path = intrinsics.path_join(parts)
  print(path)

  # Filesystem
  var exists = intrinsics.fs_exists("/tmp")
  if exists
    print("Temp directory exists")
  end
end
```

### Available Intrinsics

#### Type Predicates

- `is_nil(value)` → Bool
- `is_bool(value)` → Bool
- `is_int(value)` → Bool
- `is_float(value)` → Bool
- `is_string(value)` → Bool
- `is_list(value)` → Bool
- `is_struct(value)` → Bool
- `is_error(value)` → Bool

#### Conversions

- `to_string(value)` → String

#### Assertions

- `fail(message: String)` → Void

#### Environment

- `env_get(name: String)` → String
- `env_set(name: String, value: String)` → Void
- `env_unset(name: String)` → Void
- `env_has(name: String)` → Bool
- `env_vars()` → Dict[String, String]
- `env_cwd()` → String
- `env_set_cwd(path: String)` → Void
- `env_temp_dir()` → String
- `env_home_dir()` → String
- `env_config_dir()` → String

#### Filesystem

- `fs_read_text(path: String)` → String
- `fs_write_text(path: String, content: String)` → Void
- `fs_write_text_atomic(path: String, content: String)` → Void
- `fs_create_dir(path: String)` → Void
- `fs_remove(path: String)` → Void
- `fs_exists(path: String)` → Bool
- `fs_is_dir(path: String)` → Bool
- `fs_is_symlink(path: String)` → Bool
- `fs_size(path: String)` → Int
- `fs_modified(path: String)` → Int
- `fs_permissions(path: String)` → Int
- `fs_is_readonly(path: String)` → Bool
- `fs_list_dir(path: String)` → List[String]
- `fs_walk(path: String)` → List[String]
- `fs_glob(pattern: String)` → List[String]
- `fs_metadata(path: String)` → Struct

#### Path Operations

- `path_join(parts: List[String])` → String
- `path_components(path: String)` → List[String]
- `path_dirname(path: String)` → String
- `path_basename(path: String)` → String
- `path_extension(path: String)` → String
- `path_set_extension(path: String, ext: String)` → String
- `path_strip_extension(path: String)` → String
- `path_normalize(path: String)` → String
- `path_absolute(path: String)` → String
- `path_relative(from: String, to: String)` → String
- `path_is_absolute(path: String)` → Bool
- `path_separator()` → String

#### I/O Operations

- `io_read_line()` → String
- `io_read_all()` → String
- `io_write(text: String)` → Void
- `io_write_err(text: String)` → Void
- `io_flush()` → Void

#### Process Operations

- `process_run(args: List[String])` → Struct

#### Codecs

- `json_encode(value: Any)` → String
- `json_decode(json: String)` → Any
- `yaml_encode(value: Any)` → String
- `yaml_decode(yaml: String)` → Any

#### CLI

- `cli_args()` → List[String]

## Implementation Details

### Compiler Flow

1. **Name Resolution**: `std.intrinsics` module is registered in stdlib modules list
2. **Type Checking**: Intrinsic calls are type-checked like any other stdlib function
3. **Code Generation**:
   - **Bytecode**: Intrinsics map to existing `StdFunctionKind` enum variants
   - **AOT/LLVM**: Intrinsic calls are detected in `compile_call_internal()` and routed to `compile_intrinsic_call()`

### AOT Compilation Path

When compiling with LLVM backend:

1. Parser detects call to `intrinsics.function_name`
2. Resolver recognizes it as a stdlib module function
3. Type checker validates arguments and return type
4. AOT codegen detects `StdFunctionKind::*` and routes to appropriate intrinsic
5. `compile_intrinsic_call()` matches the intrinsic variant and calls the corresponding compilation method
6. Each method generates LLVM IR that calls the native tea-runtime function

Example:

```
intrinsics.env_get("PATH")
  ↓ (resolver)
StdFunctionKind::EnvGet
  ↓ (AOT codegen)
Intrinsic::EnvGet
  ↓ (compile_intrinsic_call)
compile_env_get_call()
  ↓ (LLVM IR generation)
call @tea_env_get(...)
  ↓ (linking)
tea_runtime::tea_env_get (native Rust function)
```

### Adding New Intrinsics

To add a new intrinsic:

1. **Define the enum variant** in `tea-compiler/src/runtime/intrinsics.rs`:

   ```rust
   pub enum Intrinsic {
       // ...
       MyNewIntrinsic,
   }
   ```

2. **Add to from_name()** method:

   ```rust
   pub fn from_name(name: &str) -> Option<Self> {
       // ...
       "my_new_intrinsic" => Self::MyNewIntrinsic,
   }
   ```

3. **Add to name()** method:

   ```rust
   pub fn name(self) -> &'static str {
       // ...
       Self::MyNewIntrinsic => "__intrinsic_my_new_intrinsic",
   }
   ```

4. **Add to all()** iterator:

   ```rust
   pub fn all() -> impl Iterator<Item = Self> {
       [
           // ...
           MyNewIntrinsic,
       ].into_iter()
   }
   ```

5. **Register in std.intrinsics module** in `tea-compiler/src/stdlib/intrinsics.rs`:

   ```rust
   std_function(
       "my_new_intrinsic",
       StdFunctionKind::MyNewIntrinsic,  // Add to StdFunctionKind too!
       StdArity::Exact(1),
       &[StdType::String],
       StdType::Int,
   ),
   ```

6. **Implement AOT compilation** in `tea-compiler/src/aot/mod.rs`:

   ```rust
   fn compile_intrinsic_call(...) {
       match intrinsic {
           // ...
           Intrinsic::MyNewIntrinsic => {
               self.compile_my_new_intrinsic_call(&call.arguments, function, locals)
           }
       }
   }
   ```

7. **Implement native function** in `tea-runtime/src/lib.rs`:
   ```rust
   #[no_mangle]
   pub extern "C" fn tea_my_new_intrinsic(arg: *const TeaString) -> c_int {
       // Implementation
   }
   ```

## Testing

Comprehensive tests are available in:

- `examples/language/basics/intrinsics_test.tea` - Basic intrinsic usage
- `examples/language/basics/intrinsics_comprehensive_test.tea` - Full test suite

Run tests:

```bash
# Bytecode backend
cargo run -p tea-cli -- examples/language/basics/intrinsics_comprehensive_test.tea --backend bytecode

# AOT backend
cargo run -p tea-cli -- build examples/language/basics/intrinsics_comprehensive_test.tea -o /tmp/test
/tmp/test
```

## Future Work

- **Snapshot compilation**: Compile Tea stdlib modules to native objects during build
- **Cross-platform support**: Handle platform-specific intrinsics
- **Optimization**: Inline common intrinsics in AOT
- **Error handling**: Better error types for intrinsics that can fail
