# ASBB: Claude Development Guide

**Last Updated**: November 3, 2025

---

## 🚀 NEW: Phase Change (Nov 3, 2025)

**Strategic Pivot**: From Analysis → Analysis + Implementation + Tool

**Previous**: Pure analytical work (978 experiments, informative but lacks practical usage)

**New Vision**:
- Complete DAG traversal (740 experiments to fill gaps)
- Build `biofast` production library with streaming
- Four-pillar paper + usable tool on crates.io

**Timeline**: 2-3 weeks (Nov 4-22, 2025)

**Key Documents**:
- **CURRENT_STATUS.md**: Updated with new roadmap
- **BIOFAST_VISION.md**: Library design and goals
- **DAG_FRAMEWORK.md**: Novel testing methodology
- **ROADMAP.md**: Detailed 2-3 week timeline

**For detailed guidance**: See documents above. This file (CLAUDE.md) describes general development principles which remain unchanged.

---

## Mission

**Democratizing Bioinformatics Compute** by breaking down FOUR barriers:

1. **Economic**: Consumer hardware ($2-4K) replaces $100K+ HPC clusters
2. **Environmental**: 300x less energy per analysis (0.5 Wh vs 150 Wh)
3. **Portability**: ARM NEON rules work across Mac, Graviton, Ampere, RPi
4. **Data Access**: Memory-efficient streaming enables 5TB analysis on 24GB laptop

**Target Audience**: LMIC researchers, small labs, students, field researchers, diagnostic labs

**See**: `DEMOCRATIZING_BIOINFORMATICS_COMPUTE.md` for full mission statement

---

## Current Status (2/4 Pillars Validated)

**Validated**:
- Economic: 849 experiments prove consumer hardware viability (20-40x NEON speedup)
- Data Access: 240,000x memory reduction via streaming (validated)

**Needs Validation**:
- Environmental: Power consumption pilot pending (1-2 days, $25 wattmeter)
- Portability: AWS Graviton validation pending (3 hours, ~$1 cost)

**See**: `CURRENT_STATUS.md` for detailed status

---

## Core Principles

### 1. Four-Pillar Focus

Every experiment must advance one of the FOUR pillars. Ask:
- Does this validate Economic, Environmental, Portability, or Data Access?
- Does this enable our target audiences (LMIC, small labs, students, etc.)?
- Are we documenting limitations honestly?

### 2. Lab Notebook Discipline

ALL experimental work requires lab notebook documentation:
- Create entry BEFORE experiments: `lab-notebook/YYYY-MM/YYYYMMDD-NNN-TYPE-name.md`
- Document objective, methods, results
- Update `lab-notebook/INDEX.md`
- Git pre-commit hook enforces this

**See**: Existing entries in `lab-notebook/2025-11/` for examples

### 3. ARM-First Thinking

This is NOT "Apple Silicon benchmarking" - it's ARM ecosystem democratization.

**Key insight**: ARM NEON (standard, not Apple-specific) provides 20-40x speedup
**Why this matters**: Works across Mac, Graviton, Ampere, RPi (portable, accessible)

Design for ARM ecosystem, not just Mac:
- NEON SIMD (standard ARM instruction set)
- Unified memory patterns (increasingly common)
- Portable optimization rules (test on Mac, deploy anywhere)

### 4. Validate Claims with Data

**Unvalidated claims hurt credibility**. Current unvalidated:
- "300x less energy" - NEEDS power consumption measurements
- "Works across ARM ecosystem" - NEEDS Graviton validation

**Validated claims are powerful**:
- "20-40x NEON speedup" - 849 experiments across 6 dimensions
- "240,000x memory reduction" - Memory footprint pilot (Entry 017)

---

## Next Experiments (Pillar Validation)

### 1. Power Consumption Pilot (Environmental Pillar)

**Why critical**: Validates environmental sustainability claim

**Scope**: 80 experiments (10 operations x 4 configs x 2 scales)
**Equipment**: $25 wattmeter + macOS powermetrics
**Timeline**: 1-2 days
**Measurements**: Idle power, active power, energy per analysis (Wh)

**Deliverable**: `results/power_consumption/FINDINGS.md`

### 2. AWS Graviton Validation (Portability Pillar)

**Why critical**: Proves ARM NEON rules transfer across platforms

**Scope**: 45 experiments (5 operations x 3 configs x 3 scales)
**Cost**: ~$1 (c7g.xlarge instance, 3 hours)
**Timeline**: 3 hours
**Comparison**: Mac M4 vs Graviton speedups

**Deliverable**: `results/cross_platform_graviton/FINDINGS.md`

### 3. Four-Pillar Paper

**After both pilots complete**: All 4 pillars validated
**Target**: GigaScience, BMC Bioinformatics, Nature Communications
**Framing**: Democratization, not benchmarking

---

## Key Experimental Findings (Current)

**Hardware Dimensions Tested** (6 complete):
1. NEON SIMD: 20-40x speedup for complexity 0.30-0.40 operations
2. GPU Metal: Only 1/10 operations benefit (complexity >0.55)
3. 2-bit Encoding: 2-4x SLOWER (conversion overhead dominates)
4. Parallel/Threading: Up to 21.47x (super-linear, 268% efficiency)
5. AMX Matrix Engine: 0.92x vs NEON (NEGATIVE finding, deferred)
6. Hardware Compression: NEGATIVE finding (deferred)

