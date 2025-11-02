# AMX Implementation Research Summary

## Background

AMX (Apple Matrix Extension) is Apple's matrix coprocessor:
- Available on M1 and later
- 512-bit wide matrix operations
- Used internally by Core ML, Accelerate framework
- **Not officially documented** by Apple (undocumented intrinsics)

## Implementation Options for Rust

### Option 1: Accelerate Framework (RECOMMENDED)
**Approach**: Use Apple's Accelerate framework via FFI
- **Pros**:
  - Official Apple framework
  - Automatically uses AMX when beneficial
  - Stable, well-tested
  - Rust bindings exist
- **Cons**:
  - Indirect control (framework decides when to use AMX)
  - May not use AMX for all operations

**Implementation**:
```rust
// Use accelerate-src crate
use accelerate_src::*;
// Matrix operations automatically use AMX
```

### Option 2: Direct AMX Intrinsics (RISKY)
**Approach**: Use undocumented AMX intrinsics
- **Pros**:
  - Direct control over AMX
  - Maximum performance potential
- **Cons**:
  - Undocumented (may change)
  - Requires assembly or C interop
  - No stability guarantees
  - Complex to implement

### Option 3: Hybrid (PRACTICAL)
**Approach**: Use Accelerate for AMX, measure vs NEON
- Implement operations using Accelerate BLAS/LAPACK
- Compare performance with NEON implementations
- **This tests AMX benefit without diving into unstable internals**

## Recommended Approach for AMX Pilot

**Use Accelerate framework** (Option 1/3):

1. **For edit distance**: Implement using Accelerate matrix operations
2. **For quality statistics**: Use vDSP (Accelerate vector/signal processing)
3. **Measurement**: Compare Accelerate (uses AMX) vs NEON (explicit SIMD)

**Key insight**: We don't need direct AMX intrinsics to test AMX benefit. Accelerate provides stable access.

## Next Steps

1. Add `accelerate-src` dependency to Cargo.toml
2. Implement edit_distance using Accelerate matrix ops
3. Implement quality_statistics using vDSP
4. Run experiments: Accelerate (AMX) vs NEON vs baseline
5. Measure if AMX provides speedup

