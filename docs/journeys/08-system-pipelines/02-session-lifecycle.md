---
name: Session Lifecycle
type: system-journey
covers: [11, 07, 04, 01, 28]
triggered-by: [04-work-with-assistants (session start/end), hooks]
---

# System: Session Lifecycle

> What happens behind the scenes during and after an AI coding session.

## Lifecycle flow

```mermaid
flowchart TD
    A[SessionStart hook fires] --> B[Create session row]
    B --> B1[Record: project, folder, ACP, timestamp]
    B --> B2[Load session context\n~300 tokens orientation]

    B1 & B2 --> C[Session active — event capture]

    C --> C1[Tool call events]
    C1 --> C1a[tool_calls row:\ntool_name, input_params,\nresponse, duration, turn_number]

    C --> C2[Turn classification]
    C2 --> C2a{Classify each user message}
    C2a -->|new_request| C2b[Increment turn counter]
    C2a -->|continuation| C2b
    C2a -->|correction| C2c[Record correction event\n+ increment correction counter]
    C2a -->|clarification| C2b

    C --> C3[Phase transitions]
    C3 --> C3a[update_phase events\nbrainstorm → analyze → build → validate]

    C1a & C2b & C2c & C3a --> D[Events stored in real-time]

    D --> E{Session ends?}
    E -->|Normal end| F[checkpoint event]
    E -->|Crash/interrupt| G[Crash detection]

    F --> F1[Compute outcome:\nfirst-try or corrected]
    F1 --> F2[FTR = corrections == 0]
    F2 --> F3[Update session row\noutcome, turns, corrections,\ntokens, duration, ftr]

    G --> G1[Heartbeat timeout detected]
    G1 --> G2[Snapshot session state]
    G2 --> G3[Mark session: interrupted]
    G3 --> G4[Next session start:\noffer to restore]

    F3 --> H[Post-session analytics]
    H --> H1[Update project FTR\nrolling 14-day window]
    H --> H2[Update module correction rate\ngroup corrections by code module]
    H --> H3[Update tool usage stats\naggregated from tool_calls]
    H --> H4[Check change_impacts\nany active measurement windows?]
    H --> H5[Trigger insight generation\nnew patterns? recurring corrections?]

    H5 --> I{Insight threshold met?}
    I -->|Yes| J[Generate recommendation]
    J --> J1[Store recommendation\nwith evidence + projected impact]
    I -->|No| K[Accumulate signal]

    J1 --> L[Observatory updated\nnext app open shows new insight]
```

## Turn classification (idea 04, 07)

Every user message is classified to determine if it's a correction (affects FTR) or normal flow.

| Classification | Signal | Affects FTR? |
|---------------|--------|-------------|
| `new_request` | New task or sub-task | No |
| `continuation` | "yes", "go ahead", "looks good" | No |
| `correction` | "no", "not that", "use X instead", reverts code | Yes |
| `clarification` | Question about scope, requirements | No |

**Classification method:**
- Phase 1: Regex heuristics (keywords, patterns)
- Phase 2: Local model classification (gemma3) — more accurate

### Correction detail capture

When a correction is detected:
```json
{
  "turn": 5,
  "type": "correction",
  "text": "account for 30s clock skew per SDK 4.2",
  "module": "auth/refresh.ts",
  "tool_before": "search('refresh token')",
  "tool_after": "search('clock skew tolerance')"
}
```

This feeds the module correction rate and recommendation engine.

## Crash recovery (idea 11)

```mermaid
flowchart LR
    A[Session active] --> B[Daemon heartbeat\nevery 30s]
    B --> C{ACP still running?}
    C -->|Yes| B
    C -->|Timeout 2min| D[Snapshot session state]
    D --> E[Mark: interrupted]
    E --> F[Next session in same project]
    F --> G[Offer restore:\n'Continue from where you left off?']
    G -->|Yes| H[Load snapshot context]
    G -->|No| I[Fresh session]
```

**Snapshot includes:**
- Active phase and task
- Files being edited
- Last tool calls and their results
- Uncommitted decisions

## Post-session analytics pipeline

After every session completes, these run:

| Step | Input | Output | Latency |
|------|-------|--------|---------|
| FTR computation | Session corrections count | `session.ftr` boolean | Immediate |
| Project FTR update | All sessions in 14-day window | Project FTR trend | < 1s |
| Module correction rate | Corrections grouped by file path | Per-module rate | < 1s |
| Tool usage aggregation | `tool_calls` rows | Usage frequency map | < 1s |
| Change impact check | Active `change_impacts` rows | Updated current metrics | < 1s |
| Insight generation | Accumulated signals | New recommendations | 1-5s (may use local model) |

## Tables written

`sessions`, `events`, `tool_calls`, `change_impacts` (updated), recommendations (generated)
