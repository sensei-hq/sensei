---
name: Workspace Intelligence
type: system-journey
covers: [16, 13, 17, 18]
triggered-by: [02-setup-discovery (scan), indexing-pipeline (post-processing), scheduled]
---

# System: Workspace Intelligence

> Cross-repo analysis: conformance checking, doc traceability, pattern knowledge, architecture health.

## Pipeline flow

```mermaid
flowchart TD
    A[Trigger] --> A1{Source?}
    A1 -->|Post-indexing| B[Single repo updated]
    A1 -->|Scheduled — daily| C[Full workspace scan]
    A1 -->|Manual| D[User requests analysis]

    B & C & D --> E[Cross-repo analysis]

    E --> F[Doc traceability — idea 13]
    F --> F1[Scan doc files\nREADME, docs/, .sensei/]
    F1 --> F2[Extract doc-to-code links\nfunction refs, file refs, API refs]
    F2 --> F3[Check link targets\ndo referenced symbols still exist?]
    F3 --> F4{Drift detected?}
    F4 -->|Yes| F5[Flag as stale\nrecord in drift_items]
    F4 -->|No| F6[Mark as current]

    E --> G[Pattern knowledge — idea 17]
    G --> G1[Load detected_patterns\nfrom all repos in project]
    G1 --> G2[Match against known catalogs\npatterns.dev, GoF, resilience]
    G2 --> G3[Identify gaps\ncommon patterns NOT present]
    G3 --> G4[Score pattern adherence\n% sessions following pattern]
    G4 --> G5[Generate pattern recommendations\nsuggested → gap → rule lifecycle]

    E --> H[Architecture conformance — idea 16]
    H --> H1[Cross-repo call edges\nrepo A calls repo B]
    H1 --> H2[Boundary violations\nunexpected cross-repo dependencies]
    H2 --> H3[Dependency health\ncircular deps, unused deps]
    H3 --> H4[Service map\nwhich repos talk to which services]

    E --> I[Testability scoring — idea 18]
    I --> I1[Per-function complexity\ncyclomatic + cognitive]
    I1 --> I2[Composability analysis\npure functions vs side effects]
    I2 --> I3[Test coverage gaps\nexported but untested]
    I3 --> I4[Testability score\nper file, per module]

    F5 & F6 & G5 & H4 & I4 --> J[Store results]
    J --> J1[Update folders.props\nwith health metrics]
    J --> J2[Update projects.props\nwith aggregate scores]
    J --> J3[Generate recommendations\nif thresholds crossed]

    J3 --> K[Recommendations feed\ninto observatory + project views]
```

## Doc traceability detail (idea 13)

```mermaid
flowchart LR
    A[Doc file\nREADME.md] --> B[Extract references]
    B --> B1["compute_metrics()"\nfunction reference]
    B --> B2["src/analytics.ts"\nfile reference]
    B --> B3["POST /api/events"\nAPI reference]

    B1 --> C{Symbol exists?}
    C -->|Yes, signature matches| D[Current ✓]
    C -->|Yes, signature changed| E[Drifted ⚠]
    C -->|No, deleted| F[Broken ✗]

    E --> G[Drift item:\nwhich doc, which symbol,\nwhat changed, when]
    F --> G
```

**Drift actions surfaced to user:**
- "README references `compute_metrics(query)` but signature changed to `compute_metrics(query, window)`"
- "API doc references `POST /api/events` which was renamed to `POST /api/v2/events`"
- Action: "Update doc" → sends prompt to ACP

## Pattern knowledge detail (idea 17)

Sources for pattern matching:

| Source | Content | How used |
|--------|---------|----------|
| patterns.dev | Industry standard patterns | Match against detected code structures |
| GoF catalog | Gang of Four design patterns | Classify detected patterns |
| Project rules | User-promoted patterns | Enforce in sessions |
| Session history | Patterns that correlate with high FTR | Recommend adoption |

**Pattern lifecycle integration:**
```
Industry catalog  →  Match against codebase  →  suggested
                                              →  gap (recommended but absent)
User promotes    →  rule (enforced in sessions)
Session data     →  correlate FTR with pattern use → evidence for promotion
```

## Architecture conformance detail (idea 16)

```mermaid
flowchart TD
    A[All repos in project] --> B[Build cross-repo edge map]
    B --> C[Classify edges]
    C --> C1[Expected: API calls frontend → backend]
    C --> C2[Unexpected: frontend → database directly]
    C --> C3[Circular: A → B → A]

    C2 --> D[Boundary violation alert]
    C3 --> E[Circular dependency alert]

    B --> F[Service map]
    F --> F1[Which repos use PostgreSQL?]
    F --> F2[Which repos call Stripe?]
    F --> F3[Which repos share no edges? — isolated]
```

## Testability scoring detail (idea 18)

Per-function scoring:

| Factor | Weight | Measurement |
|--------|--------|-------------|
| Cyclomatic complexity | 0.3 | Branches, loops, conditions |
| Side effects | 0.3 | IO, mutations, global state |
| Dependency count | 0.2 | Number of imports/injected deps |
| Test existence | 0.2 | Matching test file/function found |

Score: 0-100. Files below 40 flagged as "hard to test." Surfaced in project overview and code graph (testability overlay — potential addition to the 5 existing overlays).

## Triggered by

| Trigger | Scope | Frequency |
|---------|-------|-----------|
| Post-indexing | Single repo | After each index run |
| Scheduled | Full workspace | Daily (configurable) |
| Manual | Specific project | On-demand from UI/CLI |

## Tables read/written

**Read:** `symbols`, `call_edges`, `imports`, `detected_patterns`, `folders`, `projects`, `referenced_libraries`

**Write:** `folders.props` (health metrics), `projects.props` (aggregate scores), recommendations (if thresholds crossed)
