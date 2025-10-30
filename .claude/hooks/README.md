# Claude Code Hooks for ASBB

**Purpose**: Maintain philosophical focus and prevent regression to traditional x86 thinking patterns during development.

---

## The Problem

During BioMetal development, we repeatedly fell back into traditional bioinformatics optimization patterns designed for x86 architectures. These patterns are **not optimal** for Apple Silicon's unique capabilities.

As Claude Code's context window compacts, critical insights and philosophical guidelines can get lost, leading to:
- Regression to x86 thinking ("just use a hash table")
- Skipping novel explorations (Neural Engine, AMX, unified memory)
- Focusing on engineering (one-off solutions) vs. science (systematic exploration)
- Losing exciting discoveries between sessions

## The Solution

**Claude Code hooks** that run automatically at key moments to:
1. Reinforce core philosophy before every response
2. Prompt documentation of discoveries before context compaction
3. Re-ground in mission after context compaction

---

## Hooks Overview

### 1. `user-prompt-submit.sh`
**Trigger**: After user submits a message, before Claude responds

**Purpose**: Remind Claude of core philosophy before every response

**Content**:
- Core philosophy: Apple Silicon-first thinking
- Critical self-audit questions
- 6-step implementation checklist (traditional → NEON → Metal → heterogeneous → novel → measure)
- Mission reminder: Science, not engineering

**Impact**: Prevents sliding into autopilot/traditional thinking patterns

---

### 2. `before-context-compact.sh`
**Trigger**: Before context window compaction

**Purpose**: Capture important findings before they're lost

**Content**:
- Prompt to document exciting discoveries
- Self-audit: "Did I explore Apple Silicon-native approaches?"
- Red flag patterns to watch for ("GPU overhead is too high" without testing unified memory)
- Reminder to update NEXT_STEPS.md or CLAUDE.md with insights

**Impact**: Preserves discoveries that would otherwise be lost to compaction

---

### 3. `after-context-compact.sh`
**Trigger**: After context window compaction

**Purpose**: Re-ground in mission and philosophy after memory loss

**Content**:
- Project goal: Systematic performance mapping → formal optimization rules
- Paradigm shift: Engineering → Science
- Critical philosophy: Resist x86 assumptions
- Apple Silicon unique capabilities (8 features)
- 6-step implementation checklist
- Key document references with line numbers

**Impact**: Quickly restores mission clarity after compaction

---

## How They Work

Claude Code hooks are shell scripts that:
1. Live in `.claude/hooks/`
2. Are executable (`chmod +x`)
3. Output messages that Claude sees
4. Run automatically at specified trigger points

**No user action required** - they run automatically in the background.

---

## Example Output

When `user-prompt-submit.sh` runs:

```
🧬 ASBB MISSION REMINDER 🧬

CORE PHILOSOPHY - Apple Silicon First:
• Resist x86 assumptions - Traditional patterns may NOT apply here
• Explore novel approaches - Unified memory, Neural Engine, AMX, heterogeneous cores
• Question everything - "What does Apple Silicon enable?" not "How did x86 do this?"
• Document failures - "Neural Engine 0.8× slower" is valuable knowledge

CRITICAL QUESTIONS TO ASK YOURSELF:
❓ Am I falling back into traditional bioinformatics thinking?
❓ Have I considered Apple Silicon-specific approaches?
...
```

---

## Maintenance

### When to Update Hooks

Update hooks when:
- New critical lessons emerge from development
- Novel approaches are discovered (add to checklist)
- Red flag patterns are identified (add to before-compact warnings)
- Key document sections change (update line number references)

### Testing Hooks

Test manually:
```bash
./.claude/hooks/user-prompt-submit.sh
./.claude/hooks/before-context-compact.sh
./.claude/hooks/after-context-compact.sh
```

All should output formatted reminders without errors.

---

## Philosophy

**Why this matters**:

Traditional bioinformatics tools (BLAST, BWA, Bowtie, etc.) were designed pre-2020 for x86 architectures. Their optimization strategies assume:
- Discrete GPUs (copy overhead)
- Homogeneous cores
- SSE/AVX as afterthought
- No Neural Engine, no AMX, no unified memory

**Apple Silicon is fundamentally different**. Without active reminders, it's easy to unconsciously port x86 patterns instead of exploring native approaches.

**These hooks are not optional** - they're essential infrastructure for maintaining the "Apple Silicon-first" philosophy that differentiates ASBB from simple benchmarking projects.

---

## Integration with Documentation

Hooks reference key sections:
- `CLAUDE.md` lines 162-318: "Critical Philosophy: Think Apple Silicon First"
- `METHODOLOGY.md` lines 19-73: "Guiding Philosophy: Novel Approaches for Novel Hardware"
- `NEXT_STEPS.md` lines 8-38: "🚨 Critical Development Philosophy 🚨"

These documents provide detailed rationale and examples. Hooks provide just-in-time reminders.

---

**Status**: Active and operational
**Last Updated**: October 30, 2025
