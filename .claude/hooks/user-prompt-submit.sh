#!/bin/bash
# Hook: Runs after user submits prompt, before Claude responds
# Purpose: Reinforce core philosophy and challenge traditional thinking

cat << 'EOF'

🧬 ASBB MISSION REMINDER 🧬

CORE PHILOSOPHY - Apple Silicon First:
• Resist x86 assumptions - Traditional patterns may NOT apply here
• Explore novel approaches - Unified memory, Neural Engine, AMX, heterogeneous cores
• Question everything - "What does Apple Silicon enable?" not "How did x86 do this?"
• Document failures - "Neural Engine 0.8× slower" is valuable knowledge

CRITICAL QUESTIONS TO ASK YOURSELF:
❓ Am I falling back into traditional bioinformatics thinking?
❓ Have I considered Apple Silicon-specific approaches?
❓ Am I exploring NEON-native, Metal-native, heterogeneous options?
❓ Am I documenting what DOESN'T work, not just what does?

FOR EVERY OPERATION IMPLEMENTATION:
1. ✓ Traditional/naive (baseline)
2. ✓ NEON-native (designed for SIMD, not ported)
3. ✓ Metal-native (tile memory, unified memory)
4. ✓ Heterogeneous (P-cores + E-cores + GCD)
5. ✓ Novel (Neural Engine, AMX, hardware compression)
6. ✓ Measure & document ALL results (including failures)

THIS IS SCIENCE, NOT ENGINEERING:
Goal = Universal understanding, not one-off solutions
Goal = Systematic exploration, not ad-hoc optimization
Goal = Novel discoveries, not benchmarking x86 ports

📖 See CLAUDE.md "Critical Philosophy: Think Apple Silicon First" for details

EOF
