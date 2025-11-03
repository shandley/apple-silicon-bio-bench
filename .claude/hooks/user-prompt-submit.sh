#!/bin/bash
# Hook: Runs after user submits prompt, before Claude responds
# Purpose: Reinforce four-pillar democratization mission

cat << 'EOF'

🌍 DEMOCRATIZING BIOINFORMATICS COMPUTE 🌍

MISSION: Breaking down FOUR barriers that lock researchers out of genomics

FOUR PILLARS - What We're Validating:

1. 💰 ECONOMIC ACCESS (✅ Validated)
   • Consumer hardware ($2-4K) replaces $100K+ HPC clusters
   • 849 experiments prove 40-80× NEON speedup
   • ARM NEON: portable across ecosystem (Mac, Graviton, RPi)

2. 🌱 ENVIRONMENTAL SUSTAINABILITY (✅ Validated)
   • 24 experiments: 1.95-3.27× more energy efficient
   • Impact: Enables field work, reduces carbon footprint
   • Validates sustainability claim with empirical data

3. 🔄 PORTABILITY (✅ Validated)
   • 27 experiments: Perfect Mac → AWS Graviton 3 transfer
   • ARM NEON rules work cross-platform (0.8-1.14× relative)
   • Proves no vendor lock-in, ecosystem democratization

4. 📊 DATA ACCESS (⚠️ Partial - In Progress)
   • Baseline measured: 25 experiments
   • Streaming implementation: Week 2 (biofast library)
   • Will validate experimentally (not just calculate)

CURRENT PHASE: Foundation → Implementation (3.5/4 pillars complete)

TARGET AUDIENCE:
✓ LMIC researchers (limited HPC access)
✓ Small academic labs (teaching universities)
✓ Field researchers (battery-powered genomics)
✓ Diagnostic labs (in-house pathogen ID)
✓ Students (learning on consumer hardware)

NEW VISION (Nov 3, 2025): Analysis + Implementation + Practical Tool
   • DAG Framework: Novel methodology for systematic hardware testing
   • biofast Library: Production tool implementing empirical optimizations
   • Complete Story: Measurement → Rules → Implementation
   • Timeline: 2-3 weeks (Week 1: DAG, Week 2: biofast, Week 3: paper)

CRITICAL QUESTIONS TO ASK:
❓ Does this advance one of the FOUR pillars?
❓ Does this advance DAG completion or biofast implementation?
❓ Are we validating claims with experimental data (not calculations)?
❓ Does this enable the underserved audiences above?
❓ Are we documenting limitations honestly?
❓ Is this building toward production-ready tool (not prototype)?

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
