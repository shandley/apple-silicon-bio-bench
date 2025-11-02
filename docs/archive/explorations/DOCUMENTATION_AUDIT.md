# ASBB Documentation Audit & Restructuring Plan

**Date**: November 1, 2025
**Audit Scope**: Complete project documentation structure
**Status**: 🚨 **CRITICAL** - Significant fragmentation identified

---

## Executive Summary

**PROBLEM**: Documentation has fragmented across multiple locations with inconsistent structures, creating a coherent narrative challenge for publication.

**ROOT CAUSE**: Lab notebook system established but abandoned after Entry 008 (~Oct 30). Subsequent major work (GPU, encoding, parallel dimensions) documented ad-hoc in `/results/` directory.

**IMPACT**:
- 600+ experiments (GPU, encoding, parallel) not in lab notebook
- Findings scattered across 40+ markdown files
- No single source of truth
- Publication narrative unclear

**RECOMMENDATION**: Immediate restructuring required before next dimension pilot.

---

## Current State Analysis

### Documentation Locations (5 distinct areas)

#### 1. **Root Directory** (8 files) - ⚠️ **TOO MANY**

**Strategic/Permanent** (should stay):
- `CLAUDE.md` - AI development guide ✅
- `METHODOLOGY.md` - Experimental design ✅
- `README.md` - Project overview ✅

**Ad-hoc/Session docs** (should move):
- `PARALLEL_PILOT_READY.md` - Session instruction (→ archive or delete)
- `PHASE1_DAY1_SUMMARY.md` - Early summary (→ lab-notebook)
- `PHASE1_DAY2_SUMMARY.md` - Early summary (→ lab-notebook)
- `NEXT_STEPS.md` - Current status (→ lab-notebook/STATUS.md)
- `SESSION_WRAPUP.md` - Session summary (→ archive)

**Assessment**: Root directory cluttered with session artifacts.

---

#### 2. **Lab Notebook** (`/lab-notebook/`) - ⚠️ **ABANDONED**

**What's there**:
- `INDEX.md` - Last updated Oct 30 (stale)
- Only 3 entries in `2025-10/`:
  - `001-TEST-hook-validation.md`
  - `007-EXPERIMENT-quality-aggregation-n4.md`
  - `008-EXPERIMENT-n-content-n5.md`

**What's MISSING** (work done AFTER Entry 008):
- Entry 009: GPU dimension pilot (32 experiments) ❌
- Entry 010: 2-bit encoding dimension (72 experiments) ❌
- Entry 011: Parallel dimension pilot (600 experiments) ❌
- Entry 012: Regression analysis (complexity-speedup) ❌
- Entry 013: N=10 validation ❌

**Assessment**: System exists but not used consistently. **This is the core problem.**

---

#### 3. **Results Directory** (`/results/`) - ⚠️ **DUMPING GROUND**

**20+ files**, mix of styles:

**Phase 1 summaries**:
- `phase1_all_dimensions_complete.md` (N=10 validation)
- `phase1_gpu_dimension_complete.md` (GPU findings)
- `phase1_parallel_dimension_complete.md` (Parallel findings) ← **JUST CREATED**
- `phase1_gpu_pilot_base_counting.md`
- `phase1_gpu_comparison_analysis.md`

**Phase 2 summaries**:
- `phase2_encoding_complete_results.md`
- `phase2_encoding_initial_results.md`

**Individual operation findings** (early work):
- `gc_content_findings.md`
- `n_content_n5_findings.md`
- `quality_aggregation_n4_findings.md`
- `pilot_multiscale_findings.md`
- `revcomp_findings_2bit_checkpoint.md`

**Assessment docs** (future dimensions):
- `phase1_amx_assessment.md`
- `phase1_neural_engine_assessment.md`
- `phase1_hardware_compression_assessment.md`
- `phase1_gcd_qos_assessment.md`

**Meta docs**:
- `72_experiments_reflection.md`
- `72_experiments_reflection_with_external_validation.md`
- `combined_optimization_findings.md`
- `complexity_metric_findings.md`
- `n10_final_validation.md`

**Assessment**: No clear organization. Mix of detailed findings, summaries, and assessments.

---

#### 4. **Experiments Directory** (`/experiments/`) - ✅ **GOOD STRUCTURE**

**Protocols** (versioned, reproducible):
- `phase1_gpu_dimension/protocol.md` ✅
- `phase1_parallel_dimension/protocol.md` ✅
- `phase1_parallel_dimension/README.md` ✅
- `phase2_2bit_encoding/protocol.md` ✅

