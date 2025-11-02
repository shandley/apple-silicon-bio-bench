# Documentation Cleanup Plan - November 2, 2025

## Current State Assessment

**Total root-level markdown files**: 23 files (~330KB total)
**Problem**: Fragmented, outdated, conflicting information
**Cause**: Project evolution from "systematic pilots" → "four-pillar democratization"

---

## Core Issue

**Mission drift in documentation:**
- Original focus: "Systematic performance characterization of Apple Silicon"
- Current mission: "Democratizing bioinformatics via economic, environmental, portability, and data access breakthroughs"
- **Documentation hasn't caught up**

---

## Cleanup Strategy

### Phase 1: Archive Outdated Session Summaries

**Move to `docs/archive/session-summaries/`** (historical record, not active):
- ❌ AMX_PILOT_READY.md (Oct 31, pre-pilot summary)
- ❌ PARALLEL_PILOT_READY.md (Oct 31, pre-pilot summary)
- ❌ PHASE1_DAY1_SUMMARY.md (Oct 31, outdated)
- ❌ PHASE1_DAY2_SUMMARY.md (Nov 1, outdated)
- ❌ PILOT_CHECKPOINT.md (Oct 30, outdated)
- ❌ SESSION_SUMMARY_20251102.md (Nov 2, pre-analysis)
- ❌ SESSION_WRAPUP.md (Nov 1, outdated)
- ❌ RESTART_INSTRUCTIONS.md (Nov 1, Level 1/2 attempt, abandoned)

**Rationale**: Historical artifacts from experimentation phase, not relevant to current mission

---

### Phase 2: Archive Exploratory/Decision Documents

**Move to `docs/archive/explorations/`** (interesting but superseded):
- ❌ AMX_NEURAL_CREATIVE_EXPLORATION.md (20KB, exploratory thinking, deferred)
- ❌ DOCUMENTATION_AUDIT.md (audit from earlier session)
- ❌ DOCUMENTATION_REVIEW_AND_NEXT_STEPS.md (superseded)
- ❌ DEV_PROCESS_REVIEW.md (wrong framing, needs rewrite)
- ❌ NEXT_STEPS.md (33KB, outdated roadmap)
- ❌ PHASE1_COMPLETENESS_REVIEW.md (superseded by PHASE1_COMPLETE_ANALYSIS.md)
- ❌ PROJECT_ROADMAP.md (outdated)
- ❌ PUBLICATION_READINESS_ASSESSMENT.md (outdated)
- ❌ REALISTIC_VALUE_ASSESSMENT.md (superseded by DEMOCRATIZING_BIOINFORMATICS_COMPUTE.md)

**Rationale**: Interesting historical thinking, but superseded by current mission documents

---

### Phase 3: Keep and Update Core Documents

**Root-level (active, keep visible)**:

1. ✅ **README.md** (18KB)
   - Status: Needs update to reflect four-pillar mission
   - Action: Rewrite with democratization focus

2. ✅ **CLAUDE.md** (39KB)
   - Status: Needs major update (stale session status, missing pillar focus)
   - Action: Complete rewrite with four-pillar framework

3. ✅ **DEMOCRATIZING_BIOINFORMATICS_COMPUTE.md** (37KB)
   - Status: ✅ CURRENT - This IS the mission document
   - Action: Keep at root, minor updates for status

4. ✅ **OPTIMIZATION_RULES.md** (18KB)
   - Status: ✅ CURRENT - Quick reference for developers
   - Action: Keep at root, no changes needed

5. ✅ **METHODOLOGY.md** (26KB)
   - Status: Needs review - May need four-pillar framing
   - Action: Review and update if needed

6. ✅ **STREAMING_ASSESSMENT.md** (16KB)
   - Status: Relevant to Data Access pillar
   - Action: Keep at root, verify currency

7. ✅ **ISSUES.md** (2.6KB)
   - Status: Active tracking (GPU compatibility, etc.)
   - Action: Keep at root, review for stale items

---

### Phase 4: Update Claude Code Hooks

**Location**: `.claude/hooks/`

**Files to update**:

1. ✅ **user-prompt-submit.sh**
   - Current: "Explore all Apple Silicon approaches!"
   - Update to: "Four-pillar democratization mission focus"
   - Tone: Implementation → Validation of pillars

2. ✅ **before-context-compact.sh**
   - Current: "Document Apple Silicon discoveries"
   - Update to: "Retain pillar-critical findings"
   - Focus: Performance → Accessibility impact

