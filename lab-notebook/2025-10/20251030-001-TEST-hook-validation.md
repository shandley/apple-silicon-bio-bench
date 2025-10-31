---
entry_id: 20251030-001-TEST-hook-validation
date: 2025-10-30
type: TEST
status: in_progress
phase: test
author: Scott Handley + Claude

references:
  protocols: []
  prior_entries: []

tags:
  - test
  - hook-validation

raw_data: null
---

# Hook Validation Test Entry

This is a test entry to validate that our hooks are working correctly.

## Purpose

Testing both Claude Code hooks and Git hooks.

## Expected Behavior

1. Claude Code tool-call hook should validate this file (if supported)
2. Git pre-commit hook should validate this file when staged
3. Both should check:
   - Filename format (YYYYMMDD-NNN-TYPE-description.md)
   - YAML frontmatter presence
   - Required fields (entry_id, date, type, status)

## Test Results

Will be updated after testing.