**Assessment**: This is working well. Keep this structure.

---

#### 5. **Docs Directory** (`/docs/`) - ⚠️ **UNCLEAR PURPOSE**

**Strategic docs**:
- `COMPLEXITY_METRIC.md` - Methodology for complexity scoring
- `M5_INTEGRATION.md` - Future M5 hardware considerations

**Assessment**: Purpose unclear. Are these methodology docs? Future work? Should integrate into METHODOLOGY.md or lab-notebook.

---

## Fragmentation Issues

### Issue 1: Duplicate/Overlapping Content

**Example**: Base counting documented in multiple places:
1. `lab-notebook/2025-10/INDEX.md` - Entry 002 summary
2. `lab-notebook/raw-data/20251030-002/pilot_multiscale_findings.md` - Detailed analysis
3. `results/pilot_multiscale_findings.md` - Same detailed analysis (duplicate?)
4. `PHASE1_DAY1_SUMMARY.md` - High-level summary
5. `results/72_experiments_reflection.md` - Includes base counting

**Problem**: Which is the canonical source? For publication, where do we point?

### Issue 2: Lab Notebook Not Used for Recent Work

**Work NOT in lab notebook**:
- GPU dimension (32 experiments, Oct 31) - Only in `results/phase1_gpu_dimension_complete.md`
- 2-bit encoding (72 experiments, Oct 31) - Only in `results/phase2_encoding_complete_results.md`
- Parallel dimension (600 experiments, Oct 31) - Only in `results/phase1_parallel_dimension_complete.md`
- N=10 validation - Only in `results/n10_final_validation.md`

**Impact**: Lab notebook appears abandoned. INDEX.md last updated Oct 30, but we've done 3 major experiments since then!

### Issue 3: No Clear Publication Narrative

**For a paper, we need**:
- Clear chronology (what was tested when, why)
- Coherent story (building from simple to complex)
- Clear methodology (reproducible protocols)
- Formal results (tables, figures)
- Interpretation (what it means)

**Current state**: Fragmented across files with different formats/styles.

### Issue 4: Hook System Not Enforcing Structure

**Git hook exists** (`.claude/hooks/README.md`) but:
- Not being triggered (no entries since Oct 30)
- Not validating lab notebook updates
- Lab notebook INDEX.md not being updated

**Problem**: Infrastructure exists but not enforced/used.

---

## Root Cause Analysis

### Why Did This Happen?

**Timeline**:
1. **Oct 30**: Lab notebook established, Entries 001-008 created ✅
2. **Oct 30-31**: INDEX.md updated through Entry 008 ✅
3. **Oct 31**: GPU dimension completed → Documented in `/results/` ❌
4. **Oct 31**: 2-bit encoding completed → Documented in `/results/` ❌
5. **Oct 31**: Parallel dimension completed → Documented in `/results/` ❌
6. **Nov 1**: This audit ⚠️

**Pattern**: Once we started systematic dimension testing (GPU, encoding, parallel), we abandoned lab notebook in favor of standalone result documents.

**Hypothesis**:
- Lab notebook felt too granular for large experiments (600 tests)?
- Wanted comprehensive "complete" documents instead of log entries?
- Lost track of lab notebook system during intense experimentation?

---

## Proposed Solution: Three-Tier Documentation Architecture

### Tier 1: **Lab Notebook** (Chronological Record)

**Purpose**: Complete chronological record of ALL experimental work

**Structure**:
```
lab-notebook/
├── INDEX.md (master index, always up-to-date)
├── 2025-10/
│   ├── 20251030-001-TEST-hook-validation.md
│   ├── 20251030-007-EXPERIMENT-quality-aggregation-n4.md
│   ├── 20251030-008-EXPERIMENT-n-content-n5.md
│   ├── 20251031-009-EXPERIMENT-gpu-dimension.md ← ADD
│   ├── 20251031-010-EXPERIMENT-2bit-encoding.md ← ADD
│   └── 20251031-011-EXPERIMENT-parallel-dimension.md ← ADD
├── 2025-11/
│   └── (future entries)
└── raw-data/
    ├── 20251031-009/ (GPU raw data)
    ├── 20251031-010/ (encoding raw data)
    └── 20251031-011/ (parallel raw data)
```

