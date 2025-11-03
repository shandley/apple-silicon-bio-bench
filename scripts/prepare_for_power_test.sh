#!/bin/bash
#
# System Preparation for Power Consumption Testing
#
# This script prepares the Mac for clean power measurements by:
# 1. Disabling background processes
# 2. Setting display to minimum brightness
# 3. Closing unnecessary apps
# 4. Verifying system state
#
# Run this script BEFORE power consumption experiments.
#
# Usage:
#   ./scripts/prepare_for_power_test.sh

set -e  # Exit on error

echo "🔋 Preparing system for power consumption testing..."
echo ""

# Check if running on macOS
if [[ "$(uname)" != "Darwin" ]]; then
    echo "❌ Error: This script is designed for macOS only"
    exit 1
fi

# Check if running on Apple Silicon
if [[ "$(uname -m)" != "arm64" ]]; then
    echo "⚠️  Warning: Not running on Apple Silicon (arm64)"
    echo "   Power measurements may not be accurate"
    read -p "   Continue anyway? (y/n) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Step 1: Disable Background Processes"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Disable Time Machine (requires sudo)
echo "Disabling Time Machine..."
if sudo tmutil disable 2>/dev/null; then
    echo "✓ Time Machine disabled"
else
    echo "⚠️  Could not disable Time Machine (may require sudo password)"
fi

# Disable Spotlight indexing (requires sudo)
echo "Disabling Spotlight indexing..."
if sudo mdutil -a -i off 2>/dev/null; then
    echo "✓ Spotlight indexing disabled"
else
    echo "⚠️  Could not disable Spotlight (may require sudo password)"
fi

# Kill iCloud sync processes
echo "Stopping iCloud sync..."
killall bird 2>/dev/null && echo "✓ iCloud sync stopped" || echo "ℹ️  iCloud sync not running"

# Kill other background apps
echo "Stopping other background apps..."
killall "Google Chrome" 2>/dev/null && echo "✓ Chrome closed" || true
killall "Safari" 2>/dev/null && echo "✓ Safari closed" || true
killall "Mail" 2>/dev/null && echo "✓ Mail closed" || true
killall "Messages" 2>/dev/null && echo "✓ Messages closed" || true
killall "Calendar" 2>/dev/null && echo "✓ Calendar closed" || true
killall "Slack" 2>/dev/null && echo "✓ Slack closed" || true
killall "Discord" 2>/dev/null && echo "✓ Discord closed" || true

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Step 2: Display Settings"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Set display brightness to minimum (requires manual check)
echo "📺 Please manually set display brightness to MINIMUM:"
echo "   - Press F1 repeatedly until brightness is at minimum"
echo "   - Or: System Settings → Displays → Brightness slider to left"
echo ""
read -p "Press ENTER when brightness is at minimum..."

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Step 3: System State Verification"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Check running processes
echo "Checking running apps..."
APP_COUNT=$(osascript -e 'tell application "System Events" to count (every process whose background only is false)')
echo "ℹ️  Running apps: $APP_COUNT"
if [ "$APP_COUNT" -gt 5 ]; then
    echo "⚠️  Warning: $APP_COUNT apps still running (expected ≤5: Finder, Terminal, etc.)"
    echo ""
    echo "Please manually close unnecessary apps:"
    osascript -e 'tell application "System Events" to get name of (every process whose background only is false)' | tr ',' '\n' | sed 's/^ /   - /'
    echo ""
    read -p "Press ENTER when apps are closed..."
fi

# Check battery status
echo ""
echo "Checking battery status..."
BATTERY_STATUS=$(pmset -g batt | grep -o "AC Power" || echo "Battery")
if [[ "$BATTERY_STATUS" == "Battery" ]]; then
    echo "⚠️  Warning: Running on battery power"
    echo "   For best measurements, connect to AC power"
    read -p "   Continue on battery? (y/n) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
else
    echo "✓ Connected to AC power"
fi

# Check for external displays
echo ""
echo "Checking displays..."
DISPLAY_COUNT=$(system_profiler SPDisplaysDataType | grep -c "Display Type" || echo "1")
if [ "$DISPLAY_COUNT" -gt 1 ]; then
    echo "⚠️  Warning: External display(s) detected"
    echo "   External displays add power draw (may affect measurements)"
    echo "   Recommendation: Disconnect external displays for cleanest measurements"
    read -p "   Continue with external display? (y/n) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
else
    echo "✓ Single display (built-in)"
fi

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Step 4: Final Checklist"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

echo "Before proceeding, verify:"
echo "  ✓ Display brightness at minimum"
echo "  ✓ All unnecessary apps closed (only Finder + Terminal running)"
echo "  ✓ External peripherals disconnected (except keyboard/mouse)"
echo "  ✓ Mac plugged into AC power (not on battery)"
echo "  ✓ You have a second computer to work on during testing"
echo ""

read -p "Ready to proceed? (y/n) " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "Aborting..."
    exit 1
fi

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "✅ System Preparation Complete"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "Next steps:"
echo "  1. Let system idle for 15 minutes (allow processes to settle)"
echo "  2. Run: ./scripts/run_power_pilot.sh"
echo ""
echo "💡 Tip: Set a 15-minute timer now!"
echo ""
echo "📊 Note: powermetrics will measure CPU power automatically"
echo "   No manual measurements needed!"

exit 0
