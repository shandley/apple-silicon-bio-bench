#!/bin/bash
#
# Download results from Graviton instance
#
# Lab Notebook: Entry 021
# Usage: ./scripts/graviton_download.sh <public-ip>
#

set -e

if [ $# -ne 1 ]; then
    echo "Usage: $0 <public-ip>"
    exit 1
fi

PUBLIC_IP=$1
KEY_NAME="asbb-graviton-key"
SSH_KEY=~/.ssh/${KEY_NAME}.pem

echo "=== Downloading Graviton Results ==="
echo "Public IP: $PUBLIC_IP"
echo

# Create local results directory
mkdir -p results/cross_platform_graviton

# Find the CSV file on remote instance
echo "Finding results file..."
CSV_FILE=$(ssh -i "$SSH_KEY" -o StrictHostKeyChecking=no ec2-user@${PUBLIC_IP} "ls -t ~/asbb/results/cross_platform_graviton/graviton_raw_*.csv | head -1")

if [ -z "$CSV_FILE" ]; then
    echo "❌ No results file found on instance"
    exit 1
fi

echo "Found: $CSV_FILE"
echo

# Download results
echo "Downloading results..."
scp -i "$SSH_KEY" -o StrictHostKeyChecking=no ec2-user@${PUBLIC_IP}:${CSV_FILE} results/cross_platform_graviton/

FILENAME=$(basename "$CSV_FILE")
echo "✅ Downloaded: results/cross_platform_graviton/$FILENAME"

# Verify file
LINES=$(wc -l < "results/cross_platform_graviton/$FILENAME")
echo "File contains $LINES lines (expected: 46 = header + 45 experiments)"

if [ "$LINES" -eq 46 ]; then
    echo "✅ All 45 experiments present"
else
    echo "⚠️  Expected 46 lines, got $LINES"
fi

echo
echo "Results downloaded successfully!"
echo
echo "Next step: ./scripts/graviton_terminate.sh"
echo "           python analysis/compare_mac_graviton.py"
