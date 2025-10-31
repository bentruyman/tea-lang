# Stdlib Intrinsics Reduction Analysis

## Goal

Reduce dependence on Rust intrinsics by implementing stdlib functions in pure Tea.

## Completed Language Features (2025-10-31)

### ✅ String Indexing

- **Syntax**: `string[index]` returns single character as String
- **Commit**: dd88fe5
- **Impact**: Enables character-level string manipulation

### ✅ List Concatenation

- **Syntax**: `list1 + list2` combines lists
- **Commit**: bab0c49
- **Impact**: Simplifies list building without explicit loops

### ✅ String/List Slicing

- **Syntax**: `collection[start..end]` (exclusive), `collection[start...end]` (inclusive)
- **Commit**: c1d28ab
- **Impact**: Efficient substring and sublist extraction
- **Note**: Unicode-aware for strings (char boundaries, not bytes)

## Successfully Migrated Functions

### `path.join(parts: List[String]) -> String`

- **Status**: ✅ Pure Tea implementation (commit 1fe8a68)
- **Technique**: List iteration + string concatenation
- **Code**:

```tea
pub def join(parts: List[String]) -> String
  if util.len(parts) == 0
    return ""
  end

  var result = parts[0]
  var sep = intrinsics.path_separator()
  var i = 1

  while i < util.len(parts)
    result = result + sep + parts[i]
    i = i + 1
  end

  result
end
```

## Intrinsics That Must Remain

### Type Predicates (`is_nil`, `is_int`, `is_string`, etc.)

**Why they can't be pure Tea:**

1. **No universal type**: Tea lacks an "Any" or "Value" supertype that can hold any value
2. **Type annotations required**: Public functions must have typed parameters
3. **Compile-time vs runtime**: The `is` operator works great for unions and specific types, but requires compile-time type knowledge

**Example of the limitation:**

```tea
# This doesn't work - 'value' needs a type
pub def is_int(value) -> Bool
  value is Int  # Compile error: untyped parameter
end

# This works but is too restrictive
union SomeTypes { Int, String, Bool }
pub def is_int_limited(value: SomeTypes) -> Bool
  value is Int  # OK, but only works for SomeTypes union
end
```

**What would enable pure Tea implementation:**

- Universal supertype (e.g., `Value`, `Any`, `Object`)
- Ability to define functions without type annotations
- Runtime type reflection API

### `to_string(value) -> String`

**Why it must remain intrinsic:**

1. Same universal type problem as type predicates
2. Needs to handle ALL types (Int, Float, Bool, String, List, Dict, Struct, Error, Closure)
3. Requires deep knowledge of value internal representation
4. Format conversion logic is complex (especially for collections and structs)

**What would enable pure Tea implementation:**

- Universal supertype
- Traits/interfaces for custom `to_string` implementations
- Match expressions that can handle any value type

## Language Features Still Needed

### For Path Module Completion

1. **String search/indexOf**
   - Find position of character or substring
   - Example: `"hello".index_of("l")` → `2`

2. **String split**
   - Break string by delimiter
   - Example: `"/usr/local/bin".split("/")` → `["", "usr", "local", "bin"]`

3. **String comparison operators**
   - Compare characters: `"a" < "z"`, `"A" <= "Z"`
   - Currently only `==` and `!=` work for strings

4. **String replace**
   - Replace substrings: `"a\\b".replace("\\", "/")`

5. **Logical operators**
   - Boolean combinations: `and`, `or`, `&&`, `||`
   - Currently requires nested if statements

### For Efficient String Building

1. **String builder/buffer**
   - Efficient concatenation in loops
   - Currently `str = str + part` creates new string each time

2. **Mutable strings or StringBuilder type**
   - Append operations without reallocation

## Migration Strategy Going Forward

### High Priority (Doable with current features)

- ✅ `path.join` - DONE
- Simple list/string manipulation functions
- Pure computation functions

### Medium Priority (Need search/split)

- `path.basename` - needs finding last `/`
- `path.dirname` - needs finding last `/`
- `path.extension` - needs finding last `.`
- String utilities module

### Low Priority (Complex or OS-dependent)

- Path normalization (complex algorithm)
- Path resolution (OS-dependent)
- Filesystem operations (inherently require OS access)
- Environment operations (inherently require OS access)
- I/O operations (inherently require OS access)

## Recommendations

### For Language Design

1. **Consider adding a universal "Value" type**
   - Would enable generic utility functions
   - Could be opt-in for functions that need it
   - Trade-off: loses some type safety

2. **Add string utility built-ins**
   - `index_of`, `split`, `replace`, `contains`
   - These are fundamental enough to justify built-in status
   - Can be implemented efficiently in Rust

3. **Support logical operators**
   - `and`/`or` keywords or `&&`/`||` operators
   - Common in all modern languages

4. **Add string comparison**
   - Enable `<`, `>`, `<=`, `>=` for strings
   - Lexicographic ordering

### For Stdlib Migration

1. **Focus on pure computation first**
   - Functions that don't need OS access
   - Functions that work with existing types

2. **Document why intrinsics are needed**
   - Help users understand limitations
   - Guide future language development

3. **Keep intrinsics for OS interaction**
   - File system, process, environment operations
   - These will always need native code

## Conclusion

The recent language features (string indexing, list concatenation, slicing) have successfully enabled migration of some stdlib functions to pure Tea. However, fundamental limitations around type flexibility mean that certain utilities (type predicates, to_string) must remain intrinsic-backed.

Future progress depends on:

1. String utility functions (search, split, replace)
2. Possibly a universal value type for generic utilities
3. Additional operators (logical, string comparison)

The path forward is clear: focus on migrating functions that work with specific types, while accepting that truly generic runtime utilities require intrinsic support in Tea's current design.
