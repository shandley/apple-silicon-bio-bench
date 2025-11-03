#!/bin/bash
#
# Full automation: Launch, run, analyze, and terminate Graviton experiment
#
# Lab Notebook: Entry 021
# Usage: ./scripts/graviton_full_automation.sh
#
# This script orchestrates the complete Graviton validation:
# 1. Launch instance
# 2. Setup and compile
# 3. Run experiments
# 4. Download results
# 5. Terminate instance
# 6. Analyze results
# 7. Generate findings
#

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

cd "$PROJECT_ROOT"

echo "========================================"
echo "  Graviton Portability Validation"
echo "  Lab Notebook: Entry 021"
echo "========================================"
echo

# Phase 1: Launch instance
echo "=== Phase 1: Launch Instance ==="
./scripts/graviton_launch.sh

# Load instance info
source results/cross_platform_graviton/instance_info.txt
echo

# Phase 2: Setup instance
echo "=== Phase 2: Setup Instance ==="
./scripts/graviton_setup.sh "$PUBLIC_IP"
echo

# Phase 3: Run experiments
echo "=== Phase 3: Run Experiments (45-60 minutes) ==="
./scripts/graviton_run.sh "$PUBLIC_IP"
echo

# Phase 4: Download results
echo "=== Phase 4: Download Results ==="
./scripts/graviton_download.sh "$PUBLIC_IP"
echo

# Phase 5: Terminate instance
echo "=== Phase 5: Terminate Instance ==="
./scripts/graviton_terminate.sh
echo

# Phase 6: Extract Mac baseline
echo "=== Phase 6: Extract Mac Baseline ==="
python3 analysis/extract_mac_baseline.py \
    results/phase1_power_consumption/power_pilot_raw_20251102_184235.csv
echo

# Phase 7: Compare platforms
echo "=== Phase 7: Cross-Platform Comparison ==="
GRAVITON_CSV=$(ls -t results/cross_platform_graviton/graviton_raw_*.csv | head -1)
python3 analysis/compare_mac_graviton.py \
    results/cross_platform_graviton/mac_baseline.csv \
    "$GRAVITON_CSV"
echo

# Phase 8: Generate findings
echo "=== Phase 8: Generate Findings ==="
python3 analysis/generate_graviton_findings.py \
    results/cross_platform_graviton/mac_vs_graviton_comparison.csv
echo

# Done!
echo "========================================"
echo "  ✅ GRAVITON VALIDATION COMPLETE!"
echo "========================================"
echo
echo "Results:"
echo "  - Raw data: $GRAVITON_CSV"
echo "  - Comparison: results/cross_platform_graviton/mac_vs_graviton_comparison.csv"
echo "  - Findings: results/cross_platform_graviton/FINDINGS.md"
echo
echo "Next steps:"
echo "  1. Review FINDINGS.md"
echo "  2. Update lab notebook (Entry 021)"
echo "  3. Update CURRENT_STATUS.md (Portability pillar: ✅)"
echo
