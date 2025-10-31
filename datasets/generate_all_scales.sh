#!/bin/bash
# Generate all standard ASBB dataset scales
#
# Scales (from METHODOLOGY.md Level 2 experiments):
#   100 → Tiny (15 KB)
#   1K → Small (150 KB)
#   10K → Medium (1.5 MB)
#   100K → Large (15 MB)
#   1M → Very Large (150 MB)
#   10M → Huge (1.5 GB)

set -e  # Exit on error

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DATAGEN="${SCRIPT_DIR}/generation-scripts/target/release/datagen"

# Check if datagen is built
if [ ! -f "$DATAGEN" ]; then
    echo "⚠️  datagen not found. Building..."
    cd "${SCRIPT_DIR}/generation-scripts"
    cargo build --release
    cd "${SCRIPT_DIR}"
fi

echo "╔════════════════════════════════════════════════════════════════════╗"
echo "║  ASBB Multi-Scale Dataset Generation                              ║"
echo "║  Generating 6 scales for systematic exploration                   ║"
echo "╚════════════════════════════════════════════════════════════════════╝"
echo ""

# Common parameters
LENGTH_MEAN=150
LENGTH_STD=10
QUALITY_DIST="degrading"

# Scale 1: Tiny (100 sequences)
echo "1️⃣  Generating TINY dataset (100 sequences, ~15 KB)..."
$DATAGEN generate \
    --output "${SCRIPT_DIR}/tiny_100_150bp.fq" \
    --format fastq \
    --num-sequences 100 \
    --length-mean $LENGTH_MEAN \
    --length-std $LENGTH_STD \
    --quality-dist $QUALITY_DIST \
    --seed 1 \
    --validate
echo ""

# Scale 2: Small (1K sequences)
echo "2️⃣  Generating SMALL dataset (1,000 sequences, ~150 KB)..."
$DATAGEN generate \
    --output "${SCRIPT_DIR}/small_1k_150bp.fq" \
    --format fastq \
    --num-sequences 1000 \
    --length-mean $LENGTH_MEAN \
    --length-std $LENGTH_STD \
    --quality-dist $QUALITY_DIST \
    --seed 2 \
    --validate
echo ""

# Scale 3: Medium (10K sequences)
echo "3️⃣  Generating MEDIUM dataset (10,000 sequences, ~1.5 MB)..."
$DATAGEN generate \
    --output "${SCRIPT_DIR}/medium_10k_150bp.fq" \
    --format fastq \
    --num-sequences 10000 \
    --length-mean $LENGTH_MEAN \
    --length-std $LENGTH_STD \
    --quality-dist $QUALITY_DIST \
    --seed 3 \
    --validate
echo ""

# Scale 4: Large (100K sequences)
echo "4️⃣  Generating LARGE dataset (100,000 sequences, ~15 MB)..."
$DATAGEN generate \
    --output "${SCRIPT_DIR}/large_100k_150bp.fq" \
    --format fastq \
    --num-sequences 100000 \
    --length-mean $LENGTH_MEAN \
    --length-std $LENGTH_STD \
    --quality-dist $QUALITY_DIST \
    --seed 4 \
    --validate
echo ""

# Scale 5: Very Large (1M sequences)
echo "5️⃣  Generating VERY LARGE dataset (1,000,000 sequences, ~150 MB)..."
$DATAGEN generate \
    --output "${SCRIPT_DIR}/vlarge_1m_150bp.fq" \
    --format fastq \
    --num-sequences 1000000 \
    --length-mean $LENGTH_MEAN \
    --length-std $LENGTH_STD \
    --quality-dist $QUALITY_DIST \
    --seed 5 \
    --validate
echo ""

# Scale 6: Huge (10M sequences)
echo "6️⃣  Generating HUGE dataset (10,000,000 sequences, ~1.5 GB)..."
$DATAGEN generate \
    --output "${SCRIPT_DIR}/huge_10m_150bp.fq" \
    --format fastq \
    --num-sequences 10000000 \
    --length-mean $LENGTH_MEAN \
    --length-std $LENGTH_STD \
    --quality-dist $QUALITY_DIST \
    --seed 6 \
    --validate
echo ""

echo "╔════════════════════════════════════════════════════════════════════╗"
echo "║  Dataset Generation Complete                                       ║"
echo "╚════════════════════════════════════════════════════════════════════╝"
echo ""
echo "📊 Generated datasets:"
ls -lh "${SCRIPT_DIR}"/*.fq 2>/dev/null || echo "   (Listing files...)"
echo ""
echo "✅ All datasets validated and ready for experiments"
echo ""
