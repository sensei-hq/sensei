---
name: Work with AI Assistants
type: user-journey
covers: [01, 02, 04, 11, 23, 14, 27]
triggers: [08-system-pipelines/02-session-lifecycle, 08-system-pipelines/01-indexing-pipeline]
screens: [in-acp-invisible]
note: This journey happens inside the AI assistant — sensei has no visible screens here
---

# Journey 4: Work with AI Assistants

> The coding session. Phases, commands, context delivery, personas. The assistant uses sensei's MCP tools to get code right the first time.

## Flow

```mermaid
flowchart TD
    A[User opens Claude Code / Cursor] --> B[SessionStart hook fires]
    B --> C[get_session_context called\n~300 tokens loaded]
    C --> D[Session active — sensei watches]

    D --> E{User gives task}
    E --> F[Assistant calls MCP tools]
    F --> F1[search — find relevant code]
    F --> F2[get_callers — understand usage]
    F --> F3[get_patterns — follow house rules]
    F --> F4[get_lib_docs — library reference]

    F1 & F2 & F3 & F4 --> G[Assistant writes code]
    G --> H{User reviews}
    H -->|Correct — first try| I[FTR = true ✓]
    H -->|Correction needed| J[User redirects]
    J --> K[log_event: correction]
    K --> G

    I --> L[Session ends]
    J -->|Eventually correct| L
    L --> M[checkpoint — outcome recorded]
    M --> N[Session data flows to\nanalytics pipeline]
    N --> O[FTR computed\nInsights updated\nRecommendations regenerated]
```

## Screens (in the AI assistant, not desktop)

This journey happens inside the AI assistant's interface (Claude Code terminal, Cursor editor, etc.). Sensei is invisible — it works through MCP tools and hooks.

### What the assistant sees (via MCP)

```
Session start → get_session_context()
┌──────────────────────────────────────────────────────┐
│ Project: Lumen Cloud · lumen-auth                     │
│ Phase: build                                          │
│ Active task: Fix refresh token rotation               │
│ Patterns to follow: Adapter, Repository               │
│ Recent decisions: clock-skew tolerance (s-2891)        │
│ Open items: device-flow edge case                     │
│ Rules: All handlers wrap ApiError                      │
│ Persona: auth-tests (if configured)                   │
└──────────────────────────────────────────────────────┘
```

### What sensei captures (invisible to user)

```mermaid
sequenceDiagram
    participant U as User
    participant A as Assistant
    participant S as Sensei MCP

    U->>A: "Fix the refresh token rotation bug"
    A->>S: get_session_context()
    S-->>A: project context, patterns, rules
    A->>S: search("refresh token rotation")
    S-->>A: 3 matching symbols
    A->>S: get_callers("refresh_token")
    S-->>A: 5 call sites
    A->>A: writes code
    A->>U: "Here's the fix..."
    U->>A: "No, account for clock skew"
    Note over S: log_event(correction)
    A->>S: search("clock skew tolerance")
    S-->>A: existing pattern
    A->>A: revises code
    A->>U: "Updated with skew tolerance"
    U->>A: "Good"
    Note over S: checkpoint(outcome: corrected)
```

### What gets recorded (per session)

| Event | Captured data |
|-------|-------------|
| Session start | project, folder, ACP, timestamp |
| Tool calls | tool_name, input_params, response, duration, turn_number |
| Corrections | turn where user redirected, what was corrected |
| Phase transitions | brainstorm → analyze → build → validate |
| Outcome | first-try / corrected / abandoned |
| Tokens | input + output token counts |

## Workflow phases

```mermaid
flowchart LR
    B[brainstorm] --> AN[analyze] --> BL[blueprint] --> PL[plan] --> BU[build] --> VA[validate]

    BU -->|corrections| BU
    VA -->|issues found| BU
```

Each phase has a slash command (`/sensei:brainstorm`, `/sensei:build`, etc.) that instructs the assistant on protocol, required MCP calls, and expected outputs.

## Personas & mindsets

```
Session loads context
    │
    ▼
Global mindsets (analyst → developer → tester)
    │
    ▼
Project personas (if cwd matches triggers)
    e.g. "auth-tests" persona fires for lumen-auth/
    │
    ▼
Assistant follows persona rules + project patterns
```

## How to use

1. **Start a session** in your AI assistant. Sensei hooks fire automatically.
2. **Work normally** — the assistant calls sensei MCP tools as needed.
3. **Correct when needed** — sensei records corrections to learn from.
4. **Session ends** — outcome recorded, analytics updated.
5. **Check impact** in the observatory (Journey 3) — did FTR improve?

## Mockup status

This journey has **no sensei screens** — it happens entirely inside the AI assistant (Claude Code terminal, Cursor editor, etc.). Sensei is invisible here, working through MCP tools and hooks.

| Aspect | Mockup exists? | What's needed |
|--------|---------------|---------------|
| Session data model | ✓ `data.js` | Sessions with id, project, title, FTR, corrections, turns, events timeline |
| Session events | ✓ `data.js` | Timestamped events: start, context, edit, test, correction, end |
| Tool call sequence | ✗ | Defined in system journey [session-lifecycle](08-system-pipelines/02-session-lifecycle.md) |
| Crash recovery UX | ✗ | What does the user see when resuming an interrupted session? Needs a prompt design in the assistant (not desktop) — "Sensei detected an interrupted session. Resume from checkpoint?" |

**No desktop mockups needed for this journey.** The data model in `data.js` already defines the session structure. The system journey [session-lifecycle](08-system-pipelines/02-session-lifecycle.md) covers what happens behind the scenes.
