# Tea Maintainer Guide

This guide explains how to extend the Tea language with new intrinsics and standard library functions.

## Table of Contents

- [Adding a New Intrinsic](#adding-a-new-intrinsic)
- [Adding a New Standard Library Module](#adding-a-new-standard-library-module)
- [Adding Functions to Existing Modules](#adding-functions-to-existing-modules)
- [Architecture Overview](#architecture-overview)

---

## Adding a New Intrinsic

Intrinsics are low-level functions implemented in Rust and exposed to Tea code. They form the minimal native surface that the standard library is built upon.

### When to Add an Intrinsic

Add an intrinsic when you need to:

- Access system functionality (filesystem, environment, etc.)
- Perform operations that can't be implemented in pure Tea
- Provide critical performance-sensitive operations

**Don't** add an intrinsic if the functionality can be implemented in Tea using existing intrinsics.

### Steps to Add a New Intrinsic

Thanks to our macro-based registration system, adding an intrinsic is now simple!

Let's add a hypothetical `fs_copy` intrinsic as an example.

#### 1. Implement the Core Function

For intrinsics with significant logic, first implement in the shared core.

**File:** `tea-intrinsics/src/fs.rs`

```rust
use anyhow::Result;
use std::fs;
use tea_support::fs_error;

/// Copies a file from source to destination
pub fn copy(source: &str, dest: &str) -> Result<()> {
    fs::copy(source, dest)
        .map_err(|error| anyhow::anyhow!(fs_error("copy", source, &error)))?;
    Ok(())
}
```

#### 2. Add VM Wrapper

**File:** `tea-compiler/src/runtime/intrinsics_impl/fs.rs`

Add a thin wrapper that converts `Value` types and calls the core:

```rust
pub fn copy(_vm: &mut Vm, args: Vec<Value>) -> Result<Value> {
    if args.len() != 2 {
        bail!(VmError::Runtime(format!(
            "fs_copy expected 2 arguments but got {}",
            args.len()
        )));
    }
    let source = match &args[0] {
        Value::String(s) => s.as_str(),
        _ => bail!(VmError::Runtime(
            "fs_copy expects source to be a String".to_string()
        )),
    };
    let dest = match &args[1] {
        Value::String(s) => s.as_str(),
        _ => bail!(VmError::Runtime(
            "fs_copy expects dest to be a String".to_string()
        )),
    };

    tea_intrinsics::fs::copy(source, dest)
        .map_err(|e| VmError::Runtime(e.to_string()))?;

    Ok(Value::Void)
}
```

#### 3. Add AOT Wrapper (if needed)

**File:** `tea-runtime/src/lib.rs`

Add a C FFI wrapper that converts C types and calls the core:

```rust
#[no_mangle]
pub extern "C" fn tea_fs_copy(source: *const TeaString, dest: *const TeaString) {
    let source_str = expect_path(source);
    let dest_str = expect_path(dest);
    tea_intrinsics::fs::copy(&source_str, &dest_str)
        .unwrap_or_else(|error| panic!("{}", error));
}
```

#### 4. Add StdFunctionKind Variant

**File:** `tea-compiler/src/stdlib/mod.rs`

Add a variant to the `StdFunctionKind` enum:

```rust
pub enum StdFunctionKind {
    // ... existing variants ...
    FsCopy,    // ← Add your variant
}
```

#### 5. Register in the Macro

**File:** `tea-compiler/src/runtime/intrinsics.rs`

Add an entry to the `define_intrinsics!` macro invocation:

```rust
define_intrinsics! {
    // ... existing intrinsics ...

    // ===== Filesystem =====
    {
        name: "fs_copy",                    // Function name (without __intrinsic_ prefix)
        kind: FsCopy,                       // StdFunctionKind variant
        arity: StdArity::Exact(2),          // Accepts exactly 2 arguments
        params: [StdType::String, StdType::String],  // Parameter types
        return_type: StdType::Void,         // Return type
        impl_fn: fs::copy                   // Implementation function (VM wrapper)
    },

    // ... rest ...
}
```

#### 6. Add to Backward-Compatible Enum (for AOT)

**File:** `tea-compiler/src/runtime/intrinsics.rs`

Add the variant to the `Intrinsic` enum and its `from_name` method:

```rust
pub enum Intrinsic {
    // ... existing variants ...
    FsCopy,    // ← Add variant
}

impl Intrinsic {
    pub fn from_name(name: &str) -> Option<Self> {
        let name = name.strip_prefix("__intrinsic_")?;
        Some(match name {
            // ... existing cases ...
            "fs_copy" => Self::FsCopy,    // ← Add parsing
            _ => return None,
        })
    }
}
```

#### 7. Update Documentation

**File:** `docs/intrinsics-reference.md`

Document your new intrinsic:

````markdown
### `fs_copy(source: String, dest: String) -> Void`

Copy a file from source to destination.

**Examples:**

```tea
use intrinsics = "std.intrinsics"

intrinsics.fs_copy("input.txt", "output.txt")
```

**Note:** Prefer using `std.fs.copy()` which provides better error handling.
````

### Checklist

- [ ] Implement core function in `tea-intrinsics/src/<category>.rs`
- [ ] Add VM wrapper in `runtime/intrinsics_impl/<category>.rs`
- [ ] Add AOT wrapper in `tea-runtime/src/lib.rs` (if needed)
- [ ] Add `StdFunctionKind` variant in `stdlib/mod.rs`
- [ ] Register in `define_intrinsics!` macro in `runtime/intrinsics.rs`
- [ ] Add to `Intrinsic` enum and `from_name()` for AOT compatibility
- [ ] Add documentation to `docs/intrinsics-reference.md`
- [ ] Run `cargo test --workspace`
- [ ] Run `cargo fmt --all`

**That's it!** The macro system automatically handles:

- Function registration and dispatch
- Metadata storage (arity, types, etc.)
- Integration with the VM

---

## Adding a New Standard Library Module

Standard library modules provide high-level, user-friendly APIs built on top of intrinsics.

### When to Add a Module

Add a new module when you have:

- A cohesive set of related functionality
- At least 3-5 functions that belong together
- A clear user-facing API that's distinct from existing modules

### Steps to Add a New Module

Let's add a hypothetical `std.net` module for network operations.

#### 1. Create Module File

**File:** `tea-compiler/src/stdlib/net.rs`

```rust
use super::{std_function, std_module, StdArity, StdFunction, StdFunctionKind, StdModule, StdType};

const NET_FUNCTIONS: &[StdFunction] = &[
    std_function(
        "http_get",
        StdFunctionKind::NetHttpGet,
        StdArity::Exact(1),
        &[StdType::String],
        StdType::String,
    ),
];

pub const MODULE: StdModule = std_module!(
    "std.net",
    "Network operations and HTTP utilities",
    NET_FUNCTIONS,
);
```

#### 2. Add Function Kinds

**File:** `tea-compiler/src/stdlib/mod.rs`

```rust
pub enum StdFunctionKind {
    // ... existing variants ...
    NetHttpGet,
}
```

#### 3. Register Module

**File:** `tea-compiler/src/stdlib/mod.rs`

```rust
mod net;    // ← Add module declaration

pub static MODULES: &[StdModule] = &[
    assert::MODULE,
    env::MODULE,
    fs::MODULE,
    path::MODULE,
    intrinsics::MODULE,
    net::MODULE,    // ← Add to module list
];
```

#### 4. Add Intrinsic or Builtin Implementation

If it needs native code, add intrinsics (see above). If it's pure Tea, create `stdlib/net/mod.tea`.

**For native (builtin) functions**, implement in VM's `execute_builtin`:

**File:** `tea-compiler/src/runtime/vm.rs`

```rust
fn execute_builtin(&mut self, kind: StdFunctionKind, arg_count: usize) -> Result<()> {
    let args = self.pop_n(arg_count)?;

    // ... intrinsic dispatch ...

    match kind {
        // ... existing builtins ...
        StdFunctionKind::NetHttpGet => {
            if args.len() != 1 {
                bail!(VmError::Runtime(format!(
                    "http_get expected 1 argument but got {}",
                    args.len()
                )));
            }
            let url = match &args[0] {
                Value::String(s) => s.as_str(),
                _ => bail!(VmError::Runtime(
                    "http_get expects a String".to_string()
                )),
            };

            // Implementation...

            self.stack.push(Value::String(response));
        }
    }
}
```

#### 5. Add Documentation

**File:** `docs/stdlib-reference.md`

```markdown
## std.net

Network operations and HTTP utilities.

### Functions

#### `get(url: String) -> String`

Make an HTTP GET request.

**Example:**
\`\`\`tea
use net = "std.net"

var response = net.get("https://api.example.com/data")
\`\`\`
```

#### 6. Add Tests

**File:** `tea-compiler/tests/runtime_net.rs`

```rust
#[test]
fn http_get_basic() -> anyhow::Result<()> {
    let source = r#"
        use net = "std.net"
        var response = net.get("https://httpbin.org/get")
    "#;

    // Compile and run test...
    Ok(())
}
```

### Checklist

- [ ] Create module file in `tea-compiler/src/stdlib/`
- [ ] Add function kinds to `StdFunctionKind`
- [ ] Declare and register module in `stdlib/mod.rs`
- [ ] Implement functions (as intrinsics or builtins)
- [ ] (Optional) Create Tea module in `stdlib/`
- [ ] Add documentation to `docs/stdlib-reference.md`
- [ ] Add tests in `tea-compiler/tests/`
- [ ] Run `cargo test --workspace`
- [ ] Run `cargo fmt --all`

---

## Adding Functions to Existing Modules

This is the simplest path for extending functionality.

### Example: Adding `ensure_dir` to `std.fs`

This is now a **4-step process**:

#### 1. Implement the Function

**File:** `tea-compiler/src/runtime/intrinsics_impl/fs.rs`

```rust
pub fn ensure_dir(_vm: &mut Vm, args: Vec<Value>) -> Result<Value> {
    if args.len() != 1 {
        bail!(VmError::Runtime(format!(
            "ensure_dir expected 1 argument but got {}",
            args.len()
        )));
    }
    let path = match &args[0] {
        Value::String(text) => text.clone(),
        _ => bail!(VmError::Runtime(
            "ensure_dir expects the path to be a String".to_string()
        )),
    };
    fs::create_dir_all(&path)
        .map_err(|error| VmError::Runtime(fs_error("ensure_dir", &path, &error)))?;
    Ok(Value::Void)
}
```

#### 2. Add Function Kind

**File:** `tea-compiler/src/stdlib/mod.rs`

```rust
pub enum StdFunctionKind {
    // ... existing ...
    FsEnsureDir,    // ← Add variant
}
```

#### 3. Register in Macro

**File:** `tea-compiler/src/runtime/intrinsics.rs`

```rust
{
    name: "fs_ensure_dir",
    kind: FsEnsureDir,
    arity: StdArity::Exact(1),
    params: [StdType::String],
    return_type: StdType::Void,
    impl_fn: fs::ensure_dir
},
```

#### 4. Update Documentation

**File:** `docs/stdlib-reference.md`

```markdown
#### `ensure_dir(path: String) -> Void`

Create a directory and all its parent directories if they don't exist.
```

### Checklist

- [ ] Add implementation to appropriate `intrinsics_impl/<category>.rs`
- [ ] Add variant to `StdFunctionKind`
- [ ] Register in `define_intrinsics!` macro
- [ ] Update docs
- [ ] Run tests

---

## Architecture Overview

### Before: The Old Way (8+ Steps, 4+ Files)

Adding an intrinsic used to require:

1. Add enum variant to `Intrinsic` in `runtime/intrinsics.rs`
2. Add to `from_name()` method
3. Add to `name()` method
4. Add to `all()` iterator
5. Add variant to `StdFunctionKind` in `stdlib/mod.rs`
6. Add function definition to `stdlib/intrinsics.rs`
7. Add giant match arm in `execute_builtin()` in `runtime/vm.rs` (~30 lines of code inline)
8. Add documentation

**Problems:**

- Lots of boilerplate and duplication
- Easy to forget a step
- Giant 800+ line `execute_builtin()` function
- Hard to test individual intrinsics

### After: The New Way (4 Steps, 3 Files)

1. Implement function in organized module
2. Add `StdFunctionKind` variant
3. Register in macro (one declaration)
4. Update docs

**Benefits:**

- **Single source of truth**: Each intrinsic defined once in the macro
- **Better organization**: Implementations in category-specific modules
- **Automatic dispatch**: VM automatically routes to implementations
- **Cleaner code**: 800+ line function reduced to ~50 lines
- **Easier testing**: Each intrinsic is a standalone function
- **Less duplication**: No manual enum/name/iterator management

### The Deduplication Architecture (VM + AOT Shared Core)

To eliminate duplication between the VM (bytecode interpreter) and AOT (compiled) paths, we use a **shared core** approach:

```
┌─────────────────────────────────────────────────────────────┐
│                    tea-intrinsics crate                      │
│           (Pure Rust, no VM or AOT dependencies)            │
│                                                              │
│  pub fn read_text(path: &str) -> Result<String>            │
│  pub fn write_text(path: &str, contents: &str) -> Result<()>│
│  pub fn exists(path: &str) -> bool                          │
│  ...                                                         │
└─────────────────────────────────────────────────────────────┘
                    ▲                           ▲
                    │                           │
      ┌─────────────┴──────┐      ┌────────────┴───────────┐
      │   VM Thin Wrapper  │      │  AOT Thin Wrapper      │
      │  (tea-compiler)    │      │  (tea-runtime)         │
      │                    │      │                        │
      │  Value → &str      │      │  *TeaString → &str     │
      │  call core         │      │  call core             │
      │  String → Value    │      │  String → *TeaString   │
      └────────────────────┘      └────────────────────────┘
```

**How it works:**

1. **Core Logic** (`tea-intrinsics` crate): Pure Rust functions with no VM/AOT types
   - Takes standard Rust types: `&str`, `Vec<String>`, `bool`, etc.
   - Returns `Result<T>` or plain values
   - Contains all business logic
   - Example: `pub fn read_text(path: &str) -> Result<String>`

2. **VM Wrapper** (`tea-compiler/src/runtime/intrinsics_impl/`): Thin adapter layer
   - Extracts `&str` from `Value::String`
   - Calls `tea_intrinsics::fs::read_text(path)`
   - Converts `Result<String>` back to `Value::String` or error
   - Example: `pub fn read_text(_vm: &mut Vm, args: Vec<Value>) -> Result<Value>`

3. **AOT Wrapper** (`tea-runtime/src/lib.rs`): Thin C FFI adapter layer
   - Extracts `&str` from `*const TeaString` pointer
   - Calls `tea_intrinsics::fs::read_text(path)`
   - Converts `String` back to `*mut TeaString` for C FFI
   - Example: `pub extern "C" fn tea_fs_read_text(path: *const TeaString) -> *mut TeaString`

**Benefits:**

- **No duplication**: Core logic written once, used by both VM and AOT
- **Easier testing**: Test core functions directly without VM/FFI overhead
- **Type safety**: Core functions use idiomatic Rust types
- **Easier maintenance**: Bug fixes and improvements in one place
- **Clear separation**: Type conversions isolated to thin wrapper layers

**When adding intrinsics now:**

If your intrinsic has significant logic (filesystem, env vars, path manipulation, etc.):

1. Add core function to `tea-intrinsics/src/<category>.rs`
2. Add VM wrapper in `tea-compiler/src/runtime/intrinsics_impl/<category>.rs`
3. Add AOT wrapper in `tea-runtime/src/lib.rs`
4. Both wrappers just do type conversion and call the core

For simple intrinsics (type conversions, assertions, etc.), you can skip step 1 and implement directly in the wrappers.

### Key Files

**Shared Core Logic:**

- `tea-intrinsics/` - **Shared intrinsics core** (pure Rust, no VM/AOT types)
  - `src/fs.rs` - Filesystem operations core
  - `src/env.rs` - Environment variable operations core
  - `src/path.rs` - Path manipulation core
  - `src/string.rs` - String utilities core

**VM (Bytecode Interpreter):**

- `tea-compiler/src/runtime/intrinsics.rs` - Macro-based registry
- `tea-compiler/src/runtime/intrinsics_impl/` - VM thin wrappers (converts `Value` ↔ core types)
  - `assert.rs` - Assertion intrinsics
  - `env.rs` - Environment variable intrinsics wrapper
  - `fs.rs` - Filesystem intrinsics wrapper
  - `path.rs` - Path manipulation intrinsics wrapper
  - `string.rs` - String utility intrinsics wrapper
  - `util.rs` - Conversion utilities
- `tea-compiler/src/runtime/vm.rs` - VM dispatch (now just ~50 lines)
- `tea-compiler/src/stdlib/mod.rs` - Function kind enum

**AOT (Ahead-of-Time Compiler):**

- `tea-runtime/src/lib.rs` - AOT thin wrappers (converts C FFI types ↔ core types)

---

## See Also

- [Intrinsics Reference](intrinsics-reference.md) - Complete list of all intrinsics
- [Standard Library Reference](stdlib-reference.md) - User-facing stdlib documentation
- [Language Reference](reference/language/semantics.md) - Tea language semantics
