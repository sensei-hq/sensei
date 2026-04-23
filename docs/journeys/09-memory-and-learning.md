---
name: Memory & Learning
type: user-journey
covers: [30, 27, 11]
triggers: [08-system-pipelines/02-session-lifecycle]
screens: [dashboard-memory-panel, project-memory-view, memory-detail, memory-consolidation, context-pack-tool]
---

# Journey 9: Memory & Learning

> Sensei learns from every session. Memories accumulate, strengthen, surface contextually. Users review, validate, and consolidate what sensei has learned.

## Flow

```mermaid
flowchart TD
    A[Session happens] --> B{Something learned?}

    B -->|Correction observed| C[Memory proposed\nstrength: 1.0]
    B -->|Assistant reports learning| D[Memory proposed\nstrength: 0.5]
    B -->|Recommendation acted on| E[Memory created\nfrom outcome]
    B -->|Session ends| F[Continuity memory\nwhat we were doing]

    C & D & E & F --> G[Memory stored\nwith what + because + scope + references]

    G --> H[Next session starts]
    H --> I[Context assembly checklist]
    I --> I1{First session ever?}
    I1 -->|Yes| I2[Global preferences\nStack knowledge\nCollective insights]
    I1 -->|No| I3{First session\nfor project?}
    I3 -->|Yes| I4[+ Project info\n+ Rules/conventions]
    I3 -->|No| I5{Resuming work?}
    I5 -->|Yes| I6[+ Continuity: where we stopped\n+ Pending work]
    I5 -->|No| I7[+ Previous learnings\n+ Recurring problems]

    I2 & I4 & I6 & I7 --> J[Assemble consolidated markdown]
    J --> K[Deliver to assistant\nvia get_session_context]

    K --> L[Session proceeds]
    L --> M{Context getting bloated?}
    M -->|Yes| N[Context pack tool\nsnapshot + clear + reload memories]
    M -->|No| L

    L --> O[Session ends]
    O --> P[Memories reinforced\nor new ones created]
    P --> A
```

## Screens

### Observatory — Memory indicator

Memory surfaces in the observatory daily view as part of the "system has learned" section.

```
┌──────────────────────────────────────────────────────┐
│  System has learned                                   │
│                                                       │
│  ▎ 2d ago · lumen-cloud · strength 4                 │
│    Don't mock the database in integration tests       │
│    3x reinforced · 0 violations                       │
│                                                       │
│  ▎ 5d ago · global · strength 3                      │
│    Check clock-skew tolerance in refresh flows        │
│    2x reinforced · from s-2891                        │
│                                                       │
│  ▎ new · lumen-cloud · strength 0.5                  │
│    ⚠ Assistant learned: cache invalidation before     │
│    token rotation — awaiting validation               │
│    [Validate]  [Enhance]  [Dismiss]                   │
│                                                       │
│  42 active memories · 3 pending validation            │
│  [View all memories]                                  │
└──────────────────────────────────────────────────────┘
```

**What the user does:** Scan recent learnings. Validate assistant-proposed memories. Click through to full memory view.

### Project view — Memory section

New section in the project view alongside Overview, Graph, Patterns, Sessions, Settings.

```
┌──────────────────────────────────────────────────────┐
│  雲 Lumen Cloud · Memories                            │
│                                                       │
│  Scope: [All]  Project  Module  Task-type  Stack      │
│  Status: [Active]  Pending  Archived                  │
│  Sort: [Strength ▾]  Recent  Category                 │
│                                                       │
│  ┌────────────────────────────────────────────────┐   │
│  │ ●●●●○  Don't mock the database in             │   │
│  │        integration tests                       │   │
│  │        because: mock/prod divergence masked    │   │
│  │        broken migration (Q1 2026)              │   │
│  │        scope: project · modules: database,     │   │
│  │        migration · task: test, fix             │   │
│  │        3x reinforced · 0 violations · 47d old  │   │
│  │        refs: 2 good examples · 1 bad example   │   │
│  │        [View]  [Edit]  [Archive]               │   │
│  ├────────────────────────────────────────────────┤   │
│  │ ●●●○○  Use adapter pattern for auth            │   │
│  │        because: inline auth diverges — sync.ts │   │
│  │        missed audit log (s-2891)               │   │
│  │        scope: project · modules: auth/*        │   │
│  │        2x reinforced · 0 violations · 23d old  │   │
│  │        refs: auth_adapter.rs:14 (good)         │   │
│  │              handlers/auth.ts:42 (bad)          │   │
│  │        [View]  [Edit]  [Archive]               │   │
│  ├────────────────────────────────────────────────┤   │
│  │ ●○○○○  ⚠ Cache invalidation before rotation   │   │
│  │        (pending validation — assistant learned) │   │
│  │        [Validate]  [Enhance]  [Dismiss]        │   │
│  └────────────────────────────────────────────────┘   │
│                                                       │
│  ─────────────────────────────────────────────────    │
│                                                       │
│  Consolidation suggestions (from MOE panel)           │
│  ┌────────────────────────────────────────────────┐   │
│  │ 📎 3 memories about auth can be consolidated:  │   │
│  │    "adapter pattern" + "clock-skew" + "mutex"  │   │
│  │    → Core concept: "Auth module conventions"   │   │
│  │    [Review consolidation]                      │   │
│  └────────────────────────────────────────────────┘   │
│                                                       │
│  Memory stats                                         │
│  42 active · 3 pending · 12 archived                  │
│  Strongest: "Don't mock DB" (4.0)                     │
│  Most referenced: "Adapter pattern" (6 sessions)      │
│  Conflict detected: 0                                 │
└──────────────────────────────────────────────────────┘
```

