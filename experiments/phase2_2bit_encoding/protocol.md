# Phase 2: 2-Bit Encoding Experiments

**Date**: October 30, 2025
**Status**: Protocol Design
**Duration**: Estimated 2-3 days

---

## Executive Summary

Phase 2 explores the **encoding dimension** of the sequence/hardware performance space by comparing ASCII (1 byte per base) vs 2-bit encoding (4 bases per byte).

**Critical Hypothesis**: Reverse complement showed only **1× NEON speedup on ASCII** (excluded as outlier from N=10 analysis), but BioMetal achieved **98× speedup with 2-bit encoding**. This 98× difference suggests encoding is a major optimization dimension.

**Goal**: Systematically validate if 2-bit encoding follows complexity-based patterns, or if it's operation-specific.

---

## Background from BioMetal

### Validated 2-Bit Results

From BioMetal development (January-October 2025):

**Reverse complement** (complexity 0.72):
- ASCII + NEON: ~1× speedup (encoding-limited, branches per base)
- 2-bit + NEON: **98× speedup** (bitwise operations, 64 bases per NEON register)
- **Encoding impact**: 98× difference!

**Base counting** (complexity 0.39):
- ASCII + NEON: 16-65× speedup (validated in ASBB N=10)
- 2-bit + NEON: Expected ~20-80× (modest improvement from 4× data density)
- **Encoding impact**: ~1.2-1.5× expected

**GC content** (complexity 0.315):
- ASCII + NEON: 14-35× speedup (validated in ASBB N=10)
- 2-bit + NEON: Expected ~18-45× (modest improvement)
- **Encoding impact**: ~1.2-1.3× expected

### Key Insight

**2-bit encoding benefits are operation-specific**:
- **Transform operations** (reverse complement, complement): Dramatic (50-98×)
- **Counting operations** (base counting, GC content): Modest (1.2-1.5×)
- **Filtering operations** (quality filter): None (quality scores are ASCII-only)

---

## Research Questions

1. **Does 2-bit encoding follow complexity patterns?**
   - Or is it orthogonal (operation-specific regardless of complexity)?

2. **Which operation categories benefit most from 2-bit?**
   - Transform > Counting > Filtering?

