---
title: GitHub tracker adapter — plan
description: Decompose the tracker-github spec (H1–H7) into ordered, testable features
type: plan
status: plan
created: 2026-11-09
depends_on:
  - docs/spec/daemon/tracker-github.md
  - docs/decisions.md (D92)
related_issues: [371, 372, 373, 374, 375, 376]
references:
  - docs/spec/daemon/tracker-gitlab.md
  - docs/spec/daemon/tracker-jira-linkage.md
  - docs/spec/daemon/tracker-metrics-writeback.md
  - docs/spec/dojo/targets-and-gates.md
---

# GitHub tracker adapter — plan

## Objective

Implement `docs/spec/daemon/tracker-github.md` r2.3 (H1–H7): a GitHub Project (v2) as a
tracker, sprints from its Iteration field, read with GraphQL and a GitHub token (D92).
H, E (done gate) and C (claims) numbers refer to that spec. Work goes on
`feature/forge-trackers`. Builds run with `-j 1` in the background; `make test-template`
after any `database/` change.

**Before starting any feature,** re-run the claims ledger (C1–C28). Any `FALSE` stops the build.

## Prerequisite (blocks F3, not F1–F2)

- **P1 · Token with `project` scope (human).** `gh auth refresh -s project`, so the
  sensei-hq project can be created and recorded. The run halts here and asks if it is
  missing.

## Features

### G1 · Shared seams (H2, H5 host, H7)
- **Issue:** #371
- **Layers:** `tracker/graphql.rs` (new), `tracker/mod.rs`, `tracker/transport.rs`,
  `tracker/runtime.rs`, `api/handlers/trackers.rs:168`, `tracker/history.rs:64`,
  `tracker/writeback.rs:593`, `tracker/gitlab.rs`, `tracker/forge.rs`, `targets/http.rs`.
- **Depends on:** none. **Nothing GitHub-specific:** no `TrackerKind::Github` yet.
- **Acceptance:**
  - `GraphqlQuery::new` accepts and refuses exactly E4's cases.
  - `GetTransport::graphql` (default `UnsupportedTracker`); `RawGet::post_graphql`
    (default `Err`); `ReqwestRaw::allow_graphql`; `TransportFactory::transport(…, graphql)`
    at the four call sites with `adapter.reads_graphql()`; E5's scope case (a Jira and a
    GitLab transport send nothing).
  - GraphQL error mapping and the `Q_ISSUE` `NOT_FOUND` exception as pure functions (E5).
  - `ReqwestRaw` sends `user-agent: sensei/<version>` (E15).
  - `ValidateError` drops `Copy`, gains `IterationFieldAmbiguous(Vec<String>)`; the
    handler's 422 body carries `fields`; `gitlab.rs:409-417` compiles without `Copy`.
  - `forge_key` returns the host; `forge::tracker_for_key(trackers, key, host)` replaces
    `gitlab::tracker_for_key`; a bare key from another host never reaches a GitLab tracker.
  - E1: the whole senseid suite green; the `Ref` transport tests unchanged.

### G2 · Fixture and recorder (Build order 1)
- **Issue:** #372
- **Depends on:** P1.
- **Acceptance:** the sensei-hq project exists with the H-spec's iterations and item
  types; `scripts/record-github-fixtures.py` records `project`, `issue-on`, `issue-off`,
  `pr`, `no-field` under `crates/senseid/tests/fixtures/github/`, scrubbed by allowlist;
  `fixtures_are_scrubbed` green (E16); H2.4's field names checked against the recording
  and the spec corrected first if any differ.

### G3 · The `github` kind (H1, H3, H4, H6)
- **Issue:** #373
- **Depends on:** G1, G2.
- **Lands as one unit:** `tracker_kind.ddl` `'github'`,
  `database/apply/before/sensei_tracker_kind_github.sql` + `design.yaml`, column comments
  (`trackers.ddl:47-54`, `tracker_tasks.ddl:43`, `sprints.ddl:27-28`), `design.dbml`,
  `make test-template`; `TrackerKind::Github`; `tracker/github.rs`; `adapters()`.
- **Acceptance:** E2, E3, E6, E7, E8, E9, E10, E11, E13, E14; the tracker list returns
  `url` and `iteration_field`.

### G4 · `get_ticket` (H5)
- **Issue:** #374
- **Depends on:** G3.
- **Acceptance:** E12 in full: host-aware selection, the candidate loop (only `Ok(None)`
  advances), uniform 404, 502 with no content, the unmirrored-repo fallback.

### G5 · CLI, app, docs (H7)
- **Issue:** #375
- **Depends on:** G3 (G4 for the docs of `get_ticket`).
- **Acceptance:** E17; every Docs-row surface of H7 updated (D71/D91 notes already done);
  `make test-fast`, the app's tests and clippy green (E18).

### G6 · Live (E19, human-assisted)
- **Issue:** #376
- **Depends on:** G1–G5 and a fine-grained token.
- **Acceptance:** the sensei workspace connected to the sensei project; sprint and
  non-archived issue counts equal the project's; a session on a `12-…` branch links;
  MCP `get_ticket` returns the ticket.
