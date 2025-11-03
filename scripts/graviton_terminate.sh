#!/bin/bash
#
# Terminate Graviton instance (stop AWS billing)
#
# Lab Notebook: Entry 021
# Usage: ./scripts/graviton_terminate.sh
#

set -e

echo "=== Terminating Graviton Instance ==="
echo

# Load instance info
INFO_FILE="results/cross_platform_graviton/instance_info.txt"

if [ ! -f "$INFO_FILE" ]; then
    echo "❌ Instance info file not found: $INFO_FILE"
    echo "Please provide instance ID manually:"
    read -p "Instance ID: " INSTANCE_ID
    REGION="us-east-1"
else
    source "$INFO_FILE"
    echo "Instance ID: $INSTANCE_ID"
    echo "Region: $REGION"
fi

# Confirm termination
echo
echo "⚠️  This will TERMINATE the instance and STOP all billing."
echo "Make sure you have downloaded all results!"
echo
read -p "Terminate instance $INSTANCE_ID? (yes/no): " CONFIRM

if [ "$CONFIRM" != "yes" ]; then
    echo "Termination cancelled"
    exit 0
fi

# Terminate instance
echo
echo "Terminating instance..."
aws ec2 terminate-instances --instance-ids "$INSTANCE_ID" --region "$REGION"

# Wait for termination
echo "Waiting for instance to terminate..."
aws ec2 wait instance-terminated --instance-ids "$INSTANCE_ID" --region "$REGION"

echo "✅ Instance terminated successfully"
echo

# Calculate approximate cost
if [ -f "$INFO_FILE" ]; then
    LAUNCHED_AT=$(grep LAUNCHED_AT "$INFO_FILE" | cut -d'=' -f2)
    TERMINATED_AT=$(date -u +"%Y-%m-%dT%H:%M:%SZ")

    echo "Instance lifetime:"
    echo "  Launched: $LAUNCHED_AT"
    echo "  Terminated: $TERMINATED_AT"
    echo
    echo "Approximate cost: \$0.145/hour × runtime"
    echo "(Check AWS billing console for exact cost)"
fi

echo
echo "✅ Cleanup complete!"
echo
echo "Next step: python analysis/compare_mac_graviton.py"
