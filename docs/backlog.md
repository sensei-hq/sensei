# Backlog

> Granular, TDD-ready issues. Each can be worked unattended with clear inputs/outputs.
>
> **Every entity implementation must follow TDD (red/green/refactor) and include full coverage tests.** Write tests first, verify they fail, then implement. No entity is complete without tests covering: happy path, edge cases, error handling, and cascade behavior (deletes, FKs).

## Design gaps

| # | Title | Description | Priority |
|---|-------|-------------|----------|
| D1 | Session continuity journey | Idea 11 has no journey. Crash recovery, interrupted work resume, snapshot→continuity memory flow need an end-to-end user story. | HIGH |
| D2 | Collective intelligence journey | Idea 29 has no journey. How insights are batched, anonymized, shared, and what the user sees. Needs system pipeline doc too. | MEDIUM |
| D3 | Drift detection design | When files change, compare with associated documents and traceability matrix. Detect gaps between code and docs. Use inference gateway (idea 28) to analyze via multiple LLM providers/models, consolidate findings in structured format for visualization. Needs: trigger mechanism (on file change via watcher), scan strategy (which docs relate to which code), analysis pipeline (gateway → multi-model → structured output), gap visualization (observatory). Related: idea 13 (doc traceability), idea 28 (inference gateway). | HIGH |