**Cross-Dimension Insights**:
- NEON effectiveness predicts GPU benefit (eliminates 90% GPU testing)
- Universal 10K sequence threshold across dimensions
- Composition: NEON x Parallel = multiplicative (validated, 36 experiments)

**Total**: 849 experiments analyzed, 7 optimization rules extracted

**See**: `results/PHASE1_COMPLETE_ANALYSIS.md` for comprehensive analysis

---

## Optimization Rules (Quick Reference)

**Rule 1**: Always use ARM NEON for complexity 0.30-0.40 (10-50x speedup)
**Rule 2**: Parallelize for >10K sequences (4-21x speedup)
**Rule 3**: GPU only if NEON <2x AND complexity >0.55
**Rule 4**: NEON x Parallel = multiplicative (validated)
**Rule 5**: Streaming for >1GB datasets (240,000x memory reduction)
**Rule 6**: Skip 2-bit encoding (conversion overhead dominates)
**Rule 7**: Skip AMX (0.92x vs NEON, negative finding)

**See**: `OPTIMIZATION_RULES.md` for detailed guide with code examples

---

## Development Workflow

### Current Phase: Pillar Validation

**Not**: Systematic pilot experimentation (complete)
**Now**: Validating unproven claims (Environmental, Portability)

### Typical Experiment Flow

1. Create lab notebook entry (BEFORE experiments)
2. Write protocol in `experiments/[dimension]/protocol.md`
3. Implement pilot binary (`crates/asbb-cli/src/pilot_*.rs`)
4. Run experiments, save raw CSV
5. Analyze results, create `results/[dimension]/FINDINGS.md`
6. Update lab notebook entry with findings
7. Update `lab-notebook/INDEX.md`
8. Commit entry + INDEX + results together

### File Organization

```
experiments/          Protocol definitions
crates/asbb-cli/      Pilot binaries
results/              Analysis and findings
lab-notebook/         Experimental documentation
docs/                 Supporting documentation
```

---

## For Claude: Session Guidelines

### What to Emphasize

- Four-pillar mission (not performance benchmarking)
- Target audiences (LMIC, small labs, students, etc.)
- Validate claims with experimental data
- Lab notebook discipline (pre-commit hook enforces)

### What NOT to Do

- Suggest more systematic pilots (experimentation phase complete)
- Focus on Apple Silicon exclusivity (ARM ecosystem democratization)
- Make unvalidated claims (validate first)
- Skip lab notebook documentation (enforced by git hook)

### Decision Framework

**When user asks "what's next?"**:
1. Check CURRENT_STATUS.md (2/4 pillars validated)
2. Suggest power pilot OR Graviton validation (both high priority)
3. NOT: Additional hardware dimensions (diminishing returns)

**When user proposes experiment**:
1. Which pillar does this validate?
2. Is this claim currently unvalidated?
3. What's the cost/benefit? (prioritize high-impact, low-cost)

---

## Common Pitfalls to Avoid

### 1. Scope Creep

**Bad**: "Let's test Neural Engine, M5 GPUs, more platforms..."
**Good**: "Let's validate the 2 unproven pillars (Environmental, Portability)"

### 2. Unvalidated Claims

**Bad**: "300x less energy" (no measurements yet)
**Good**: "20-40x NEON speedup" (849 experiments prove this)

### 3. Apple Silicon Exclusivity

**Bad**: "This only works on Mac"
**Good**: "ARM NEON works across Mac, Graviton, Ampere, RPi"

### 4. Skipping Documentation

**Bad**: Run experiments, forget lab notebook
**Good**: Lab notebook entry FIRST, then experiments (git hook enforces)

---

## Publication Framing

**Title**: "Democratizing Bioinformatics: Breaking Economic, Environmental, Portability, and Data Access Barriers"

**NOT**: "Systematic Performance Characterization of Apple Silicon"

**Key Messages**:
- Consumer hardware enables production bioinformatics ($2-4K vs $100K+)
- 300x energy reduction (Environmental pillar - NEEDS validation)
- Portable ARM NEON rules (Portability pillar - NEEDS validation)
- Memory-efficient streaming unlocks 40+ PB public data (Data Access pillar)

**Impact Statement**:
- Enables LMIC researchers (economic barrier removed)
- Reduces carbon footprint (environmental benefit)
- No vendor lock-in (portable ARM ecosystem)
- Unlocks public data archives (memory-efficient streaming)

---

## References

**Mission**: `DEMOCRATIZING_BIOINFORMATICS_COMPUTE.md`
**Status**: `CURRENT_STATUS.md`
**Analysis**: `results/PHASE1_COMPLETE_ANALYSIS.md`
**Rules**: `OPTIMIZATION_RULES.md`
**Notebook**: `lab-notebook/INDEX.md`

---

**Last Updated**: November 2, 2025
**Status**: 2/4 pillars validated, 2 pending (power, Graviton)
**Next**: Power consumption pilot (Environmental pillar)