3. **Does 2-bit change the NEON lower bound?**
   - Current: complexity <0.25 → NEON 1.0× on ASCII
   - Hypothesis: Same lower bound applies (encoding doesn't fix inherently simple ops)

4. **Can we predict 2-bit benefit from complexity score?**
   - Or do we need a separate "encoding sensitivity" metric?

5. **What's the overhead of encoding/decoding?**
   - If we must convert ASCII → 2-bit → process → ASCII, is it worth it?

---

## Experimental Design

### Operations to Test (Priority Order)

#### Tier 1: Expected Dramatic Benefit

1. **Reverse complement** (N=11, complexity 0.72)
   - ASCII baseline: 1× NEON (from excluded data)
   - 2-bit target: 98× NEON (BioMetal validated)
   - **Critical validation**: Does ASBB replicate BioMetal's 98× result?

2. **Complement** (N=12, new operation, complexity ~0.65)
   - ASCII expected: ~1-2× NEON (similar to reverse complement)
   - 2-bit expected: ~50-80× NEON (bitwise XOR)
   - **Tests if pattern generalizes** to other transform operations

#### Tier 2: Expected Modest Benefit

3. **Base counting** (N=1, complexity 0.39)
   - ASCII baseline: 16-65× NEON (validated)
   - 2-bit expected: 20-80× NEON (4× density helps cache)
   - **Tests counting category** encoding sensitivity

4. **GC content** (N=2, complexity 0.315)
   - ASCII baseline: 14-35× NEON (validated)
   - 2-bit expected: 18-45× NEON (modest improvement)
   - **Validates counting pattern** across operations

5. **AT content** (N=7, complexity 0.35)
   - ASCII baseline: 12-45× NEON (validated)
   - 2-bit expected: 15-55× NEON (similar to GC)
   - **Third counting operation** for pattern confidence

#### Tier 3: Expected No Benefit (Control Group)

6. **Quality filter** (N=8, complexity 0.55)
   - ASCII baseline: 1.11-1.44× NEON (validated)
   - 2-bit: **Not applicable** (quality scores are ASCII-only)
   - **Negative control**: Confirms 2-bit doesn't help quality-dependent ops

7. **Length filter** (N=9, complexity 0.25)
   - ASCII baseline: 1.0× NEON (validated)
   - 2-bit expected: 1.0× NEON (sequence length metadata unchanged)
   - **Negative control**: Confirms 2-bit doesn't fix inherently simple ops

8. **Sequence length** (N=6, complexity 0.20)
   - ASCII baseline: 0.99-1.12× NEON (validated)
   - 2-bit expected: ~1.0× NEON (metadata operation)
   - **Lower bound control**: Validates encoding doesn't change lower bound

### Experimental Matrix

**For each applicable operation**:

| Scale | Encoding | NEON | Parallel | Combined | Experiments |
|-------|----------|------|----------|----------|-------------|
| tiny (100) | ASCII | ✓ | ✓ | ✓ | 3 |
| tiny (100) | 2-bit | ✓ | ✓ | ✓ | 3 |
| small (1K) | ASCII | ✓ | ✓ | ✓ | 3 |
| small (1K) | 2-bit | ✓ | ✓ | ✓ | 3 |
| ... | ... | ... | ... | ... | ... |
| huge (10M) | 2-bit | ✓ | ✓ | ✓ | 3 |

**Total per operation**: 6 scales × 2 encodings × 3 configs = **36 experiments**

**Total Phase 2**:
- 5 applicable operations (reverse complement, complement, base counting, GC content, AT content)
- 3 control operations (quality filter skipped, length ops for validation)
- **5 × 36 = 180 experiments** (main tier)
- **2 × 36 = 72 experiments** (control tier)
- **Grand total: ~250 experiments** (fully automated)

---

## Implementation Strategy

### Step 1: Integrate 2-Bit Encoding (Day 1 Morning)

**Source**: BioMetal `biometal-core/src/bitseq.rs`

**Core type**:
```rust
/// 2-bit DNA encoding (4 bases per byte)
pub struct BitSeq {
    data: Vec<u8>,      // Packed 2-bit representation
    length: usize,      // Number of bases (not bytes!)
}

impl BitSeq {
    /// Encode ASCII to 2-bit: A=00, C=01, G=10, T=11
    pub fn from_ascii(seq: &[u8]) -> Self;

    /// Decode 2-bit to ASCII
    pub fn to_ascii(&self) -> Vec<u8>;

    /// Reverse complement (2-bit native)
    pub fn reverse_complement_2bit(&self) -> Self;
}
```

**Integration plan**:
1. Copy `bitseq.rs` to `crates/asbb-core/src/encoding/mod.rs`
2. Add encoding enum to `DataCharacteristics`:
   ```rust
   pub enum Encoding {
       ASCII,
       TwoBit,
   }
   ```
3. Update `SequenceRecord` to support both encodings:
   ```rust
   pub struct SequenceRecord {
       pub id: String,
       pub sequence: SequenceData,  // Enum: ASCII or TwoBit
       pub quality: Option<Vec<u8>>,
   }
   ```

### Step 2: Add 2-Bit Backends (Day 1 Afternoon)

**For each Tier 1-2 operation**:

Add `execute_2bit_neon()` method:
```rust
impl PrimitiveOperation for ReverseComplement {
    // Existing ASCII methods
    fn execute_naive(&self, data: &[SequenceRecord]) -> Result<Output>;
    fn execute_neon(&self, data: &[SequenceRecord]) -> Result<Output>;

    // New 2-bit method
    fn execute_2bit_neon(&self, data: &[SequenceRecord]) -> Result<Output> {
        // Convert ASCII → 2-bit (if needed)
        // Process with 2-bit NEON (64 bases per register!)
        // Convert 2-bit → ASCII (if needed for output)
    }
}
```

**Key decision**: Include encoding/decoding overhead in benchmarks
- **Scenario 1**: Pure 2-bit (data already encoded, output stays encoded)
- **Scenario 2**: Encoding overhead (ASCII → 2-bit → process → ASCII)
- **Both scenarios measured** to understand practical impact

### Step 3: Generate 2-Bit Datasets (Day 1 Evening)

**Approach**: Pre-encode existing datasets to 2-bit

```bash
# For each scale: tiny, small, medium, large, vlarge, huge
cargo run --release --bin encode-to-2bit \
  datasets/tiny_100_150bp.fq \
  datasets/tiny_100_150bp.2bit
```

**Files**:
- `datasets/tiny_100_150bp.fq` (ASCII, existing)
- `datasets/tiny_100_150bp.2bit` (2-bit, new)
- Both represent identical sequences

### Step 4: Implement Priority Operations (Day 2)

**Priority 1: Reverse complement** (critical validation)
```rust
// crates/asbb-ops/src/reverse_complement.rs (UPDATE)

#[cfg(target_arch = "aarch64")]
fn reverse_complement_2bit_neon(bitseq: &BitSeq) -> BitSeq {
    use std::arch::aarch64::*;

    // BioMetal validated this achieves 98× speedup
    // 64 bases per NEON register (vs 16 for ASCII)
    // Complement: XOR with 0b10101010... (flips both bits)
    // Reverse: Use vrev64_u8 + vext

    // (Borrow implementation from BioMetal)
}
```

**Priority 2: Complement** (new operation)
```rust
// crates/asbb-ops/src/complement.rs (NEW)

pub struct Complement;

#[cfg(target_arch = "aarch64")]
fn complement_2bit_neon(bitseq: &BitSeq) -> BitSeq {
    // Even simpler than reverse complement (no reversal)
    // XOR each byte with 0b10101010...
    // Expected: ~80× speedup (similar to reverse complement)
}
```

**Priority 3-5: Base counting, GC content, AT content** (update existing)
- Add `execute_2bit_neon()` to existing implementations
- Expected modest improvement (1.2-1.5×)

### Step 5: Run Experiments (Day 2 Evening)

**Automated experiment harness**:
```rust
// crates/asbb-cli/src/pilot_encoding_comparison.rs (NEW)

const OPERATIONS: &[&str] = &[
    "reverse_complement",
    "complement",
    "base_counting",
    "gc_content",
    "at_content",
];

const SCALES: &[(&str, &str, &str)] = &[
    ("tiny", "datasets/tiny_100_150bp.fq", "datasets/tiny_100_150bp.2bit"),
    ("small", "datasets/small_1k_150bp.fq", "datasets/small_1k_150bp.2bit"),
    // ... 6 scales
];

fn main() -> Result<()> {
    for operation in OPERATIONS {
        for (scale_name, ascii_path, twobit_path) in SCALES {
            // Run 3 configs on ASCII
            benchmark_ascii(operation, ascii_path, scale_name)?;

            // Run 3 configs on 2-bit
            benchmark_2bit(operation, twobit_path, scale_name)?;
        }
    }
}
```

**Output**: CSV with encoding comparison
```csv
operation,complexity,encoding,scale,num_sequences,neon_speedup,parallel_speedup,combined_speedup
reverse_complement,0.72,ASCII,tiny,100,1.0,3.65,3.65
reverse_complement,0.72,TwoBit,tiny,100,98.0,3.65,357.0
base_counting,0.39,ASCII,tiny,100,53.0,0.23,0.21
base_counting,0.39,TwoBit,tiny,100,68.0,0.25,0.23
```

### Step 6: Analysis (Day 3 Morning)

**Python analysis script**:
```python
# analysis/encoding_comparison.py

import pandas as pd

df = pd.read_csv('analysis/encoding_data.csv')

# For each operation, calculate encoding benefit
df['encoding_benefit'] = df.groupby(['operation', 'scale']).apply(
    lambda g: g[g.encoding == 'TwoBit']['neon_speedup'].values[0] /
              g[g.encoding == 'ASCII']['neon_speedup'].values[0]
)

# Plot: Encoding benefit vs Complexity
plt.scatter(df['complexity'], df['encoding_benefit'])
plt.xlabel('Complexity Score')
plt.ylabel('2-bit Speedup / ASCII Speedup')
plt.title('Encoding Benefit vs Operation Complexity')

# Expected pattern:
# - High complexity (0.65-0.72): 50-98× benefit (transform ops)
# - Medium complexity (0.35-0.40): 1.2-1.5× benefit (counting ops)
# - Low complexity (<0.25): ~1.0× benefit (no change)
```

**Key metric**: **Encoding benefit ratio**
```
Encoding Benefit = (2-bit NEON speedup) / (ASCII NEON speedup)
```

**Expected results**:
- Reverse complement: 98× / 1× = **98× encoding benefit**
- Complement: ~80× / ~2× = **40× encoding benefit**
- Base counting: 68× / 53× = **1.3× encoding benefit**
- GC content: 18× / 14× = **1.3× encoding benefit**
- Length filter: 1× / 1× = **1.0× encoding benefit** (no change)

### Step 7: Document Findings (Day 3 Afternoon)

**Deliverable**: `results/phase2_encoding_findings.md`

**Key sections**:
1. Encoding benefit by operation category
2. Transform operations: 40-98× benefit (operation-specific, not complexity-driven)
3. Counting operations: 1.2-1.5× benefit (modest, universal)
4. Filtering operations: 1.0× benefit (encoding irrelevant)
5. Overhead analysis: Encoding/decoding cost vs processing benefit
6. Updated optimization rules: When to use 2-bit encoding

---

## Success Criteria

### Technical Validation

- [ ] Replicate BioMetal's 98× reverse complement speedup in ASBB framework
- [ ] Demonstrate encoding benefit is operation-specific (not complexity-driven)
- [ ] Quantify encoding/decoding overhead (pure vs with conversion)
- [ ] Validate control operations show no encoding benefit

### Scientific Contribution

- [ ] Establish formal decision rules for encoding choice
- [ ] Document operation categories by encoding sensitivity
- [ ] Quantify when encoding overhead is worthwhile
- [ ] Extend complexity metric or create "encoding sensitivity" dimension

### Integration

- [ ] Update `asbb-rules` crate with encoding decision logic
- [ ] Provide BioMetal integration examples
- [ ] Document encoding trade-offs for community

---

## Expected Challenges

### Challenge 1: Encoding/Decoding Overhead

**Problem**: If data is ASCII, converting to 2-bit adds overhead

**Approach**: Measure both scenarios:
1. **Pure 2-bit**: Data pre-encoded (streaming pipelines can stay 2-bit)
2. **With conversion**: ASCII → 2-bit → process → ASCII (one-off operations)

**Decision rule**: Use 2-bit if `(encoding benefit) × (processing time) > (conversion time)`

### Challenge 2: Quality Score Handling

**Problem**: FASTQ has both sequence (can be 2-bit) and quality (must be ASCII)

**Approach**: Hybrid representation
```rust
pub struct SequenceRecord {
    pub id: String,
    pub sequence: SequenceData,  // Can be ASCII or 2-bit
    pub quality: Option<Vec<u8>>, // Always ASCII (Phred scores)
}
```

**Implication**: Quality-dependent operations (quality filter, quality aggregation) cannot use 2-bit for quality part

### Challenge 3: Complexity Metric May Not Apply

**Problem**: Encoding benefit seems operation-specific, not complexity-driven

**Approach**: If complexity doesn't predict encoding benefit, create separate "encoding sensitivity" metric

**Potential metric dimensions**:
1. **Bitwise operations**: High (complement, reverse complement)
2. **Comparison operations**: Low (counting, filtering)
3. **Data density benefit**: Cache-bound operations get 4× data density
4. **Branch sensitivity**: Branchy operations see less benefit

---

## Timeline

**Day 1** (6-8 hours):
- Morning: Integrate 2-bit encoding infrastructure (2-3 hours)
- Afternoon: Add 2-bit backends to priority operations (3-4 hours)
- Evening: Generate 2-bit datasets (1 hour automated)

**Day 2** (6-8 hours):
- Morning: Implement reverse complement + complement 2-bit (3-4 hours)
- Afternoon: Update counting operations (base/GC/AT) for 2-bit (2-3 hours)
- Evening: Run automated experiments (~250 tests, 2-3 hours)

**Day 3** (4-6 hours):
- Morning: Analysis and visualization (2-3 hours)
- Afternoon: Documentation and findings report (2-3 hours)

**Total**: 2-3 days (16-22 hours)

---

## Deliverables

### Code

- [ ] `crates/asbb-core/src/encoding/mod.rs` - 2-bit encoding infrastructure
- [ ] `crates/asbb-ops/src/reverse_complement.rs` - Updated with 2-bit backend
- [ ] `crates/asbb-ops/src/complement.rs` - NEW operation
- [ ] `crates/asbb-ops/src/base_counting.rs` - Updated with 2-bit backend
- [ ] `crates/asbb-ops/src/gc_content.rs` - Updated with 2-bit backend
- [ ] `crates/asbb-ops/src/at_content.rs` - Updated with 2-bit backend
- [ ] `crates/asbb-cli/src/pilot_encoding_comparison.rs` - Encoding experiment harness

### Data

- [ ] `datasets/*.2bit` - 6 scales of pre-encoded 2-bit datasets
- [ ] `analysis/encoding_data.csv` - Full encoding comparison data (~250 rows)
- [ ] `analysis/encoding_benefit_by_operation.png` - Visualization
- [ ] `analysis/encoding_vs_complexity.png` - Encoding benefit vs complexity

### Documentation

- [ ] `results/phase2_encoding_findings.md` - Comprehensive findings report
- [ ] `experiments/phase2_2bit_encoding/protocol.md` - This document
- [ ] Updated `NEXT_STEPS.md` with Phase 2 completion status
- [ ] Updated `asbb-rules` with encoding decision logic

---

## Integration with N=10 Regression

**Key question**: Does 2-bit encoding change the complexity-speedup relationship?

**Approach**: Run regression with encoding as additional predictor
```python
# Model 1: ASCII only (current)
speedup ~ complexity + log10(scale)

# Model 2: With encoding
speedup ~ complexity + log10(scale) + encoding + complexity×encoding

# Test if interaction term is significant
# If yes: Encoding benefit depends on complexity
# If no: Encoding benefit is operation-specific (independent of complexity)
```

**Expected**: Interaction term NOT significant → Encoding is orthogonal dimension

**Implication**: Need separate decision rules for encoding (can't predict from complexity alone)

---

## Publication Impact

### Novel Contribution

**First systematic study** of encoding impact on bioinformatics performance:
- Quantifies operation-specific encoding benefits (98× for transforms, 1.3× for counting)
- Establishes formal decision rules for encoding choice
- Validates encoding/decoding overhead trade-offs

### Broader Applicability

**Findings apply beyond Apple Silicon**:
- 2-bit encoding benefits any SIMD architecture (x86 AVX, ARM NEON)
- Decision rules portable to other tools
- Encoding sensitivity metric generalizable

### Community Value

**Enables informed optimization**:
- "Should I implement 2-bit for my tool?" → Decision flowchart
- "Which operations benefit most?" → Operation category mapping
- "Is conversion overhead worth it?" → Quantified trade-off analysis

---

**Status**: Protocol design complete, ready for implementation
**Next**: Begin Day 1 - Integrate 2-bit encoding infrastructure
