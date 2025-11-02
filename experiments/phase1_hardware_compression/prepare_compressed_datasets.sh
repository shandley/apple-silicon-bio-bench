#!/bin/bash
# Prepare compressed versions of all datasets for Hardware Compression pilot

set -e

echo "╔════════════════════════════════════════════════════════════════════╗"
echo "║       Hardware Compression Pilot - Dataset Preparation             ║"
echo "║       Creating gzip and zstd compressed versions                    ║"
echo "╚════════════════════════════════════════════════════════════════════╝"
echo ""

# Datasets to compress
DATASETS=(
    "../../datasets/tiny_100_150bp.fq"
    "../../datasets/small_1k_150bp.fq"
    "../../datasets/medium_10k_150bp.fq"
    "../../datasets/large_100k_150bp.fq"
    "../../datasets/vlarge_1m_150bp.fq"
    "../../datasets/huge_10m_150bp.fq"
)

echo "📁 Compressing datasets:"
echo ""

total_datasets=${#DATASETS[@]}
current=0

for dataset in "${DATASETS[@]}"; do
    current=$((current + 1))

    if [ ! -f "$dataset" ]; then
        echo "⚠️  Dataset not found: $dataset (skipping)"
        continue
    fi

    basename=$(basename "$dataset")
    original_size=$(stat -f%z "$dataset" 2>/dev/null || stat -c%s "$dataset")
    original_mb=$(echo "scale=2; $original_size / 1048576" | bc)

    echo "[$current/$total_datasets] $basename (${original_mb} MB)"

    # gzip compression
    echo -n "  🟢 gzip ... "
    if [ -f "${dataset}.gz" ]; then
        echo "exists (skipping)"
    else
        gzip -c "$dataset" > "${dataset}.gz"
        gz_size=$(stat -f%z "${dataset}.gz" 2>/dev/null || stat -c%s "${dataset}.gz")
        gz_mb=$(echo "scale=2; $gz_size / 1048576" | bc)
        gz_ratio=$(echo "scale=2; 100 * (1 - $gz_size / $original_size)" | bc)
        echo "✅ ${gz_mb} MB (${gz_ratio}% compression)"
    fi

    # zstd compression
    echo -n "  🔵 zstd ... "
    if [ -f "${dataset}.zst" ]; then
        echo "exists (skipping)"
    else
        zstd -q -f -o "${dataset}.zst" "$dataset"
        zst_size=$(stat -f%z "${dataset}.zst" 2>/dev/null || stat -c%s "${dataset}.zst")
        zst_mb=$(echo "scale=2; $zst_size / 1048576" | bc)
        zst_ratio=$(echo "scale=2; 100 * (1 - $zst_size / $original_size)" | bc)
        echo "✅ ${zst_mb} MB (${zst_ratio}% compression)"
    fi

    echo ""
done

echo "═══════════════════════════════════════════════════════════════════"
echo "✅ Dataset Preparation Complete"
echo ""
echo "📊 Summary:"
echo "   - Original datasets: 6"
echo "   - Compressed formats: 2 (gzip, zstd)"
echo "   - Total files: 18 (6 original + 6 gzip + 6 zstd)"
echo ""
echo "📋 Next Steps:"
echo "   1. Verify compressed files exist"
echo "   2. Create experiment harness (asbb-pilot-compression)"
echo "   3. Run 54 experiments (3 ops × 3 compressions × 6 scales)"
echo ""
