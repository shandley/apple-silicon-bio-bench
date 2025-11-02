#!/bin/bash
# Hook: Runs before context compaction
# Purpose: Capture pillar-critical findings before memory compaction

cat << 'EOF'

⚠️  CONTEXT COMPACTION APPROACHING ⚠️

PILLAR-CRITICAL INFORMATION - ENSURE DOCUMENTED:
📝 Economic: Performance numbers that prove consumer hardware viability
📝 Environmental: Energy consumption measurements (power pilot data)
📝 Portability: Cross-platform validation results (Mac → Graviton → etc.)
📝 Data Access: Memory footprint data, streaming performance

FOUR-PILLAR VALIDATION CHECKLIST:
🔍 Economic (✅ Validated): Do we have speedup numbers for key operations?
🔍 Environmental (⏳ Pending): Do we have power consumption data (Wh per analysis)?
🔍 Portability (⏳ Pending): Have we validated on non-Mac ARM platforms?
🔍 Data Access (✅ Validated): Do we have memory footprint characterization?

IMPACT CLAIMS - MUST BE BACKED BY DATA:
🔍 "300× less energy" - Do we have measurements to support this?
🔍 "Works across ARM ecosystem" - Have we tested beyond Mac?
🔍 "240,000× memory reduction" - Is this experimentally validated? (YES)
🔍 "$2-4K replaces $100K+ HPC" - Do we have performance parity data? (YES)

IF YOU'VE BEEN EXPERIMENTING:
✓ Ensure pillar validation data is in lab notebook entries
✓ Update CURRENT_STATUS.md with pillar completion status
✓ Document any limitations or caveats discovered
✓ Note which pillar each experiment validates

IF YOU'VE BEEN ANALYZING:
✓ Ensure findings are saved in results/ (not just chat)
✓ Update phase analysis documents if new insights emerged
✓ Document statistical validation (p-values, confidence intervals)
✓ Note which target audience each finding serves (LMIC, small labs, students, etc.)

TARGET AUDIENCE REMINDERS:
💡 LMIC researchers: Do findings reduce barrier to entry?
💡 Small labs: Can they afford and deploy this?
💡 Field researchers: Battery-powered, portable, no internet?
💡 Students: Learning on consumer hardware, not HPC?
💡 Diagnostic labs: In-house pathogen ID, no bioinformatics staff?

💡 Use this moment to preserve pillar-critical insights before compaction!

EOF
