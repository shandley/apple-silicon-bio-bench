# Claude Code Hooks for ASBB

**Purpose**: Maintain philosophical focus AND enforce documentation standards to prevent fragmentation.

**Updated**: November 1, 2025 - Enhanced with enforcement mechanisms after documentation fragmentation incident

---

## The Problem (Updated)

**Original problem** (Oct 30): During BioMetal development, we repeatedly fell back into traditional bioinformatics optimization patterns designed for x86 architectures.

**New problem discovered** (Nov 1): Documentation fragmented across multiple locations when lab notebook system was abandoned during intense experimentation (Oct 31). 704 experiments (GPU, encoding, parallel dimensions) documented only in `/results/`, not in lab notebook.

**Combined risks**:
- Regression to x86 thinking ("just use a hash table")
- Skipping novel explorations (Neural Engine, AMX, unified memory)
- **Documentation fragmentation** (work in /results/ instead of lab notebook)
- **Loss of chronological narrative** (no INDEX.md updates)
- **Publication challenges** (scattered findings, unclear story)

## The Solution (Enhanced)

**Multi-layered enforcement system**:

1. **Claude Code hooks**: Run automatically at key moments to:
   - Reinforce core philosophy before every response
   - **ENFORCE** lab notebook documentation (not just suggest)
   - Detect and warn about sync issues at session start
   - Block workflows that bypass lab notebook