**What the user does:**
1. Browse memories by scope, status, strength
2. Validate assistant-proposed memories
3. Edit "because" reasoning or scope
4. Archive memories that no longer apply
5. Review MOE consolidation suggestions
6. View memory stats — what's strongest, most referenced

### Memory detail (drill-in)

```
┌──────────────────────────────────────────────────────┐
│  Memory: Don't mock the database in integration tests │
│                                                       │
│  Strength: ●●●●○ (4.0)    Status: active             │
│  Category: correctness     Created: 2026-03-15        │
│                                                       │
│  Because:                                             │
│  Q1 2026: mocked tests passed but prod migration      │
│  failed. Three days of debugging. The mock diverged   │
│  from actual PostgreSQL behavior on nullable FKs.     │
│                                                       │
│  Scope:                                               │
│  Project: lumen-cloud                                 │
│  Modules: database/*, migration/*                     │
│  Task types: test, fix                                │
│                                                       │
│  ─────────────────────────────────────────────────    │
│                                                       │
│  References                                           │
│  ✓ Good: tests/integration/auth_flow.rs:28            │
│    "Real DB connection with test transaction"          │
│  ✗ Bad: tests/unit/mock_refresh.rs:14                 │
│    "Mocked DB — missed nullable FK behavior"          │
│  📝 Evidence: sessions s-2891, s-2895, s-2901         │
│                                                       │
│  ─────────────────────────────────────────────────    │
│                                                       │
│  History                                              │
│  2026-04-22 · reinforced (session s-2901)             │
│  2026-04-15 · reinforced (session s-2895)             │
│  2026-03-20 · reinforced (session s-2891)             │
│  2026-03-15 · created from correction                 │
│    user: "no, don't mock the database"                │
│                                                       │
│  [Edit]  [Archive]  [Convert to guideline]            │
└──────────────────────────────────────────────────────┘
```

**What the user does:**
1. Read the full reasoning and history
2. Click references to navigate to code
3. Edit the "because" or scope if it needs refinement
4. Convert to a project guideline (permanent, max strength)
5. Archive if no longer relevant

### Memory consolidation review

```
┌──────────────────────────────────────────────────────┐
│  Consolidation: Auth module conventions               │
│                                                       │
│  MOE panel suggests merging 3 memories:               │
│                                                       │
│  Source memories:                                      │
│  1. "Use adapter pattern for auth" (strength 3)       │
│  2. "Check clock-skew tolerance" (strength 4)         │
│  3. "Use inFlightMutex for concurrent refresh" (2)    │
│                                                       │
│  Proposed consolidated memory:                        │
│  ┌────────────────────────────────────────────────┐   │
│  │ Auth module conventions                        │   │
│  │                                                │   │
│  │ All auth integrations use the adapter pattern  │   │
│  │ (see auth_adapter.rs:14). Key rules:           │   │
│  │ - 30s clock-skew tolerance for token flows     │   │
│  │ - inFlightMutex for concurrent refresh ops     │   │
│  │ - Never inline auth logic in handlers          │   │
│  │                                                │   │
│  │ Because: inline auth diverges (sync.ts missed  │   │
│  │ audit log), clock-skew causes TokenExpiredError │   │
│  │ at +3s offset, concurrent refresh without      │   │
│  │ mutex causes race conditions.                  │   │
│  │ Evidence: s-2891, s-2895, s-2901 (3 sessions)  │   │
│  └────────────────────────────────────────────────┘   │
│                                                       │
│  Combined strength: 4.0 (highest of sources)          │
│  Original memories will be archived.                  │
│                                                       │
│  [Accept consolidation]  [Edit first]  [Keep separate]│
└──────────────────────────────────────────────────────┘
```

**What the user does:** Review the proposed merge. Accept, edit, or keep memories separate.

### Context pack tool (mid-session)

When context gets bloated in a long session, the assistant or user triggers a context rotation:

```
┌──────────────────────────────────────────────────────┐
│  Context rotation                                     │
│                                                       │
│  Current session context is large. Sensei can:        │
│                                                       │
│  1. Snapshot current progress                         │
│     ✓ Working on: inFlightMutex implementation        │
│     ✓ Completed: skewTolerance, tests passing         │
│     ✓ Pending: mutex implementation, integration test │
│     ✓ In-flight: auth/refresh.ts                      │
│                                                       │
│  2. Reload relevant memories                          │
│     → 5 global memories                               │
│     → 8 project memories (lumen-cloud)                │
│     → 3 module memories (auth/refresh.ts)             │
│     → 2 task-type memories (fix)                      │
│                                                       │
│  3. Clear accumulated noise                           │
│     → Remove stale file reads from context             │
│     → Keep: snapshot + memories + active files         │
│                                                       │
│  [Rotate context now]  [Cancel]                       │
└──────────────────────────────────────────────────────┘
```

