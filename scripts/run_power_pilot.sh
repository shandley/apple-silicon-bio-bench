#!/bin/bash
#
# Power Consumption Pilot Execution Script
#
# This script:
# 1. Starts powermetrics in background (requires sudo)
# 2. Runs power consumption pilot (24 experiments, ~4 hours)
# 3. Stops powermetrics when complete
# 4. Saves all results with timestamps
#
# Prerequisites:
# - System prepared (run prepare_for_power_test.sh first)
# - Idle for 15 minutes
# - Idle power recorded from Kill-A-Watt meter
#
# Usage:
#   ./scripts/run_power_pilot.sh

set -e  # Exit on error

TIMESTAMP=$(date +"%Y%m%d_%H%M%S")
RESULTS_DIR="results/phase1_power_consumption"
POWERMETRICS_LOG="$RESULTS_DIR/powermetrics_$TIMESTAMP.txt"
PILOT_CSV="$RESULTS_DIR/power_pilot_raw_$TIMESTAMP.csv"
PILOT_LOG="$RESULTS_DIR/power_pilot_log_$TIMESTAMP.txt"

echo "🔋 Power Consumption Pilot - Execution"
echo ""
echo "Timestamp: $TIMESTAMP"
echo "Results directory: $RESULTS_DIR"
echo ""

# Create results directory
mkdir -p "$RESULTS_DIR"
mkdir -p "$RESULTS_DIR/killawatt_photos"

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Pre-Flight Checklist"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "💡 Note: Using powermetrics for CPU power measurement"
echo "   No external hardware needed!"
echo ""

# Verify datasets exist
echo "Checking datasets..."
if [ ! -f "datasets/medium_10k_150bp.fq" ]; then
    echo "❌ Error: datasets/medium_10k_150bp.fq not found"
    echo "   Run: cargo run --release -p asbb-datagen"
    exit 1
fi
if [ ! -f "datasets/large_100k_150bp.fq" ]; then
    echo "❌ Error: datasets/large_100k_150bp.fq not found"
    echo "   Run: cargo run --release -p asbb-datagen"
    exit 1
fi
echo "✓ Datasets present"

# Build pilot binary
echo ""
echo "Building power pilot binary..."
if cargo build --release -p asbb-cli --bin asbb-pilot-power 2>&1 | tee /tmp/build_log.txt | grep -q "error"; then
    echo "❌ Error: Build failed"
    cat /tmp/build_log.txt
    exit 1
fi
echo "✓ Binary built"

# Verify sudo access for powermetrics
echo ""
echo "Checking sudo access for powermetrics..."
if ! sudo -n true 2>/dev/null; then
    echo "🔐 sudo password required for powermetrics"
    echo "   (powermetrics requires root access to read CPU power)"
    sudo -v  # Prompt for password
fi
echo "✓ sudo access verified"

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Experiment Details"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "Operations: 3 (base_counting, gc_content, quality_aggregation)"
echo "Configs: 4 (naive, neon, neon_4t, neon_8t)"
echo "Scales: 2 (Medium 10K, Large 100K)"
echo "Total experiments: 24"
echo ""
echo "Duration per experiment: ~60 seconds"
echo "Cooldown between experiments: 5 seconds"
echo "Estimated total time: ~30 minutes (experiments) + warmup/cooldown = ~1 hour"
echo ""
echo "⚠️  IMPORTANT: Do NOT use this computer during the experiment!"
echo "   - Leave computer idle"
echo "   - Do not move mouse or type"
echo "   - Work on a different computer"
echo ""
echo "✅ No manual intervention needed - fully automated!"
echo "   - powermetrics logs continuously"
echo "   - All data correlated automatically"
echo ""

read -p "Ready to start? (y/n) " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "Aborting..."
    exit 1
fi

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Starting Experiment"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Start powermetrics in background
echo "Starting powermetrics (background)..."
echo "  Output: $POWERMETRICS_LOG"
sudo powermetrics --samplers cpu_power --sample-rate 100 > "$POWERMETRICS_LOG" 2>&1 &
POWERMETRICS_PID=$!
echo "✓ powermetrics started (PID: $POWERMETRICS_PID)"

# Wait a moment for powermetrics to start
sleep 2

# Keep sudo alive in background
# (powermetrics is long-running, sudo may timeout)
(while true; do sudo -n true; sleep 60; done) 2>/dev/null &
SUDO_KEEPER_PID=$!

echo ""
echo "Starting power pilot..."
echo "  Output CSV: $PILOT_CSV"
echo "  Output log: $PILOT_LOG"
echo ""

# Run power pilot
./target/release/asbb-pilot-power > "$PILOT_CSV" 2> "$PILOT_LOG"
PILOT_EXIT_CODE=$?

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Stopping Measurements"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Stop sudo keeper
kill $SUDO_KEEPER_PID 2>/dev/null || true

# Stop powermetrics
echo "Stopping powermetrics..."
sudo kill $POWERMETRICS_PID 2>/dev/null || true
sleep 1
echo "✓ powermetrics stopped"

# Check pilot exit code
if [ $PILOT_EXIT_CODE -ne 0 ]; then
    echo ""
    echo "❌ Error: Power pilot exited with code $PILOT_EXIT_CODE"
    echo "Check log: $PILOT_LOG"
    exit 1
fi

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "✅ Experiment Complete"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Show file sizes
echo "Results saved:"
echo "  CSV: $PILOT_CSV ($(wc -l < "$PILOT_CSV") rows)"
echo "  Log: $PILOT_LOG ($(wc -l < "$PILOT_LOG") lines)"
echo "  powermetrics: $POWERMETRICS_LOG ($(wc -l < "$POWERMETRICS_LOG") lines, $(du -h "$POWERMETRICS_LOG" | cut -f1))"
echo ""

echo "Next steps:"
echo "  1. Re-enable background processes: ./scripts/cleanup_after_power_test.sh"
echo "  2. Parse powermetrics: python analysis/parse_powermetrics.py $POWERMETRICS_LOG $PILOT_CSV"
echo "  3. Generate findings: python analysis/generate_power_findings.py"
echo ""

exit 0
