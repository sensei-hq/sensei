---
name: Metrics & Analytics
description: Track development interactions, compute quality scores (FTR, turn count, rework rate), surface coaching insights, and visualize trends
date: 2026-04-17
status: idea
sources: features/07-analytics.md, design/22-telemetry-resilience.md, design/gaps.md
---

# Metrics & Analytics

## Problem

We collect tool call events and session data but don't close the loop — there's no way to see whether development quality is improving, which patterns cause rework, or where the AI consistently fails. FTR is computed but not visualized. Coaching feedback is planned but not implemented.

## Current state

- FTR scoring: computed from snapshots and tool errors (partial)
- Session telemetry: tool calls, turn counts, token data recorded (partial)
- Telemetry resilience: JSONL fallback exists for tool events; OTLP has no fallback (partial)
- Quality coaching: not implemented
- Dashboard visualization: not implemented
- Per-model cost attribution: not implemented

## What this idea covers

- **Interaction tracking**: capture user messages, AI responses, tool usage, and outcomes at conversation level
- **Quality metrics**: FTR (first-time-right), turn count per task, rework rate, pattern adherence score
- **Coaching layer**: surface personalized recommendations based on metric trends (e.g., "requirement quality dropped — consider using `/sensei:idea` more")
- **Benchmark framework**: compare model/agent performance across standardized tasks
- **Trend visualization**: dashboard pages showing quality metrics over time per project

## Open questions

- Where does interaction data live? SQLite? Separate analytics DB?
- Privacy: what level of conversation content is stored vs. just metadata?
- How does this integrate with the workflow phases? Each phase could emit its own metrics.
- Should coaching be proactive (AI suggests improvements) or on-demand (`/sensei:analyze metrics`)?
