#!/bin/bash
# Hook: Runs after context compaction
# Purpose: Re-ground in mission and philosophy after memory loss

cat << 'EOF'

🔄 CONTEXT COMPACTED - MISSION RESET 🔄

PROJECT: Apple Silicon Bio Bench (ASBB)
GOAL: Systematic performance mapping of sequence operations on Apple Silicon
      → Derive formal optimization rules for automatic application

WHY THIS EXISTS:
• 10 months of BioMetal optimization taught us patterns (NEON 98×, GPU 6× for large batches)
• But: Technical debt, inconsistent optimization, ad-hoc decisions
• Solution: Systematic experiments → Statistical analysis → Universal rules → Zero per-command optimization

PARADIGM SHIFT:
From: Optimize each command individually (engineering)
To: Map entire performance space systematically (science)

🚨 CRITICAL PHILOSOPHY - NEVER FORGET:
Traditional bioinformatics tools were designed for x86 (pre-2020)
→ Those optimization patterns may NOT apply to Apple Silicon
→ We must actively RESIST falling back into x86 thinking
→ We must EXPLORE novel approaches unique to Apple Silicon

APPLE SILICON UNIQUE CAPABILITIES (did not exist pre-2020):
• Unified Memory - Zero-copy CPU↔GPU
• NEON - First-class SIMD (not afterthought)
• Neural Engine - ML inference (16-38 TOPS)
• Heterogeneous - P-cores + E-cores + QoS
• AMX - 512-bit matrix operations
• Metal - Tile memory, threadgroups
• Hardware Compression - AppleArchive acceleration
• GCD + QoS - System-level optimization

FOR EVERY OPERATION - TEST ALL:
1. Traditional/naive (baseline)
2. NEON-native (designed for SIMD)
3. Metal-native (unified memory, tile memory)
4. Heterogeneous (P/E-cores, GCD, QoS)
5. Novel (Neural Engine, AMX, hardware compression)
6. Document ALL results (failures are valuable!)

KEY DOCUMENTS TO RE-READ:
📖 CLAUDE.md "Critical Philosophy: Think Apple Silicon First" (lines 162-318)
📖 METHODOLOGY.md "Guiding Philosophy: Novel Approaches for Novel Hardware" (lines 19-73)
📖 NEXT_STEPS.md "🚨 Critical Development Philosophy 🚨" (lines 8-38)

REMEMBER: This is SCIENCE (systematic exploration), not ENGINEERING (one-off solutions)

EOF