3. ✅ **session-start.sh**
   - Current: Lab notebook status (good)
   - Action: Add four-pillar status reminder

---

### Phase 5: Update Git Hooks

**Location**: `.githooks/pre-commit`

**Assessment**: ✅ Working perfectly, lab notebook enforcement good
**Action**: No changes needed

---

### Phase 6: Create New Core Documents

**New documents to create**:

1. ✅ **CURRENT_STATUS.md** (replaces stale CLAUDE.md section)
   - Four-pillar completion status
   - What's validated, what's pending
   - Next experiments needed
   - Clear, concise, always current

2. ✅ **CONTRIBUTING.md** (for future contributors)
   - Four-pillar mission
   - How to run experiments
   - How to add operations
   - Lab notebook requirements

---

## Execution Plan

### Step 1: Create Archive Structure
```bash
mkdir -p docs/archive/session-summaries
mkdir -p docs/archive/explorations
```

### Step 2: Move Session Summaries (8 files)
```bash
mv AMX_PILOT_READY.md docs/archive/session-summaries/
mv PARALLEL_PILOT_READY.md docs/archive/session-summaries/
mv PHASE1_DAY1_SUMMARY.md docs/archive/session-summaries/
mv PHASE1_DAY2_SUMMARY.md docs/archive/session-summaries/
mv PILOT_CHECKPOINT.md docs/archive/session-summaries/
mv SESSION_SUMMARY_20251102.md docs/archive/session-summaries/
mv SESSION_WRAPUP.md docs/archive/session-summaries/
mv RESTART_INSTRUCTIONS.md docs/archive/session-summaries/
```

### Step 3: Move Explorations (9 files)
```bash
mv AMX_NEURAL_CREATIVE_EXPLORATION.md docs/archive/explorations/
mv DOCUMENTATION_AUDIT.md docs/archive/explorations/
mv DOCUMENTATION_REVIEW_AND_NEXT_STEPS.md docs/archive/explorations/
mv DEV_PROCESS_REVIEW.md docs/archive/explorations/
mv NEXT_STEPS.md docs/archive/explorations/
mv PHASE1_COMPLETENESS_REVIEW.md docs/archive/explorations/
mv PROJECT_ROADMAP.md docs/archive/explorations/
mv PUBLICATION_READINESS_ASSESSMENT.md docs/archive/explorations/
mv REALISTIC_VALUE_ASSESSMENT.md docs/archive/explorations/
```

### Step 4: Update Core Documents

**Priority order**:
1. CLAUDE.md (comprehensive rewrite with four-pillar focus)
2. README.md (update to reflect democratization mission)
3. Hook updates (user-prompt-submit, before-context-compact, session-start)
4. Create CURRENT_STATUS.md (always-accurate status)
5. Review METHODOLOGY.md (add four-pillar framing if needed)
6. Review STREAMING_ASSESSMENT.md (verify currency)
7. Review ISSUES.md (close stale, add power/portability pilots)

### Step 5: Create Archive README
```bash
# docs/archive/README.md
Explains what's archived and why
```

---

## After Cleanup

### Root Directory Structure

**Core mission** (7 files):
- README.md - Project overview with four-pillar focus
- CLAUDE.md - Development guide with four-pillar framework
- DEMOCRATIZING_BIOINFORMATICS_COMPUTE.md - The mission statement
- OPTIMIZATION_RULES.md - Developer quick reference
- METHODOLOGY.md - Scientific methodology
- CURRENT_STATUS.md - Always-current status
- ISSUES.md - Active tracking

**Specialized** (2 files):
- STREAMING_ASSESSMENT.md - Data Access pillar technical
- CONTRIBUTING.md - For future contributors

**Total root docs**: 9 files (down from 23)
**Archived**: 17 files (historical record preserved)

---

## Success Criteria

✅ Root directory has only current, mission-aligned documents
✅ Claude Code hooks reinforce four-pillar mission
✅ CLAUDE.md reflects current status and mission (not Oct 30)
✅ README.md attracts LMIC researchers, small labs, students
✅ All documents emphasize: Economic + Environmental + Portability + Data Access
✅ Stale/exploratory docs archived (not deleted - historical value)
✅ Clear path forward: Power pilot → AWS Graviton → Four-pillar paper

---

## Timeline

**Phase 1-3** (Archive): 15 minutes
**Phase 4-5** (Hook updates): 30 minutes
**Phase 6** (Core doc rewrites): 2-3 hours

**Total**: 3-4 hours for complete cleanup

---

**Next**: Execute cleanup in stages with user approval at each step
