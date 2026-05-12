---
name: Session Continuity
description: Preserve context across sessions and interruptions — orientation, snapshots, recovery, and interrupt detection
date: 2026-04-17
status: idea
sources: features/03-session-continuity.md, gap-analysis.md
---

# Session Continuity

## Problem

Every new conversation starts cold. The AI re-reads files, re-discovers patterns, and loses decisions from prior sessions. Snapshots exist but recovery is incomplete. Interruptions (IDE close, token limit, crashes) leave sessions in an unknown state.

## Current state

- Session orientation (`get_session_context`): implemented, returns project summary and interrupted context (sub-600-token)
- Snapshot & recovery (`take_snapshot`): partial — snapshots recorded, recovery context surfaced but not all scenarios covered
- Interrupt detection: planned — IDE close, restart, token limit detection not fully implemented
- Heartbeat / liveness: partial — `beat()` fires on tool calls but `active_sessions` view may not update correctly
- Worktree-aware snapshots: planned, not built

## What this idea covers

- **Robust session recovery**: when a session is interrupted, the next session gets a complete handoff — what was being done, what decisions were made, what's left
- **Interrupt detection**: detect IDE close, terminal kill, token limit exhaustion, and auto-snapshot before context is lost
- **Session handoff document**: generate a concise handoff doc that survives between conversations (complements the workflow phase documents)
- **Active session tracking**: accurate liveness detection so stale sessions are cleaned up and current sessions are visible
- **Cross-session decision tracking**: decisions made in conversation N are available in conversation N+1 without relying on memory

## Open questions

- Should session recovery be automatic (SessionStart hook loads last snapshot) or explicit (`/sensei:session`)?
- How much conversation content should be preserved vs. just the summary?
- Does the workflow phase document system (from idea 01) make dedicated session continuity less critical?
