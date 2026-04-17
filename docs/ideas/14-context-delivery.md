---
name: Context Delivery
description: Serve the right code at the right resolution — token-budgeted, task-relevant, deduplicated context for AI assistants
date: 2026-04-17
status: idea
sources: features/02-smart-context-delivery.md, design/06-compression.md, design/19-context-manager.md
---

# Context Delivery

## Problem

AI assistants waste tokens loading irrelevant code and miss critical context that's buried in large files. The gap between "grep for a string" and "understand the relevant code path" is where most rework happens. Context delivery should be precise, budgeted, and task-aware.

## Current state

- Context assembly (`context_pack`): implemented — task-driven packing with token budget
- Resolution levels: L0 (signature), L1 (IO pattern), L2 (logic flow) designed; L3-L5 not detailed
- LLM summarization for L2: not implemented (requires local inference)
- Context ranking: diff-first BFS, traceability boost, semantic fallback designed; partially wired
- Token budgeting: hard budget exists; budget-aware graph traversal not implemented
- Deduplication across sessions: planned, not built

## What this idea covers

- **Resolution level serving**: serve code at the right depth — signature only, IO contracts, or full logic — based on what the task needs
- **Budget-aware graph traversal**: walk the call graph up to the token budget, prioritizing most relevant paths
- **Task-relevant ranking**: rank context by relevance to the current task, not just proximity
- **Session deduplication**: track what the AI has already seen this session; don't resend
- **Local inference for summaries**: use Gemma4 via Ollama to generate L2 logic flow summaries during indexing

## Open questions

- Is L2 (logic flow summary) worth the indexing cost? Or is L0+L1 sufficient for most tasks?
- Should context delivery be push (auto-loaded) or pull (AI requests specific context)?
- How does this interact with the refocus commands? `/sensei:refocus` is essentially a context reload.
