#!/bin/bash
#
# Cleanup After Power Consumption Testing
#
# This script re-enables background processes that were disabled during power testing:
# 1. Re-enable Time Machine
# 2. Re-enable Spotlight indexing
# 3. Restore normal system operation
#
# Run this script AFTER power consumption experiments complete.
#
# Usage:
#   ./scripts/cleanup_after_power_test.sh

set -e  # Exit on error

echo "🧹 Cleaning up after power consumption testing..."
echo ""

# Re-enable Time Machine
echo "Re-enabling Time Machine..."
if sudo tmutil enable 2>/dev/null; then
    echo "✓ Time Machine re-enabled"
else
    echo "⚠️  Could not re-enable Time Machine"
fi

# Re-enable Spotlight indexing
echo "Re-enabling Spotlight indexing..."
if sudo mdutil -a -i on 2>/dev/null; then
    echo "✓ Spotlight indexing re-enabled"
else
    echo "⚠️  Could not re-enable Spotlight"
fi

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "✅ Cleanup Complete"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "Your system is restored to normal operation."
echo "You can now:"
echo "  - Use the computer normally"
echo "  - Open apps"
echo "  - Restore display brightness"
echo ""

exit 0
