# Tea AOT Performance Analysis & Optimization Opportunities

**Date:** 2025-10-29  
**Goal:** Match or exceed Rust performance for compiled binaries

---

## Executive Summary

The Tea AOT compiler currently produces binaries that are **1.17x to 57.8x slower than Rust** across different workloads. The biggest performance gaps are:

1. **Loops benchmark**: 57.8x slower (111ms vs 1.9ms)
2. **Strings benchmark**: 14.9x slower (30ms vs 2ms)
3. **Math benchmark**: 7.2x slower (26ms vs 3.7ms)
4. **Dicts benchmark**: 3.3x slower (19.5ms vs 6ms)
5. **Fib benchmark**: 1.17x slower (206ms vs 176ms) ✅ Close to target!

**Good news:** The fibonacci benchmark shows Tea is very close to Rust performance, indicating the compiler foundations are solid. The other benchmarks reveal specific bottlenecks that can be addressed.

---

## Current Benchmark Results (Oct 29, 2025)

| Benchmark | Tea AOT | Rust    | Slowdown  | Gap     |
| --------- | ------- | ------- | --------- | ------- |
| loops     | 111.3ms | 1.9ms   | **57.8x** | 109.4ms |
| strings   | 30.3ms  | 2.0ms   | **14.9x** | 28.3ms  |
| math      | 26.4ms  | 3.7ms   | **7.2x**  | 22.7ms  |
| dicts     | 19.5ms  | 6.0ms   | **3.3x**  | 13.5ms  |
| fib       | 205.9ms | 176.4ms | 1.17x     | 29.5ms  |

---

## Root Cause Analysis

### Critical Issue #1: Unnecessary Memory Operations in Hot Loops

**Affected benchmarks:** loops (57.8x slower), math (7.2x slower)

**Problem identified in LLVM IR:**

```llvm
define i64 @sum_to_n(i64 %n) {
entry:
  %i = alloca i64, align 8        ; ← Stack allocation
  %total = alloca i64, align 8    ; ← Stack allocation
  store i64 0, ptr %total, align 4
  store i64 1, ptr %i, align 4
  br label %loop_cond

loop_cond:
  %i1 = load i64, ptr %i, align 4      ; ← Load from memory
  %cmptmp = icmp sle i64 %i1, %n
  br i1 %cmptmp, label %loop_body, label %loop_exit

loop_body:
  %total2 = load i64, ptr %total, align 4  ; ← Load from memory
  %i3 = load i64, ptr %i, align 4          ; ← Load from memory
  %addtmp = add i64 %total2, %i3
  store i64 %addtmp, ptr %total, align 4   ; ← Store to memory
  %i4 = load i64, ptr %i, align 4          ; ← Load from memory
  %addtmp5 = add i64 %i4, 1
  store i64 %addtmp5, ptr %i, align 4      ; ← Store to memory
  br label %loop_cond
```

**Impact:**

- **4 loads + 2 stores per loop iteration** = 6 memory operations
- Loops benchmark runs 100,000 × 1,000 iterations = **600 million memory operations**
- Modern CPUs can execute ~10 billion integer adds/sec but only ~100 million random memory ops/sec
- This explains the **57.8x slowdown** on the loops benchmark

**Equivalent Rust code** (what LLVM generates for Rust):

```llvm
loop_body:
  %total.phi = phi i64 [ 0, %entry ], [ %addtmp, %loop_body ]  ; ← In register!
  %i.phi = phi i64 [ 1, %entry ], [ %addtmp5, %loop_body ]     ; ← In register!
  %addtmp = add i64 %total.phi, %i.phi                         ; ← Pure ALU op
  %addtmp5 = add i64 %i.phi, 1                                 ; ← Pure ALU op
  %cmptmp = icmp sle i64 %addtmp5, %n
  br i1 %cmptmp, label %loop_body, label %loop_exit
```

Rust uses **PHI nodes** (SSA form) instead of memory operations, which:

- Keeps values in CPU registers
- Enables aggressive optimization (constant propagation, dead code elimination)
- Allows auto-vectorization (SIMD)

---

### Critical Issue #2: Runtime FFI Calls for Collections

**Affected benchmarks:** strings (14.9x slower), dicts (3.3x slower)

**Problem:** Every string and dict operation calls into the Rust runtime via FFI:

```llvm
declare ptr @tea_alloc_string(i64, ptr)
declare ptr @tea_string_concat(ptr, ptr)
declare void @tea_dict_set(ptr, ptr, ptr)
declare ptr @tea_dict_get(ptr, ptr)
```

**Impact:**

- FFI call overhead (save/restore registers, ABI conversion)
- Prevents inlining and optimization across FFI boundary
- Heap allocation for every string operation
- Hash table lookups for every dict operation

**Example:** The strings benchmark does 100 iterations × 1,000 concatenations = **100,000 FFI calls** to `tea_string_concat`.

---