**Entry format** (existing template works):
```markdown
---
title: "GPU Dimension Pilot - Complete"
date: 2025-10-31
type: EXPERIMENT
status: Complete
phase: 1
operation: base_counting, reverse_complement, quality_aggregation, complexity_score
---

## Objective
[Brief objective]

## Methods
[Reference to experiments/protocol.md]

## Results Summary
[High-level findings - 2-3 paragraphs]

## Key Findings
- Bullet list of discoveries

## Raw Data
- CSV: results/parallel_dimension_raw_YYYYMMDD.csv
- Analysis: results/phase1_gpu_dimension_complete.md

## References
- Entry 008 (previous)
- experiments/phase1_gpu_dimension/protocol.md
```

**Content**: Brief summaries + pointers to detailed analysis (Tier 2)

---

### Tier 2: **Results** (Detailed Findings)

**Purpose**: Comprehensive analysis documents (publication-ready)

**Structure**:
```
results/
├── phase1/
│   ├── neon_dimension_complete.md (consolidate early work)
│   ├── gpu_dimension_complete.md ✅
│   ├── parallel_dimension_complete.md ✅
│   ├── amx_dimension_complete.md (future)
│   ├── neural_engine_dimension_complete.md (future)
│   └── all_dimensions_synthesis.md (future)
├── phase2/
│   ├── encoding_dimension_complete.md ✅
│   └── encoding_gpu_interaction.md (future)
└── archive/
    └── (move early individual findings here)
```

**Content**: Detailed tables, analysis, visualizations, decision rules

**Format**: Publication-quality markdown (convert to LaTeX later)

---

### Tier 3: **Experiments** (Protocols)

**Purpose**: Reproducible experimental protocols

**Structure**: (already working well)
```
experiments/
├── phase1_neon_dimension/
│   └── protocol.md
├── phase1_gpu_dimension/
│   └── protocol.md
├── phase1_parallel_dimension/
│   ├── protocol.md
│   └── README.md
└── phase2_2bit_encoding/
    └── protocol.md
```

**Content**: How to run experiments (reproducible)

---

## Migration Plan

### Step 1: Backfill Lab Notebook (Priority: HIGH)

**Create missing entries**:
1. `20251031-009-EXPERIMENT-gpu-dimension.md`
2. `20251031-010-EXPERIMENT-2bit-encoding.md`
3. `20251031-011-EXPERIMENT-parallel-dimension.md`
4. `20251030-012-ANALYSIS-n10-validation.md`
5. `20251030-013-ANALYSIS-complexity-regression.md`

**Update INDEX.md**:
- Current total: 8 entries (claimed) but only 3 files
- Actual total should be: 13 entries (008 + 5 new)
- Update stats, timeline, references

**Timeline**: 2-3 hours

---

### Step 2: Reorganize Results (Priority: MEDIUM)

**Create subdirectories**:
```bash
mkdir -p results/phase1
mkdir -p results/phase2
mkdir -p results/archive
```

**Move files**:
```bash
# Phase 1 dimension complete docs (keep)
mv results/phase1_*_dimension_complete.md results/phase1/

# Phase 2 dimension complete docs (keep)
mv results/phase2_*.md results/phase2/

# Early individual findings (archive)
mv results/gc_content_findings.md results/archive/
mv results/n_content_n5_findings.md results/archive/
mv results/quality_aggregation_n4_findings.md results/archive/
mv results/pilot_multiscale_findings.md results/archive/
# etc.
```

**Timeline**: 1 hour

---

### Step 3: Clean Root Directory (Priority: MEDIUM)

**Archive session docs**:
```bash
mkdir -p archive/session-docs
mv PARALLEL_PILOT_READY.md archive/session-docs/
mv SESSION_WRAPUP.md archive/session-docs/
mv PHASE1_DAY1_SUMMARY.md archive/session-docs/
mv PHASE1_DAY2_SUMMARY.md archive/session-docs/
```

**Consolidate NEXT_STEPS**:
- Move to `lab-notebook/STATUS.md` (current status, always up-to-date)
- Or keep in root but enforce regular updates

**Timeline**: 30 minutes

---

### Step 4: Re-enable Git Hooks (Priority: HIGH)

**Test hook**:
```bash
./.claude/hooks/user-prompt-submit.sh
```

**Verify**:
- Hook outputs reminders
- Hook references correct documents
- Hook enforces lab notebook updates

**Update hooks** if needed to reference new structure.

**Timeline**: 1 hour

---

### Step 5: Create Publication Narrative (Priority: LOW - After migration)

**New doc**: `PUBLICATION_NARRATIVE.md`

**Content**:
- Chronological story (N=1 → N=10 → GPU → encoding → parallel)
- Key findings progression
- How discoveries built on each other
- Decision points and checkpoints

**Purpose**: Blueprint for manuscript writing

