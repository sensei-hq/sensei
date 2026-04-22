# 05 — Codebase

> Routes: `/p/{id}/arch` (architecture), `/p/{id}/trace` (traceability), `/r/{id}` and `/p/{id}/r/{rid}` (repo detail)

## Purpose

Understand the shape of the code. Where are the complex areas? How do repos connect? Where are the gaps between requirements and tests? This informs both the developer and the AI — god nodes cause rework, untested areas cause corrections, stale docs cause confusion.

## Architecture (`/p/{id}/arch`)

### Structural View

Force-directed graph visualization of the project's code graph.

**What the developer sees:**
- Nodes = functions and types, sized by connection count
- Edges = calls and imports
- Color = community membership (auto-detected clusters)
- Red/large nodes = god nodes (high degree centrality — many callers/callees)
- Grey/small nodes = orphans (no connections — possibly dead code)
- Orange border = bridges (connect otherwise separate communities)

**Interactions:**
- Hover → tooltip with name, file:line, degree
- Click → detail panel: signature, callers, callees
- Double-click → open file in editor (Tauri)
- Scroll/pinch → zoom
- Click community → isolate and zoom into that cluster

**Why this matters for FTR:** God nodes cause rework because changes have wide blast radius. When the AI edits a god node without understanding its callers, corrections follow. The observatory can warn the AI in session-start context: "PaymentService has 15 callers across 4 communities — changes here need careful impact analysis."

### Doc Drift View

Functions where the code has changed since the associated documentation was last updated.

```
⚠ handleLogin (auth.ts:42)     doc: auth-flow.md    last sync: 14 days ago
⚠ processPayment (stripe.ts:8) doc: payments.md     last sync: 30 days ago
✓ createUser (users.ts:15)     doc: user-guide.md   in sync
```

**Why this matters:** Stale docs confuse the AI. If `auth-flow.md` describes the old login flow, the AI will follow outdated guidance and produce code that needs correction.

### Deployment View (future)

Derived from infrastructure files (Dockerfile, docker-compose, k8s manifests, terraform). Shows services, databases, queues, external APIs as a topology diagram.

Not yet buildable — depends on infra file detection in the daemon indexer.

## Traceability (`/p/{id}/trace`)

### Requirements → Design → Code → Tests

A chain view showing how requirements trace through to implementation and testing.

```
Requirement        Design          Code         Tests
AUTH-001           auth-flow.md    login.ts     ✓ 3 tests
User login         (acme-api)      (acme-api)

PAY-001            payments.md     stripe.ts    ✗ 0 tests  ← gap
Stripe payments    (acme-api)      (acme-api)

UI-003             —               Button.tsx   ✓ 1 test
Button redesign    ← gap           (acme-ui)
```

**Gaps are the insight.** A requirement with no tests is a risk. A code file with no tracing requirement might be undocumented scope creep.

Works across repos: requirement doc in one repo, implementation in another. Connected via project membership and graph edges (COVERS, MENTIONS_FN).

**Why this matters for FTR:** When the AI works on code that traces to a requirement, it should know the requirement. When tests are missing, the AI should be told to write them. The traceability view identifies these gaps so the developer can add rules or session-start context.

### Filter

- By repo (within project)
- By coverage status (fully traced / partial / no tests)
- "Gaps only" toggle

## Repo Detail (`/r/{id}` and `/p/{id}/r/{rid}`)

### What the Developer Sees

Single repo view with:

- **Header:** name, path, language, last indexed
- **Stats:** functions, types, files, average complexity
- **Complexity hotspots:** top functions by cyclomatic complexity
- **Community structure:** mini force-directed graph
- **Recent sessions:** sessions that touched this repo
- **Index status:** last indexed time, error count, re-index button

The `/p/{id}/r/{rid}` variant adds project breadcrumbs and back-navigation.

### Indexing

Indexing status is inline on the repo detail page:

- Current status: idle / indexing / queued / error
- Progress bar during active indexing (SSE-driven)
- Error list: which files failed and why
- Re-index button

The SSE event pattern (queue → started → progress → completed/failed) drives real-time updates. See `archive/queue-worker-sse-pattern.md` for the implementation pattern.

## What's Built

- Architecture structural view: implemented (D3 force-directed graph with communities, god nodes, zoom)
- Doc drift view: implemented
- Repo detail: implemented (stats, graph, complexity hotspots, indexing)
- Traceability view: partially implemented (route exists, data model partially there)

## What Needs to Be Built

| Feature | Dependency |
|---------|-----------|
| Deployment view | Infra file detection in daemon indexer |
| Data flow view | API endpoint + DB table extraction in daemon |
| Traceability data | Doc classification in daemon (auto-classify by folder/content) |
| Traceability cross-repo | COVERS/MENTIONS_FN edge types in graph |
| God node warnings in session context | Export graph analysis to MCP session-start |
