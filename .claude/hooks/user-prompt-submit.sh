#!/bin/bash
# Hook: Runs after user submits prompt, before Claude responds
# Purpose: Reinforce four-pillar democratization mission

cat << 'EOF'

🌍 DEMOCRATIZING BIOINFORMATICS COMPUTE 🌍

MISSION: Breaking down FOUR barriers that lock researchers out of genomics

FOUR PILLARS - What We're Validating:

1. 💰 ECONOMIC ACCESS (✅ Validated)
   • Consumer hardware ($2-4K) replaces $100K+ HPC clusters
   • Mac Mini/Studio performance proven (849 experiments)
   • ARM NEON: 20-40× speedup, portable across ecosystem

2. 🌱 ENVIRONMENTAL SUSTAINABILITY (⏳ Needs validation)
   • Claim: 300× less energy per analysis (0.5 Wh vs 150 Wh)
   • Status: UNVALIDATED - power consumption pilot pending
   • Impact: 7,475 tons CO₂/year saved if 10K labs adopt

3. 🔄 PORTABILITY (⏳ Needs validation)
   • Claim: ARM NEON rules transfer (Mac → Graviton → Ampere → RPi)
   • Status: UNVALIDATED - only tested on Mac
   • Next: AWS Graviton cross-platform validation (~$1, 3 hours)

4. 📊 DATA ACCESS (✅ Validated)
   • Memory-efficient streaming: 240,000× reduction
   • 5TB dataset analysis on 24GB laptop (proven)
   • Unlocks 40+ petabytes of public data for reanalysis

CURRENT PHASE: Pillar Validation (2/4 complete)

TARGET AUDIENCE:
✓ LMIC researchers (limited HPC access)
✓ Small academic labs (teaching universities)
✓ Field researchers (battery-powered genomics)
✓ Diagnostic labs (in-house pathogen ID)
✓ Students (learning on consumer hardware)

CRITICAL QUESTIONS TO ASK:
❓ Does this work advance one of the FOUR pillars?
❓ Are we validating claims with experimental data?
❓ Does this enable the underserved audiences above?
❓ Are we documenting limitations honestly?

EOF

# ============================================================================
# Lab notebook ENFORCEMENT (keep as-is)
# ============================================================================

USER_MESSAGE="$1"

# Strong reminder for experimental work
if echo "$USER_MESSAGE" | grep -qiE "experiment|pilot|dimension|test.*operation|run.*benchmark|complete.*testing"; then
    today=$(date +%Y%m%d)
    recent_entries=$(find lab-notebook -name "${today}-*.md" 2>/dev/null | wc -l | xargs)

    cat << 'LABEOF'

📔 LAB NOTEBOOK POLICY - MANDATORY DOCUMENTATION
   🚨 ALL experimental work MUST be documented in lab notebook

   BEFORE starting experiments:
   1. Create lab-notebook/YYYY-MM/YYYYMMDD-NNN-EXPERIMENT-name.md
   2. Include proper frontmatter (entry_id, date, type, status, operation)
   3. Document objective, methods, expected outcomes

   AFTER completing experiments:
   1. Update entry with results summary and key findings
   2. Save detailed analysis in results/phase1/ or results/phase2/
   3. Reference detailed analysis from lab notebook entry
   4. Update lab-notebook/INDEX.md (Total Entries, Quick Stats)
   5. Commit entry + INDEX.md + results together

   ⚠️  Git pre-commit hook will BLOCK commits with results/*.md but no lab notebook

LABEOF

    if [ $recent_entries -eq 0 ]; then
        cat << 'LABEOF2'
   📊 STATUS: No lab notebook entry created today
      Next entry should be: YYYYMMDD-020-EXPERIMENT-pillar-name.md
      (See lab-notebook/INDEX.md for next entry number)

LABEOF2
    else
        echo "   ✅ Found $recent_entries entry/entries today"
        find lab-notebook -name "${today}-*.md" 2>/dev/null | while read file; do
            echo "      • $(basename "$file")"
        done
        echo ""
    fi
fi

# Reminder when user indicates work is complete
if echo "$USER_MESSAGE" | grep -qiE "complete|finished|done|results|analyze.*data|create.*report"; then
    cat << 'LABEOF3'

📝 COMPLETION CHECKLIST
   Before considering work "done":
   ✓ Lab notebook entry exists and is complete
   ✓ Key findings documented in entry
   ✓ Raw data saved in lab-notebook/raw-data/YYYYMMDD-NNN/
   ✓ Detailed analysis in results/
   ✓ INDEX.md updated with this work
   ✓ Entry references protocols in experiments/
   ✓ Which PILLAR does this validate? (Economic/Environmental/Portability/Data)

LABEOF3
fi

# Detect if user is asking to commit without lab notebook
if echo "$USER_MESSAGE" | grep -qiE "commit|git.*add|create.*pr|push"; then
    if git diff --cached --name-only 2>/dev/null | grep -q "^results/.*\.md$"; then
        cat << 'LABEOF4'

⚠️  GIT COMMIT WARNING
   Staged files include results/*.md
   Pre-commit hook will REQUIRE corresponding lab notebook entry

   Make sure you have:
   1. Created lab notebook entry for this work
   2. Staged the entry: git add lab-notebook/YYYY-MM/YYYYMMDD-NNN-*.md
   3. Updated INDEX.md: git add lab-notebook/INDEX.md

LABEOF4
    fi
fi
