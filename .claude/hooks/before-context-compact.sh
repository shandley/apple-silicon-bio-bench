#!/bin/bash
# Hook: Runs before context compaction
# Purpose: Capture important findings and challenge thinking patterns

cat << 'EOF'

⚠️  CONTEXT COMPACTION APPROACHING ⚠️

BEFORE MEMORIES FADE - DOCUMENT:
📝 Any exciting discoveries or unexpected results
📝 Novel approaches that worked (or failed)
📝 Performance cliffs, thresholds, or interaction effects
📝 Open questions that need further exploration
📝 "Aha moments" about Apple Silicon capabilities

SELF-AUDIT QUESTIONS:
🔍 In recent work, did I explore Apple Silicon-native approaches?
🔍 Or did I fall back into traditional x86 thinking?
🔍 Did I test NEON-native, Metal-native, heterogeneous, novel implementations?
🔍 Did I document negative results (what DIDN'T work)?
🔍 Am I treating this as science (exploration) or engineering (solutions)?

IF YOU'VE BEEN IMPLEMENTING:
✓ Document any "traditional approach worked better than expected" findings
✓ Document any "novel approach failed but we learned why" insights
✓ Update NEXT_STEPS.md with discoveries or open questions
✓ Consider if findings should be added to CLAUDE.md lessons

PATTERNS TO WATCH FOR (RED FLAGS):
🚫 "Let's just use a hash table" (Did we explore NEON hashing, Metal tile memory, AMX?)
🚫 "GPU overhead is too high" (Did we test with unified memory zero-copy?)
🚫 "This is obviously faster" (Did we MEASURE it?)
🚫 "Traditional approach is fine" (Did we explore novel alternatives?)

💡 Use this moment to capture insights before they're lost to compaction!

EOF
