---
name: Pattern Store
description: Capture, index, and enforce recurring code patterns — so the AI builds on existing foundations instead of reinventing
date: 2026-04-17
status: idea
sources: design/17-pattern-store.md, features/01-codebase-intelligence.md
---

# Pattern Store

## Problem

The AI reinvents structure that already exists in the codebase. Adapter patterns, task worker wrappers, test fixtures — these are recurring structures that should be codified and reused. Currently, patterns live in PATTERNS.md (manual) but there's no automatic detection or enforcement.

## Current state

- PATTERNS.md: manual, maintained by user via `/sensei:pattern-extract`
- Pattern-based development skill: exists in plugin (loads patterns before coding)
- Auto-detection during indexing: planned, not implemented
- Pattern search/matching: not implemented
- Pattern persistence across re-index: not implemented

## What this idea covers

- **Pattern auto-detection**: during indexing, identify recurring file/function structures and surface them as candidate patterns
- **Pattern search**: find patterns by description or example code
- **Pattern enforcement**: during `/sensei:build`, check that new code follows applicable patterns; flag deviations
- **Pattern persistence**: patterns survive re-indexing; versioned alongside code
- **Pattern templates**: generate new files from pattern templates (like scaffolding)

## Open questions

- How do we distinguish a "pattern" from mere repetition? What's the threshold?
- Should patterns be global (across projects) or project-specific? Both?
- How does this interact with guardrails? Are patterns a subset of guardrails?