2. **Git pre-commit hook**: Validates and BLOCKS commits that:
   - Add results/*.md files without corresponding lab notebook entries
   - Create new lab notebook entries without updating INDEX.md
   - Have malformed frontmatter or invalid naming

**This is now an ENFORCEMENT SYSTEM, not just reminders.**

---

## Hooks Overview

### 1. `user-prompt-submit.sh` (ENHANCED)
**Trigger**: After user submits a message, before Claude responds

**Purpose**:
- Remind Claude of core philosophy before every response
- **ENFORCE** lab notebook documentation for all experimental work
- Warn about git commits that will fail pre-commit validation

**Content**:
- Core philosophy: Apple Silicon-first thinking
- Critical self-audit questions
- 6-step implementation checklist (traditional → NEON → Metal → heterogeneous → novel → measure)
- Mission reminder: Science, not engineering
- **NEW**: Mandatory lab notebook policy with workflow steps
- **NEW**: Completion checklist for finished work
- **NEW**: Git commit warnings if results/*.md are staged

**Impact**:
- Prevents sliding into autopilot/traditional thinking patterns
- **Prevents documentation fragmentation** by enforcing lab notebook workflow
- **Catches issues before commit** (proactive warning system)

---

### 2. `before-context-compact.sh`
**Trigger**: Before context window compaction

**Purpose**: Capture important findings before they're lost

**Content**:
- Prompt to document exciting discoveries
- Self-audit: "Did I explore Apple Silicon-native approaches?"
- Red flag patterns to watch for ("GPU overhead is too high" without testing unified memory)
- Reminder to update NEXT_STEPS.md or CLAUDE.md with insights

**Impact**: Preserves discoveries that would otherwise be lost to compaction

---

### 3. `after-context-compact.sh`
**Trigger**: After context window compaction

**Purpose**: Re-ground in mission and philosophy after memory loss

**Content**:
- Project goal: Systematic performance mapping → formal optimization rules
- Paradigm shift: Engineering → Science
- Critical philosophy: Resist x86 assumptions
- Apple Silicon unique capabilities (8 features)
- 6-step implementation checklist
- Key document references with line numbers

**Impact**: Quickly restores mission clarity after compaction

---

### 4. `session-start.sh` (ENHANCED)
**Trigger**: At the start of each Claude Code session

**Purpose**:
- Display lab notebook status
- **NEW**: Detect documentation sync issues
- **NEW**: Warn if INDEX.md is stale
- **NEW**: Check for uncommitted lab notebook entries

**Content**:
- Entry counts (total, today's, in-progress, checkpoints)
- **NEW**: Sync warning if results/ modified but no lab notebook entries
- **NEW**: INDEX.md staleness check (warns if >2 days old)
- **NEW**: Uncommitted entry warnings

**Impact**:
- Quick status overview at session start
- **Early detection of fragmentation** (proactive warning)
- **Reminds to commit completed work**

---

## Git Pre-Commit Hook (NEW - CRITICAL ENFORCEMENT)

### `/.githooks/pre-commit`
**Trigger**: Before every git commit

**Purpose**: **BLOCK** commits that violate documentation standards

**Validations (ENFORCED)**:

1. **Results without lab notebook** (NEW - addresses fragmentation root cause):
   - ❌ BLOCKS commits with new/modified `results/*.md` files
   - ✅ ALLOWS if corresponding lab notebook entry exists in same commit
   - Exception: Files in organized subdirs (results/phase1/, phase2/, archive/)

2. **Missing INDEX.md updates** (NEW):
   - ❌ BLOCKS new lab notebook entries without INDEX.md update
   - Validates entries are actually listed in INDEX.md

3. **Entry format validation** (existing):
   - Filename format: YYYYMMDD-NNN-TYPE-description.md
   - YAML frontmatter present
   - Required fields: entry_id, date, type, status
   - entry_id matches filename
   - Type matches filename

**This hook PREVENTS the exact fragmentation that occurred Oct 31.**

**Exit codes**:
- Exit 1 (BLOCK): Validation failed, commit prevented
- Exit 0 (ALLOW): All validations passed

---

## How They Work

Claude Code hooks are shell scripts that:
1. Live in `.claude/hooks/`
2. Are executable (`chmod +x`)
3. Output messages that Claude sees
4. Run automatically at specified trigger points

**No user action required** - they run automatically in the background.

---

## Example Output

When `user-prompt-submit.sh` runs:

```
🧬 ASBB MISSION REMINDER 🧬

CORE PHILOSOPHY - Apple Silicon First:
• Resist x86 assumptions - Traditional patterns may NOT apply here
• Explore novel approaches - Unified memory, Neural Engine, AMX, heterogeneous cores
• Question everything - "What does Apple Silicon enable?" not "How did x86 do this?"
• Document failures - "Neural Engine 0.8× slower" is valuable knowledge

CRITICAL QUESTIONS TO ASK YOURSELF:
❓ Am I falling back into traditional bioinformatics thinking?
❓ Have I considered Apple Silicon-specific approaches?
...
```

---

## Maintenance

### When to Update Hooks

Update hooks when:
- New critical lessons emerge from development
- Novel approaches are discovered (add to checklist)
- Red flag patterns are identified (add to before-compact warnings)
- Key document sections change (update line number references)

### Testing Hooks

Test manually:
```bash
./.claude/hooks/user-prompt-submit.sh
./.claude/hooks/before-context-compact.sh
./.claude/hooks/after-context-compact.sh
```

All should output formatted reminders without errors.

---

## Philosophy

**Why this matters**:

Traditional bioinformatics tools (BLAST, BWA, Bowtie, etc.) were designed pre-2020 for x86 architectures. Their optimization strategies assume:
- Discrete GPUs (copy overhead)
- Homogeneous cores
- SSE/AVX as afterthought
- No Neural Engine, no AMX, no unified memory

**Apple Silicon is fundamentally different**. Without active reminders, it's easy to unconsciously port x86 patterns instead of exploring native approaches.

**These hooks are not optional** - they're essential infrastructure for maintaining the "Apple Silicon-first" philosophy that differentiates ASBB from simple benchmarking projects.

---

## Integration with Documentation

Hooks reference key sections:
- `CLAUDE.md` lines 162-318: "Critical Philosophy: Think Apple Silicon First"
- `METHODOLOGY.md` lines 19-73: "Guiding Philosophy: Novel Approaches for Novel Hardware"
- `NEXT_STEPS.md` lines 8-38: "🚨 Critical Development Philosophy 🚨"

These documents provide detailed rationale and examples. Hooks provide just-in-time reminders.

---

**Status**: Active and operational - ENHANCED with enforcement (Nov 1, 2025)
**Last Updated**: November 1, 2025

---

## What Happened: The Fragmentation Incident (Oct 31, 2025)

**Timeline**:
- **Oct 30**: Lab notebook system working well (Entries 001-008, 120 experiments)
- **Oct 31**: Intense experimentation begins (GPU, encoding, parallel dimensions)
- **Oct 31**: Comprehensive results docs created in `/results/` directly
- **Oct 31**: **704 experiments completed** but NO lab notebook entries created
- **Nov 1**: Fragmentation discovered during audit

**Root cause**: Original hooks only **reminded** about lab notebook, didn't **enforce** it. When workflow shifted to creating polished results docs directly in `/results/`, hooks didn't catch the bypass.

**Evidence from git**:
```bash
# Oct 30 commit (d75834d) - Hook working:
lab-notebook/2025-10/20251030-008-EXPERIMENT-n-content-n5.md ✅
lab-notebook/INDEX.md ✅

# Oct 31 commit (11422cb) - Hook bypassed:
results/n10_final_validation.md ← Should be Entry 009
(no lab-notebook entries) ❌
```

**Impact**:
- 3 major dimensions (GPU 32 exp, encoding 72 exp, parallel 600 exp) undocumented in lab notebook
- INDEX.md stale (showed 8 entries, should be 11)
- No chronological record of Oct 31 work
- Publication narrative unclear

**Resolution** (Nov 1):
- Backfilled 3 missing entries (009-011)
- Updated INDEX.md (8 → 11 entries, 120 → 824 experiments)
- Reorganized results/ into phase1/, phase2/, archive/
- **Enhanced all hooks with ENFORCEMENT mechanisms**
- **Updated git pre-commit hook to BLOCK fragmentation patterns**

**Lesson learned**: Documentation systems need **enforcement, not just reminders**.