### Critical Issue #3: Global Variables Instead of Stack Locals

**Affected benchmarks:** All benchmarks

**Problem:**

```llvm
@.binding.i = private global i64 0
@.binding.result = private global i64 0
@.binding.iterations = private global i64 0

loop_body:
  %i = load i64, ptr @.binding.i, align 4        ; ← Global memory load
  %iterations = load i64, ptr @.binding.iterations, align 4
  %cmptmp = icmp slt i64 %i, %iterations
```

**Impact:**

- Every access requires a memory load (can't keep in register)
- Prevents mem2reg optimization pass from promoting to SSA
- Forces conservative alias analysis (globals could be modified anywhere)
- Prevents loop optimizations (unrolling, vectorization)

---

## High-Priority Optimization Plan

### 🔥 Priority 1: Fix Loop Performance (Target: 50x speedup on loops benchmark)

**Issue:** Unnecessary stack allocations and memory operations in tight loops

**Current code location:** `tea-compiler/src/aot/mod.rs:1462-1520`

**Fix:**

1. **Distinguish mutable vs immutable variables:**

   ```rust
   struct LocalVariable<'ctx> {
       pointer: Option<PointerValue<'ctx>>,  // Only for mutable vars
       value: Option<BasicValueEnum<'ctx>>,   // For immutable vars (SSA)
       ty: ValueType,
       mutable: bool,  // ← Already tracked!
   }
   ```

2. **Modify `compile_var_statement` to use SSA for immutable locals:**

   ```rust
   // Current (always allocates):
   let ptr = self.builder.build_alloca(ty, name);
   self.builder.build_store(ptr, value);

   // Proposed (SSA for immutable):
   if var_stmt.is_const || !mutated {
       // Store SSA value directly
       local_vars.insert(name, LocalVariable {
           pointer: None,
           value: Some(value),
           ty: value_type,
           mutable: false,
       });
   } else {
       // Allocate for mutable
       let ptr = self.builder.build_alloca(ty, name);
       self.builder.build_store(ptr, value);
       local_vars.insert(name, LocalVariable {
           pointer: Some(ptr),
           value: None,
           ty: value_type,
           mutable: true,
       });
   }
   ```

3. **Use PHI nodes for loop variables:**

   ```rust
   // When entering loop body:
   let phi = self.builder.build_phi(self.int_type(), "i.phi");
   phi.add_incoming(&[(&initial_value, entry_block)]);

   // When updating in loop:
   let new_value = self.builder.build_int_add(phi_value, increment, "i.next");
   phi.add_incoming(&[(&new_value, loop_body_block)]);
   ```

**Expected impact:** Loops benchmark from 111ms → **~2ms** (55x speedup)

---

### 🔥 Priority 2: Inline Primitive Operations (Target: 10-15x speedup on strings/math)

**Issue:** String concatenation and arithmetic go through FFI

**Fix:**

1. **Detect small string concatenations at compile time:**

   ```rust
   if left.is_constant_string() && right.is_constant_string() {
       // Fold at compile time
       return const_string(left_str + right_str);
   }
   ```

2. **Inline integer arithmetic (no FFI):**

   ```rust
   // Already done correctly! Just need to ensure no boxing
   BinaryOperator::Add => {
       let result = self.builder.build_int_add(lhs, rhs, "addtmp");
       Ok(ExprValue::Int(result))  // ← Stay in SSA form
   }
   ```

3. **Use small-string optimization:**
   ```rust
   // Strings < 24 bytes: store inline instead of heap-allocating
   struct SmallString {
       len: u8,
       data: [u8; 23],
   }
   ```

**Expected impact:**

- Strings benchmark: 30ms → **2-3ms** (10-15x speedup)
- Math benchmark: 26ms → **3-4ms** (6-8x speedup)

---

### 🔥 Priority 3: Move Globals to Stack (Target: 2-3x speedup on all benchmarks)

**Issue:** Top-level variables use global memory instead of stack

**Fix:**

1. **Allocate top-level vars on stack in main:**

   ```rust
   define i32 @main() {
   entry:
     %i = alloca i64
     %result = alloca i64
     %iterations = alloca i64
     %n = alloca i64
     store i64 0, ptr %i
     store i64 0, ptr %result
     store i64 100000, ptr %iterations
     store i64 1000, ptr %n
     ; ... rest of main
   }
   ```

2. **Or better, use SSA values directly:**

   ```rust
   define i32 @main() {
   entry:
     br label %loop_cond

   loop_cond:
     %i.phi = phi i64 [ 0, %entry ], [ %i.next, %loop_body ]
     %result.phi = phi i64 [ 0, %entry ], [ %call_result, %loop_body ]
     ; ... no memory operations!
   }
   ```

**Expected impact:** 2-3x speedup across all benchmarks

---

## Implementation Roadmap

### Phase 1: Critical Fixes (1-2 weeks) - Target: Match Rust on most benchmarks

**Week 1:**

- [x] Already have: O3 optimization, CPU auto-detection ✅
- [ ] Fix loop variables to use SSA/PHI nodes (Priority 1)
- [ ] Move top-level vars to stack (Priority 3)

**Week 2:**

- [ ] Inline primitive arithmetic (Priority 2)
- [ ] Add constant folding for compile-time strings
- [ ] Add inlining hints for small functions

**Expected results after Phase 1:**

| Benchmark | Current | Target | Speedup |
| --------- | ------- | ------ | ------- |
| loops     | 111.3ms | ~2ms   | 55x     |
| strings   | 30.3ms  | ~3ms   | 10x     |
| math      | 26.4ms  | ~4ms   | 6.5x    |
| dicts     | 19.5ms  | ~6ms   | 3x      |
| fib       | 205.9ms | ~180ms | 1.1x    |

**Overall:** Tea should match or beat Rust on 4/5 benchmarks.

---

### Phase 2: Advanced Optimizations (2-4 weeks) - Target: Beat Rust by 20-50%

- [ ] Stack-allocate small collections (strings < 24 bytes, lists < 8 elements)
- [ ] Tail call optimization for deep recursion
- [ ] Specialize hot paths (inline `len()`, `get()`, etc.)
- [ ] Add loop unrolling and vectorization hints

---

### Phase 3: World-Class Performance (1-2 months) - Target: Best-in-class

- [ ] Link-Time Optimization (LTO)
- [ ] Profile-Guided Optimization (PGO)
- [ ] Custom memory allocators (arena, bump)
- [ ] SIMD auto-vectorization

---

## Code Locations to Modify

1. **Loop variable handling:** `tea-compiler/src/aot/mod.rs:1462-1520`
   - Modify `compile_loop_statement` to use PHI nodes
   - Check mutability before allocating stack space

2. **Local variable allocation:** `tea-compiler/src/aot/mod.rs:480-490` (LocalVariable struct)
   - Already tracks `mutable` flag
   - Use `value` field for immutable, `pointer` field for mutable

3. **Global variable handling:** `tea-compiler/src/aot/mod.rs:1787-1810`
   - Modify `collect_globals` to emit stack allocations in main instead
   - Or use SSA values for const globals

4. **Binary operators:** (Already correct for integers)
   - Ensure no unnecessary boxing/unboxing
   - Keep arithmetic in SSA form

5. **Function parameters:** `tea-compiler/src/aot/mod.rs:1152-1237`
   - Use `find_mutated_parameters` (already implemented!)
   - Don't allocate stack for immutable parameters

---

## Validation Plan

After each optimization:

1. **Run full benchmark suite:**

   ```bash
   ./scripts/bench.sh all
   ```

2. **Verify correctness:**

   ```bash
   cargo test --workspace
   ./scripts/e2e.sh
   ```

3. **Inspect generated LLVM IR:**

   ```bash
   cargo run -p tea-cli -- build benchmarks/tea/loops.tea --emit llvm-ir -o /tmp/loops_ir
   # Check for:
   # - No unnecessary alloca/load/store
   # - PHI nodes in loops
   # - Function attributes (nounwind, willreturn)
   ```

4. **Compare against Rust:**
   ```bash
   hyperfine --warmup 3 \
     bin/loops_aot \
     bin/loops_rust
   ```

---

## Success Metrics

**Minimum Acceptable Performance (MVP):**

- All benchmarks within **2x of Rust** performance

**Target Performance (V1):**

- All benchmarks within **1.5x of Rust** or faster
- At least 2 benchmarks **faster than Rust**

**Stretch Goal (V2):**

- All benchmarks **match or beat Rust**
- 20-50% faster than Rust on specific workloads

---

## Conclusion

The Tea AOT compiler has **excellent foundations** (proven by the fib benchmark being only 1.17x slower than Rust). The performance gaps are caused by **specific, fixable issues**:

1. ✅ **Optimization level:** Already using O3 + native CPU
2. ❌ **Memory vs registers:** Need to use SSA/PHI nodes for loop variables
3. ❌ **Global vs stack:** Need to move top-level vars to stack
4. ❌ **FFI overhead:** Need to inline primitive operations

**These are all code generation issues, not fundamental limitations.** With the fixes outlined above, Tea should achieve Rust-level performance within 2-4 weeks of focused work.

The fibonacci benchmark proves the compiler can already generate efficient code when the patterns are right. We just need to apply those patterns more broadly.

---

## Next Steps

**Immediate action items:**

1. Start with **Priority 1** (loop optimization) - biggest impact, moderate complexity
2. Add **integration tests** for each optimization to prevent regressions
3. Set up **continuous benchmarking** to track progress
4. Consider creating **targeted micro-benchmarks** for each pattern

Would you like me to implement any of these optimizations? I recommend starting with Priority 1 (loop variables) as it will have the biggest impact on the benchmark results.