**What the user does:** Trigger when session feels sluggish or assistant starts forgetting rules. Sensei snapshots, clears, and reloads with fresh memories.

## How it works

### Memory creation pipeline

```mermaid
flowchart LR
    subgraph Sources
        A1[Corrections]
        A2[Assistant reports]
        A3[Recommendations]
        A4[Session continuity]
        A5[Collective insights]
        A6[User explicit]
    end

    subgraph Processing
        B1[Extract what + because]
        B2[Determine scope]
        B3[Find references in code]
        B4[Set initial strength]
        B5[Check for conflicts]
        B6[Check for consolidation]
    end

    subgraph Storage
        C1[Memory stored]
        C2[References as edges]
        C3[History tracked]
    end

    A1 & A2 & A3 & A4 & A5 & A6 --> B1
    B1 --> B2 --> B3 --> B4 --> B5 --> B6
    B6 --> C1 & C2 & C3
```

### Memory retrieval for sessions

```mermaid
flowchart TD
    A[get_session_context called] --> B[Determine session type]

    B --> B1[Identify: project, stack, task, module]

    B1 --> C[Run checklist]
    C --> C1[Global memories where strength >= 1]
    C --> C2[Stack memories matching project stack]
    C --> C3[Project memories]
    C --> C4[Module memories matching session files]
    C --> C5[Task-type memories matching task]
    C --> C6[Continuity memory if resuming]

    C1 & C2 & C3 & C4 & C5 & C6 --> D[Combine + deduplicate]
    D --> E[Rank by strength]
    E --> F[Format as consolidated markdown]
    F --> G[Deliver to assistant]
```

### Memory lifecycle over time

```mermaid
gantt
    title Memory: "Don't mock the database"
    dateFormat YYYY-MM-DD

    section Lifecycle
    Created from correction     :milestone, m1, 2026-03-15, 0d
    Reinforced (s-2891)         :milestone, m2, 2026-03-20, 0d
    Reinforced (s-2895)         :milestone, m3, 2026-04-15, 0d
    Reinforced (s-2901)         :milestone, m4, 2026-04-22, 0d

    section Strength
    1.0 (new)                   :a1, 2026-03-15, 5d
    2.0 (reinforced)            :a2, 2026-03-20, 26d
    3.0 (reinforced)            :a3, 2026-04-15, 7d
    4.0 (battle-tested)         :a4, 2026-04-22, 30d
```

## How to use

1. **Do nothing** — memories accumulate automatically from corrections and session activity
2. **Validate** — when assistant proposes a memory, review and validate from observatory or project view
3. **Enhance** — edit the "because" reasoning or adjust scope when a memory is too narrow/broad
4. **Consolidate** — review MOE suggestions to merge related memories into concise knowledge
5. **Rotate context** — in long sessions, trigger context pack tool to snapshot + reload with fresh memories
6. **Convert to guideline** — promote a battle-tested memory to a permanent project rule

## Mockup status

| Screen | Mockup exists? | What's missing |
|--------|---------------|----------------|
| Observatory memory indicator | ✗ | **New section** in daily view — recent learnings, pending validation, memory stats |
| Project memory view | ✗ | **New section** in project view — filter/sort memories, validate, consolidate |
| Memory detail | ✗ | **New screen** — full reasoning, references to code, reinforcement history |
| Memory consolidation | ✗ | **New screen** — MOE-proposed merge, preview, accept/edit/keep separate |
| Context pack tool | ✗ | **New screen** (or modal) — snapshot summary, memory reload preview, rotate action |

### Design brief for missing screens

**Observatory memory indicator:**
- Part of "system has learned" section in observatory daily view
- Shows 3-5 most recent memories with strength indicators (● dots)
- Pending validation items have ⚠ badge and Validate/Enhance/Dismiss actions
- Count: "42 active · 3 pending · [View all]"

**Project memory view:**
- New tab in project view: Overview / Graph / Patterns / **Memories** / Sessions / Settings
- Filter bar: scope (all, project, module, task-type, stack) + status (active, pending, archived) + sort
- Each memory shows: strength dots, title, truncated "because", scope tags, reinforcement count, age, reference count
- Actions per memory: View, Edit, Archive
- Bottom section: MOE consolidation suggestions + memory stats

**Memory detail:**
- Drill-in from project memory list or observatory
- Full "because" text, scope breakdown, reference list with clickable code links
- Reinforcement history timeline
- Actions: Edit, Archive, Convert to guideline

**Memory consolidation:**
- Triggered from project memory view when MOE suggests a merge
- Shows source memories side by side with proposed consolidated version
- User can edit the consolidated text before accepting
- Accept archives originals, creates merged memory with combined strength

**Context pack tool:**
- Modal triggered by assistant or user during a session
- Shows: current progress snapshot, memories to reload (grouped by scope), noise to clear
- Single "Rotate context now" action
