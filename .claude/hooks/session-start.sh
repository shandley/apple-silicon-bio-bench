#!/bin/bash
# Hook: Runs at the start of each Claude Code session
# Purpose: Display lab notebook status and four-pillar mission status

cat << 'EOF'

🌍 ASBB: DEMOCRATIZING BIOINFORMATICS COMPUTE 🌍

FOUR-PILLAR MISSION STATUS:
  💰 Economic Access:        ✅ VALIDATED (849 experiments, 40-80× NEON speedup)
  🌱 Environmental:           ✅ VALIDATED (24 experiments, 1.95-3.27× energy efficiency)
  🔄 Portability:             ✅ VALIDATED (27 experiments, Mac → Graviton transfer)
  📊 Data Access:             ⚠️  PARTIAL (baseline measured, streaming in Week 2)

COMPLETION: 3.5/4 pillars validated | Week 2: Complete 4th pillar via biofast

EOF

# ============================================================================
# Lab notebook status (keep existing functionality)
# ============================================================================

# Count entries
if [ -d "lab-notebook" ]; then
    total_entries=$(find lab-notebook -name "*.md" ! -name "INDEX.md" 2>/dev/null | wc -l | xargs)
    echo "📔 LAB NOTEBOOK STATUS"
    echo "   Total entries: $total_entries"

    # Show today's entries
    today=$(date +%Y%m%d)
    today_entries=$(find lab-notebook -name "${today}-*.md" 2>/dev/null | wc -l | xargs)
    if [ $today_entries -gt 0 ]; then
        echo "   Today's entries: $today_entries"
        find lab-notebook -name "${today}-*.md" 2>/dev/null | while read file; do
            echo "     • $(basename "$file")"
        done
    else
        echo "   Today's entries: 0"
    fi

    # Show active checkpoints
    active_checkpoints=$(grep -l "status: active" lab-notebook/**/*.md 2>/dev/null | wc -l | xargs)
    if [ $active_checkpoints -gt 0 ]; then
        echo ""
        echo "🚨 ACTIVE CHECKPOINTS: $active_checkpoints"
        grep -l "status: active" lab-notebook/**/*.md 2>/dev/null | while read file; do
            title=$(grep "^# " "$file" | head -1 | sed 's/^# //')
            echo "     • $(basename "$file" .md)"
        done
    fi

    # Show in-progress experiments
    in_progress=$(grep -l "status: in-progress\|status: in_progress" lab-notebook/**/*.md 2>/dev/null | wc -l | xargs)
    if [ $in_progress -gt 0 ]; then
        echo ""
        echo "🔬 IN-PROGRESS: $in_progress"
        grep -l "status: in-progress\|status: in_progress" lab-notebook/**/*.md 2>/dev/null | while read file; do
            echo "     • $(basename "$file" .md)"
        done
    fi

    # Check if INDEX.md exists
    if [ ! -f "lab-notebook/INDEX.md" ]; then
        echo ""
        echo "⚠️  INDEX.md not found - should be created"
    else
        # Check if INDEX.md is current
        index_date=$(grep "Last Updated" lab-notebook/INDEX.md | head -1 | grep -oE "[0-9]{4}-[0-9]{2}-[0-9]{2}" || echo "unknown")
        current_date=$(date +%Y-%m-%d)

        if [ "$index_date" != "$current_date" ] && [ "$index_date" != "unknown" ]; then
            days_old=$(( ($(date +%s) - $(date -j -f "%Y-%m-%d" "$index_date" +%s)) / 86400 ))
            if [ $days_old -gt 2 ]; then
                echo ""
                echo "⚠️  INDEX.md may be stale (last updated: $index_date, $days_old days ago)"
                echo "   Review if recent work needs to be added"
            fi
        fi
    fi

    # ========================================================================
    # SYNC CHECK: Detect undocumented experimental results
    # ========================================================================

    # Count results files modified in last 3 days that might not be in lab notebook
    recent_results=$(find results -name "*.md" -mtime -3 2>/dev/null | wc -l | xargs)

    if [ $recent_results -gt 0 ]; then
        # Check if we have recent lab notebook entries
        recent_lab=$(find lab-notebook -name "*.md" ! -name "INDEX.md" -mtime -3 2>/dev/null | wc -l | xargs)

        if [ $recent_lab -eq 0 ] && [ $recent_results -gt 0 ]; then
            echo ""
            echo "🚨 DOCUMENTATION SYNC WARNING"
            echo "   Found $recent_results results file(s) modified in last 3 days"
            echo "   But NO lab notebook entries in same period"
            echo ""
            echo "   Recent results files:"
            find results -name "*.md" -mtime -3 2>/dev/null | head -5 | while read file; do
                echo "      • $(basename "$file")"
            done
            if [ $recent_results -gt 5 ]; then
                echo "      ... and $(($recent_results - 5)) more"
            fi
            echo ""
            echo "   ⚠️  All experimental work should have lab notebook entries"
            echo "   Consider backfilling missing entries if this represents real experiments"
            echo ""
        fi
    fi

    # Check for uncommitted lab notebook entries
    if [ -d ".git" ]; then
        uncommitted_entries=$(git status --porcelain 2>/dev/null | grep "^[AM]. lab-notebook/.*\.md$" | grep -v "INDEX.md" | wc -l | xargs)
        if [ $uncommitted_entries -gt 0 ]; then
            echo ""
            echo "📝 UNCOMMITTED LAB NOTEBOOK ENTRIES: $uncommitted_entries"
            echo "   Remember to commit lab notebook entries when work is complete"
            git status --porcelain 2>/dev/null | grep "^[AM]. lab-notebook/.*\.md$" | grep -v "INDEX.md" | while read status file; do
                echo "      • $(basename "$file")"
            done
            echo ""
        fi
    fi

else
    echo "   ⚠️  Lab notebook directory not found"
fi

echo ""
echo "🚀 CURRENT PHASE: Foundation → Implementation (Nov 3, 2025)"
echo ""
echo "📋 ROADMAP (2-3 weeks to completion):"
echo "   Week 1 (Nov 4-8):   Complete DAG traversal (740 experiments)"
echo "                       → Fills testing gaps, enables optimal biofast configs"
echo "   Week 2 (Nov 11-14): Build biofast production library"
echo "                       → Streaming + auto-optimization + CLI tools"
echo "                       → Validates Data Access pillar experimentally"
echo "   Week 3 (Nov 18-22): Validation + paper draft"
echo "                       → Comprehensive paper + production tool"
echo ""
echo "📚 KEY DOCUMENTS:"
echo "   • CURRENT_STATUS.md  - Always-current project status"
echo "   • BIOFAST_VISION.md  - Production library design"
echo "   • DAG_FRAMEWORK.md   - Novel testing methodology"
echo "   • ROADMAP.md         - Detailed timeline"
echo ""