**Timeline**: 2-3 hours

---

## Recommended Documentation Standards Going Forward

### For Each New Experiment:

1. **Before starting**:
   - Create entry in `lab-notebook/YYYY-MM/YYYYMMDD-NNN-TYPE-title.md`
   - Update `lab-notebook/INDEX.md` with planned entry

2. **During experiment**:
   - Raw data goes in `lab-notebook/raw-data/YYYYMMDD-NNN/`
   - Log key observations in entry file

3. **After completion**:
   - Write detailed analysis in `results/phaseX/dimension_complete.md`
   - Update lab notebook entry with summary + pointer to analysis
   - Update INDEX.md with final stats

4. **Git commit**:
   - Hook validates lab notebook entry exists
   - Hook checks INDEX.md updated
   - Warns if missing

### File Naming Conventions:

**Lab notebook entries**:
```
YYYYMMDD-NNN-TYPE-title.md

Where:
  YYYYMMDD = date (20251031)
  NNN = sequential number (009)
  TYPE = EXPERIMENT, ANALYSIS, REFLECTION, CHECKPOINT, etc.
  title = short-kebab-case-description
```

**Results documents**:
```
phase[N]_dimension_complete.md  (for dimension summaries)
[topic]_findings.md              (for specific analyses)
```

**Experiment protocols**:
```
experiments/phase[N]_dimension/protocol.md
experiments/phase[N]_dimension/README.md (optional quick-start)
```

---

## Immediate Action Items

### Must Do Before Next Pilot:

- [ ] Backfill lab notebook (Entries 009-013)
- [ ] Update INDEX.md to reflect current state
- [ ] Test and re-enable git hooks
- [ ] Create results/ subdirectories (phase1/, phase2/, archive/)

### Should Do Soon:

- [ ] Reorganize results/ files into subdirectories
- [ ] Clean root directory (archive session docs)
- [ ] Create STATUS.md for current state

### Nice to Have:

- [ ] Create PUBLICATION_NARRATIVE.md
- [ ] Consolidate docs/ into METHODOLOGY.md
- [ ] Update CLAUDE.md with documentation standards

---

## Benefits of Restructuring

### For Publication:

✅ **Clear chronology**: Lab notebook INDEX.md tells the story
✅ **Reproducible**: Experiments/ has all protocols
✅ **Detailed findings**: Results/ has publication-ready analysis
✅ **No duplication**: One canonical source per type of info

### For Development:

✅ **Context recovery**: After breaks, read INDEX.md to catch up
✅ **Hook validation**: Git prevents incomplete documentation
✅ **Consistent structure**: Know where everything goes

### For Collaboration:

✅ **Onboarding**: New collaborators read INDEX.md → get full context
✅ **Traceability**: Every decision has a paper trail
✅ **Transparency**: All work visible in chronological log

---

## Risk Assessment

### Risk 1: Migration Breaks References

**Mitigation**:
- Test all internal links after moving files
- Use relative paths in markdown
- Create redirects/symlinks if needed

### Risk 2: Too Much Overhead

**Mitigation**:
- Lab notebook entries should be brief (2-3 paragraphs + bullets)
- Detailed analysis stays in results/
- Hooks automate checks

### Risk 3: Slows Development

**Mitigation**:
- This is a one-time migration cost (~5 hours)
- Ongoing overhead is minimal (15 min per experiment)
- Prevents much larger cost of publication narrative reconstruction

---

## Success Criteria

✅ **Lab notebook current**: All work through Nov 1 documented
✅ **INDEX.md accurate**: Stats reflect actual state
✅ **Results organized**: Clear phase1/, phase2/ structure
✅ **Root clean**: Only strategic docs in root
✅ **Hooks working**: Git enforces structure
✅ **Publication ready**: Clear narrative path to manuscript

---

## Recommendation

**PROCEED WITH MIGRATION** before next pilot (AMX dimension).

**Timeline**: 5-6 hours total
- Step 1 (backfill): 2-3 hours
- Step 2 (reorganize): 1 hour
- Step 3 (clean root): 30 min
- Step 4 (re-enable hooks): 1 hour
- Step 5 (narrative): 2-3 hours (deferred)

**Value**: Prevents fragmentation from compounding. Current state is manageable; after 2-3 more dimensions, it would be chaos.

**Priority**: HIGH - Do before AMX pilot

---

**Audit Date**: November 1, 2025
**Auditor**: Claude (with user concern validation)
**Status**: 🚨 **ACTION REQUIRED**
**Next**: User decision on proceeding with migration
