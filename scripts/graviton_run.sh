#!/bin/bash
#
# Run Graviton portability experiments
#
# Lab Notebook: Entry 021
# Usage: ./scripts/graviton_run.sh <public-ip>
#

set -e

if [ $# -ne 1 ]; then
    echo "Usage: $0 <public-ip>"
    exit 1
fi

PUBLIC_IP=$1
KEY_NAME="asbb-graviton-key"
SSH_KEY=~/.ssh/${KEY_NAME}.pem

echo "=== Running Graviton Experiments ==="
echo "Public IP: $PUBLIC_IP"
echo "Total experiments: 45 (5 ops × 3 configs × 3 scales)"
echo "Expected duration: 45-60 minutes"
echo

# Create results directory on instance
echo "Creating results directory..."
ssh -i "$SSH_KEY" -o StrictHostKeyChecking=no ec2-user@${PUBLIC_IP} "mkdir -p ~/asbb/results/cross_platform_graviton"

# Run experiments
echo "Starting experiments..."
echo

ssh -i "$SSH_KEY" -o StrictHostKeyChecking=no ec2-user@${PUBLIC_IP} "source ~/.cargo/env && cd ~/asbb && ./target/release/asbb-pilot-graviton"

echo
echo "✅ Experiments complete!"
echo
echo "Next step: ./scripts/graviton_download.sh $PUBLIC_IP"
