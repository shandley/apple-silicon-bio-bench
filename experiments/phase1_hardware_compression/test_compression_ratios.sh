#!/bin/bash
# Test compression ratios for FASTQ datasets
# This validates that compression provides benefit before implementing full pilot

# Don't exit on errors - we want to test all algorithms even if one fails
set +e

echo "╔════════════════════════════════════════════════════════════════════╗"
echo "║          Hardware Compression Pilot - Ratio Pre-Check              ║"
echo "║          Testing LZFSE, zstd, gzip compression                     ║"
echo "╚════════════════════════════════════════════════════════════════════╝"
echo ""

# Check for required tools
if ! command -v zstd &> /dev/null; then
    echo "⚠️  zstd not found. Install with: brew install zstd"
    exit 1
fi

if ! command -v gzip &> /dev/null; then
    echo "⚠️  gzip not found (should be system default)"
    exit 1
fi

# Note: LZFSE via aa (Apple Archive) command requires macOS 11+
if ! command -v aa &> /dev/null; then
    echo "⚠️  aa (Apple Archive) not found. LZFSE tests will be skipped."
    HAS_AA=false
else
    HAS_AA=true
fi

echo "📊 Compression Tools Status:"
echo "   ✅ zstd: $(zstd --version | head -n1)"
echo "   ✅ gzip: $(gzip --version | head -n1)"
if [ "$HAS_AA" = true ]; then
    echo "   ✅ aa (Apple Archive): Available"
else
    echo "   ⚠️  aa (Apple Archive): Not available"
fi
echo ""

# Test on a few representative datasets
DATASETS=(
    "../../datasets/tiny_100_150bp.fq"
    "../../datasets/medium_10k_150bp.fq"
    "../../datasets/large_100k_150bp.fq"
)

echo "📁 Testing compression on representative datasets:"
echo ""

for dataset in "${DATASETS[@]}"; do
    if [ ! -f "$dataset" ]; then
        echo "⚠️  Dataset not found: $dataset (skipping)"
        continue
    fi

    original=$(stat -f%z "$dataset" 2>/dev/null || stat -c%s "$dataset")
    original_mb=$(echo "scale=2; $original / 1048576" | bc)

    echo "═══════════════════════════════════════════════════════════════════"
    echo "Dataset: $(basename $dataset)"
    echo "Original Size: ${original_mb} MB (${original} bytes)"
    echo "───────────────────────────────────────────────────────────────────"

    # Test zstd
    echo -n "  🔵 zstd ... "
    zstd -q -f -o "${dataset}.zst" "$dataset" 2>/dev/null
    zstd_size=$(stat -f%z "${dataset}.zst" 2>/dev/null || stat -c%s "${dataset}.zst")
    zstd_mb=$(echo "scale=2; $zstd_size / 1048576" | bc)
    zstd_ratio=$(echo "scale=4; $zstd_size / $original" | bc)
    zstd_compression=$(echo "scale=1; (1 - $zstd_ratio) * 100" | bc)
    echo "${zstd_mb} MB (${zstd_compression}% compression, ratio: ${zstd_ratio})"
    rm -f "${dataset}.zst"

    # Test gzip
    echo -n "  🟢 gzip ... "
    gzip -q -f -c "$dataset" > "${dataset}.gz"
    gzip_size=$(stat -f%z "${dataset}.gz" 2>/dev/null || stat -c%s "${dataset}.gz")
    gzip_mb=$(echo "scale=2; $gzip_size / 1048576" | bc)
    gzip_ratio=$(echo "scale=4; $gzip_size / $original" | bc)
    gzip_compression=$(echo "scale=1; (1 - $gzip_ratio) * 100" | bc)
    echo "${gzip_mb} MB (${gzip_compression}% compression, ratio: ${gzip_ratio})"
    rm -f "${dataset}.gz"

    # Test LZFSE (if available)
    if [ "$HAS_AA" = true ]; then
        echo -n "  🍎 LZFSE... "
        if aa archive -d "${dataset}.lzfse" -i "$dataset" --compression lzfse 2>/dev/null; then
            if [ -f "${dataset}.lzfse" ]; then
                lzfse_size=$(stat -f%z "${dataset}.lzfse" 2>/dev/null || stat -c%s "${dataset}.lzfse")
                lzfse_mb=$(echo "scale=2; $lzfse_size / 1048576" | bc)
                lzfse_ratio=$(echo "scale=4; $lzfse_size / $original" | bc)
                lzfse_compression=$(echo "scale=1; (1 - $lzfse_ratio) * 100" | bc)
                echo "${lzfse_mb} MB (${lzfse_compression}% compression, ratio: ${lzfse_ratio})"
                rm -f "${dataset}.lzfse"
            else
                echo "Failed to create LZFSE archive"
            fi
        else
            echo "⚠️  aa command failed (skipping)"
        fi
    fi

    echo ""
done

echo "═══════════════════════════════════════════════════════════════════"
echo ""
echo "✅ Compression Ratio Pre-Check Complete"
echo ""
echo "📈 Expected Results for FASTQ Data:"
echo "   - Compression ratio: 0.35-0.50 (50-65% compression)"
echo "   - Benefit if I/O-bound: ~2× faster at large scale"
echo ""
echo "📋 Next Steps:"
echo "   1. Review compression ratios above"
echo "   2. If ratios < 0.50, proceed with pilot implementation"
echo "   3. Implement compression backends using zstd/gzip Rust crates"
echo "   4. Run full 72-experiment pilot"
echo ""
