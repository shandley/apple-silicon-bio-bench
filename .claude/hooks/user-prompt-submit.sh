#!/bin/bash
# Hook: Runs after user submits prompt, before Claude responds
# Purpose: Reinforce core philosophy and challenge traditional thinking

cat << 'EOF'

🧬 ASBB MISSION REMINDER 🧬

CORE PHILOSOPHY - Apple Silicon First:
• Resist x86 assumptions - Traditional patterns may NOT apply here
• Explore novel approaches - Unified memory, Neural Engine, AMX, heterogeneous cores
• Question everything - "What does Apple Silicon enable?" not "How did x86 do this?"
• Document failures - "Neural Engine 0.8× slower" is valuable knowledge

CRITICAL QUESTIONS TO ASK YOURSELF:
❓ Am I falling back into traditional bioinformatics thinking?
❓ Have I considered Apple Silicon-specific approaches?
❓ Am I exploring NEON-native, Metal-native, heterogeneous options?
❓ Am I documenting what DOESN'T work, not just what does?

FOR EVERY OPERATION IMPLEMENTATION:
1. ✓ Traditional/naive (baseline)
2. ✓ NEON-native (designed for SIMD, not ported)
3. ✓ Metal-native (tile memory, unified memory)
4. ✓ Heterogeneous (P-cores + E-cores + GCD)
5. ✓ Novel (Neural Engine, AMX, hardware compression)
6. ✓ M5: GPU Neural Accelerators (4× AI performance, ML on GPU)
7. ✓ Measure & document ALL results (including failures)

THIS IS SCIENCE, NOT ENGINEERING:
Goal = Universal understanding, not one-off solutions
Goal = Systematic exploration, not ad-hoc optimization
Goal = Novel discoveries, not benchmarking x86 ports

📖 See CLAUDE.md "Critical Philosophy: Think Apple Silicon First" for details

EOF

# ============================================================================
# Lab notebook ENFORCEMENT (not just suggestion)
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
      Next entry should be: YYYYMMDD-012-EXPERIMENT-dimension-name.md
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
   ✓ Detailed analysis in results/phase1/ or results/phase2/
   ✓ INDEX.md updated with this work
   ✓ Entry references protocols in experiments/

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
