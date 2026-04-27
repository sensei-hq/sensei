---
target: github/copilot-docs (or appropriate Copilot feedback channel)
status: draft
relates-to: docs/ideas/24a-observatory-data-audit.md
---

# Copilot Chat/Agent: expose session metrics and lifecycle hooks

## Summary

GitHub Copilot (Chat and Agent modes in VS Code/JetBrains) doesn't expose session-level metrics or lifecycle events to extensions. This blocks building quality analytics, cost dashboards, and efficiency tracking for AI-assisted development.

## What's needed

1. **Token/cost data per chat session** — available via extension API or VS Code output channel after session completes
2. **Session lifecycle events** — `onSessionStart`, `onSessionEnd` events with aggregate metrics (turns, tokens, duration, outcome)
3. **Tool call observability** — when Copilot Agent uses tools (@workspace, terminal, file operations), expose what was called and what it returned (truncated)
4. **Usage/quota visibility** — API or UI to see remaining Copilot usage (especially relevant for metered plans)

## Use case

A VS Code extension that shows:
- "This Copilot session: 12 turns, estimated $0.40"
- "Copilot Agent used 8 tool calls: 6 file reads, 1 terminal, 1 search"
- "Sessions where I provided clear context had 2x fewer corrections"

## Why this matters

Developers using Copilot daily can't answer "is Copilot helping me?" with data. They can't compare:
- Chat vs Agent mode efficiency
- Sessions with @workspace context vs without
- Cost per task or per repo

Enterprise customers especially need this for ROI analysis and usage optimization.

## Proposed extension API

```typescript
vscode.copilot.onSessionEnd((event) => {
  event.sessionId;       // string
  event.turnCount;       // number
  event.tokensIn;        // number
  event.tokensOut;       // number
  event.toolCalls;       // {name, responsePreview}[]
  event.durationSeconds; // number
});
```
