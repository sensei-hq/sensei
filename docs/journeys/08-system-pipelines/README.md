---
name: System Pipelines
type: system-journey
description: Behind-the-scenes processes — no user-facing screens, triggered by user actions or schedules
---

# System Pipelines

What sensei does behind the scenes. These pipelines have no screens — they are triggered by user actions in the user journeys or run on schedules.

## Pipelines

| # | Pipeline | Triggered by | Ideas covered |
|---|----------|-------------|---------------|
| 01 | [Indexing Pipeline](./01-indexing-pipeline.md) | Scan (J2), watcher events (J4), manual re-index | 08, 22, 14, 18, 09, 20 |
| 02 | [Session Lifecycle](./02-session-lifecycle.md) | Session start/end in ACP (J4) | 11, 07, 04, 01 |
| 03 | [Workspace Intelligence](./03-workspace-intelligence.md) | Post-indexing, daily schedule, on-demand | 16, 13, 17, 18 |

## How they connect to user journeys

```mermaid
flowchart TD
    subgraph User["User Journeys"]
        J1[J1 Install]
        J2[J2 Setup]
        J3[J3 Observe]
        J4[J4 Work]
        J5[J5 Understand]
        J6[J6 Improve]
    end

    subgraph System["System Pipelines"]
        S1[01 Indexing]
        S2[02 Session Lifecycle]
        S3[03 Workspace Intelligence]
    end

    J2 -->|scan step| S1
    J4 -->|session start/end| S2
    S1 -->|post-indexing| S3
    S2 -->|session complete| J3
    S3 -->|recommendations| J3
    S1 -->|symbols, graph| J5
    S2 -->|FTR, corrections| J6
    S3 -->|drift, patterns| J5
```
