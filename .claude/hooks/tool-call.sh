#!/bin/bash
# Hook: Runs after tool calls (if supported)
# Purpose: Validate lab notebook entries

# Note: This may not be a supported hook type in Claude Code
# If it doesn't run, we'll rely on git hooks for validation

TOOL_NAME="$1"

# Check if we're in a Write or Edit operation on lab-notebook files
if [ "$TOOL_NAME" = "Write" ] || [ "$TOOL_NAME" = "Edit" ]; then
    # Try to find recently modified lab notebook files
    find lab-notebook -name "*.md" ! -name "INDEX.md" -mmin -1 2>/dev/null | while read FILE_PATH; do
        if [ -f "$FILE_PATH" ]; then
            echo "📔 Validating lab notebook entry: $(basename "$FILE_PATH")"

            basename=$(basename "$FILE_PATH")

            # Validate naming convention
            if ! echo "$basename" | grep -qE '^[0-9]{8}-[0-9]{3}-[A-Z]+-.*\.md$'; then
                echo "❌ INVALID FILENAME FORMAT"
                echo "   Expected: YYYYMMDD-NNN-TYPE-description.md"
                echo "   Got: $basename"
                exit 1
            fi

            # Check for YAML frontmatter
            if ! head -1 "$FILE_PATH" | grep -q "^---$"; then
                echo "⚠️  MISSING FRONTMATTER (will be caught by git hook)"
                echo "   All lab notebook entries should start with YAML frontmatter"
            fi

            # Check required fields
            missing_fields=""
            for field in "entry_id" "date" "type" "status"; do
                if ! grep -q "^$field:" "$FILE_PATH"; then
                    missing_fields="$missing_fields $field"
                fi
            done

            if [ -n "$missing_fields" ]; then
                echo "⚠️  MISSING REQUIRED FIELDS:$missing_fields"
                echo "   (will be caught by git hook)"
            fi

            # Check if INDEX.md needs updating
            entry_id=$(basename "$FILE_PATH" .md)
            if [ -f "lab-notebook/INDEX.md" ]; then
                if ! grep -q "$entry_id" lab-notebook/INDEX.md 2>/dev/null; then
                    echo "💡 REMINDER: Update lab-notebook/INDEX.md"
                    echo "   New entry not found in index: $entry_id"
                fi
            fi

            if [ -z "$missing_fields" ]; then
                echo "✅ Lab notebook entry validated"
            fi
        fi
    done
fi
