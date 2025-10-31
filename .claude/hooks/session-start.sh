#!/bin/bash
# Hook: Runs at the start of each Claude Code session
# Purpose: Display lab notebook status and active work

cat << 'EOF'

📔 LAB NOTEBOOK STATUS
EOF

# Count entries
if [ -d "lab-notebook" ]; then
    total_entries=$(find lab-notebook -name "*.md" ! -name "INDEX.md" 2>/dev/null | wc -l | xargs)
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
    fi
else
    echo "   ⚠️  Lab notebook directory not found"
fi

echo ""
