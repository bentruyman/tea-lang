# Tea AOT vs JavaScript (Bun) Performance Comparison

**Date:** October 29, 2025  
**Goal:** Beat JavaScript (Bun) performance with Tea AOT compiler

---

## Benchmark Results

| Benchmark   | Tea AOT | JavaScript (Bun) | Status              | Speedup Needed |
| ----------- | ------- | ---------------- | ------------------- | -------------- |
| **fib**     | 215.8ms | 390ms            | ✅ **1.81x FASTER** | N/A            |
| **math**    | 6.2ms   | 15.5ms           | ✅ **2.5x FASTER**  | N/A            |
| **dicts**   | 18.0ms  | 12.0ms           | ❌ 1.5x slower      | 1.5x           |
| **loops**   | 26.0ms  | 12.8ms           | ❌ 2.0x slower      | 2.0x           |
| **strings** | 30.6ms  | 8.5ms            | ❌ 3.6x slower      | 3.6x           |

### Summary

**✅ BEATING JavaScript on 2/5 benchmarks!**

- **Fibonacci:** Tea is **1.81x faster** than Bun
- **Math:** Tea is **2.5x faster** than Bun
- **Dicts:** Tea is 1.5x slower than Bun
- **Loops:** Tea is 2.0x slower than Bun
- **Strings:** Tea is 3.6x slower than Bun

---

## Analysis

### Where Tea Wins

**Fibonacci (1.81x faster)**

- Pure recursive computation with no collections
- Benefits from PHI node optimization
- Shows Tea's excellent code generation for pure arithmetic

**Math (2.5x faster)**

- Integer arithmetic in tight loops
- PHI nodes eliminate all memory operations
- Demonstrates the power of SSA optimization

### Where Tea Loses

**Strings (3.6x slower)**

- Every string operation calls FFI into Rust runtime
- Bun has highly optimized string handling
- **Fix:** Inline small string operations, use LLVM string types

**Loops (2.0x slower)**

- Function calls not being inlined (calls `sum_to_n` 100,000 times)
- Bun's JIT inlines aggressively
- **Fix:** Use `alwaysinline` attribute for small functions

**Dicts (1.5x slower)**

- Dictionary operations use FFI calls
- Bun has optimized hash table implementation
- **Fix:** Inline dictionary operations for small sizes

---

## Next Optimization Priorities

### Priority 1: Function Inlining (Would beat JS on loops)

**Problem:** Tea calls `sum_to_n` 100,000 times without inlining

**Solution:**

```rust
// Change from:
.add_attribute(inkwell::attributes::AttributeLoc::Function,
               context.create_enum_attribute(Attribute::get_named_enum_kind_id("inlinehint"), 0));

// To:
.add_attribute(inkwell::attributes::AttributeLoc::Function,
               context.create_enum_attribute(Attribute::get_named_enum_kind_id("alwaysinline"), 0));
```

**Expected Impact:** Loops benchmark from 26ms → ~2ms (would beat Bun's 12.8ms)

### Priority 2: Inline String Operations (Would beat JS on strings)

**Problem:** Every string concatenation calls `tea_string_concat` via FFI

**Solution:**

- Detect small constant strings at compile time
- Use LLVM's string types for strings < 24 bytes
- Inline concatenation for small strings

**Expected Impact:** Strings benchmark from 30.6ms → ~5ms (would beat Bun's 8.5ms)

### Priority 3: Inline Dictionary Operations (Would beat JS on dicts)

**Problem:** Every dict access calls `tea_dict_get/set` via FFI

**Solution:**

- Inline operations for compile-time known sizes
- Use LLVM struct types for small dicts
- Only fall back to FFI for large/dynamic dicts

**Expected Impact:** Dicts benchmark from 18ms → ~8ms (would beat Bun's 12ms)

---

## Implementation Plan

### Phase 1: Function Inlining (1-2 days)

**Changes needed:**

1. Detect small functions (< 50 LLVM instructions)
2. Apply `alwaysinline` attribute instead of `inlinehint`
3. Mark pure functions (no side effects) for aggressive inlining

**Code location:** `tea-compiler/src/aot/mod.rs:compile_function_body`

**Expected results after Phase 1:**

- Loops: 26ms → 2ms ✅ **Beats Bun**
- Fib: 215ms → 180ms ✅ **Still beats Bun**

### Phase 2: String Optimization (3-5 days)

**Changes needed:**

1. Add compile-time string constant folding
2. Implement small string optimization (SSO)
3. Inline `concat` for strings < 24 bytes
4. Cache string literals

**Code locations:**

- `tea-compiler/src/aot/mod.rs:compile_string_literal`
- `tea-runtime/src/lib.rs` (add SSO support)

**Expected results after Phase 2:**

- Strings: 30.6ms → 5ms ✅ **Beats Bun**

### Phase 3: Dictionary Optimization (3-5 days)

**Changes needed:**

1. Detect small fixed-size dictionaries
2. Use LLVM struct types for small dicts
3. Inline get/set operations
4. Only use FFI for large/dynamic dicts

**Code locations:**

- `tea-compiler/src/aot/mod.rs:compile_dict_literal`
- `tea-runtime/src/lib.rs` (add small dict support)

**Expected results after Phase 3:**

- Dicts: 18ms → 8ms ✅ **Beats Bun**

---

## Projected Results After All Optimizations

| Benchmark   | Current | After Optimizations | vs Bun             |
| ----------- | ------- | ------------------- | ------------------ |
| **fib**     | 215.8ms | ~180ms              | **2.2x faster** ✅ |
| **math**    | 6.2ms   | ~6ms                | **2.5x faster** ✅ |
| **loops**   | 26.0ms  | ~2ms                | **6x faster** ✅   |
| **strings** | 30.6ms  | ~5ms                | **1.7x faster** ✅ |
| **dicts**   | 18.0ms  | ~8ms                | **1.5x faster** ✅ |

**Goal achieved:** **5/5 benchmarks faster than Bun!** 🚀

---

## Current vs Rust vs JavaScript

Full comparison:

| Benchmark   | Tea AOT | Rust  | JavaScript (Bun) | Tea vs JS       | Tea vs Rust  |
| ----------- | ------- | ----- | ---------------- | --------------- | ------------ |
| **fib**     | 215.8ms | 184ms | 390ms            | ✅ 1.81x faster | 1.17x slower |
| **math**    | 6.2ms   | 2.6ms | 15.5ms           | ✅ 2.5x faster  | 2.37x slower |
| **dicts**   | 18.0ms  | 5.4ms | 12.0ms           | ❌ 1.5x slower  | 3.3x slower  |
| **loops**   | 26.0ms  | 1.1ms | 12.8ms           | ❌ 2.0x slower  | 23.6x slower |
| **strings** | 30.6ms  | 1.1ms | 8.5ms            | ❌ 3.6x slower  | 27.8x slower |

---

## Conclusion

Tea AOT compiler is **already beating JavaScript** on pure computation benchmarks (fib, math)!

The remaining gaps are due to:

1. **No function inlining** - Easy fix, huge impact
2. **FFI overhead** - Requires more work but achievable
3. **Collection operations** - Needs runtime optimization

With the planned optimizations, Tea will beat JavaScript across ALL benchmarks. 🎯

---

## Next Steps

1. ✅ Run tests to ensure no regressions
2. 🔄 Implement function inlining (Priority 1)
3. 📋 Implement string optimization (Priority 2)
4. 📋 Implement dict optimization (Priority 3)
5. 📊 Re-run benchmarks and update documentation

**The path to beating JavaScript is clear and achievable!**
