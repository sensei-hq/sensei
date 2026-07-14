# Layer · data

> **Serves:** every objective — the data layer is the substrate the whole
> [core loop](../requirements/vision.md#the-core-loop) turns on. Owns the schema,
> the model, and the DB conventions.

## What it is

One PostgreSQL database, `sensei`, on port **7744** (single mode — no dev/prod
split). The daemon owns runtime migrations; the DDL under
[`database/ddl/`](../../database/ddl/) is the declarative source of truth
(applied with **dbd**). DDL is organised by object type (`enum/`, `table/`,
`function/`, `procedure/`, `view/`) then by schema.

## The schemas

```mermaid
flowchart TD
    subgraph sensei["sensei — the knowledge core (67 tables)"]
        N[nodes · edges<br/>code graph] --- F[folders · projects<br/>one repo = one owner]
        M[memories] --- L[libraries · library_pages]
        R[rules · file_tags · communities]
    end
    subgraph activity["activity — captured behaviour (9)"]
        S[sessions · turns] --- E[assistant_events · tool_calls]
        TB[transcript_turns]
    end
    subgraph inference["inference — learned output (11)"]
        REC[recommendations] --- DP[detected_patterns]
        COR[corrections] --- RT[reasoning_traces]
    end
    subgraph gateway["gateway — LLM routing config (7)"]
        RO[routers · models · chains]
    end
    subgraph dojo["dojo — team layer (separate service DB)"]
        MB[memberships · roles · engagements · artifacts]
    end
    activity --> inference
    sensei --> inference
    inference -.->|insight-copy| sensei
    gateway -.->|routes inference| inference
```

| Schema | Owns | Notes |
|---|---|---|
| **sensei** | code graph (`nodes`/`edges`), `folders`/`projects`, `memories`, `libraries`, `rules`, `communities`, `file_tags`, `insight_copy` | the knowledge core; **one repo = one project = one owner** (see [daemon](daemon.md)) |
| **activity** | `sessions`, `turns`, `assistant_events`, `tool_calls`, `transcript_turns` | the captured pair-behaviour — raw material for FTR |
| **inference** | `recommendations`, `detected_patterns`, `corrections`, `reasoning_traces` | what the analyzer learns |
| **gateway** | `routers`, `models`, `chains` | table-driven LLM routing config the daemon loads at boot |
| **dojo** | `memberships`, `roles`, `engagements`, `artifacts`, `incidents`, `audit_events` | the team layer — lives in the **dojo-mind** service DB, not the local `sensei` DB |
| public | `logs` (TTL-pruned) | structured-log sink |
| history / staging | change history · seed import staging | seeding uses timestamp-guarded import procedures |

## Conventions (the rules that keep the schema honest)

- **DDL-source-first.** Edit the `.ddl` first, then apply — otherwise the
  daemon's boot auto-apply recreates the old object. The daemon reads the
  *released* DDL bundle (`database@vVERSION`); to see local DDL changes use
  `make bump` or `SENSEI_DDL_DIR`.
- **Apply via dbd** (`dbd deploy`/`apply`/`graph`), never `dbd combine`'s dump.
  Pre-release additive columns go through **`dbd reconcile`** (incremental);
  `deploy` is a declarative snapshot and won't ALTER-add. Enum variants deploy
  **alphabetically** — never rely on declaration order; rank with a CASE.
- **Full-DDL, no ALTERs** in source — a table's `.ddl` is its whole definition;
  migrations are dbd's job.
- **Seeding is idempotent + guarded** — staging tables + import procedures with
  a timestamp guard so a reseed never overwrites production data.

## One-owner invariant

Every file is owned by exactly one folder — the repo/git-root. Structural
subfolders (`kind='folder'`) are members with a role/kind and own **no** code
nodes. This is enforced at scan-classification + a self-healing reconcile
(`dedup_structural_folder_nodes`); see [daemon](daemon.md#scan--the-code-graph).

## Where the gaps are

Orphaned tables (`inference.insights`/`insight_batches` — no writer, superseded
by `insight_copy`) and empty-by-design dojo tables (external-blocked). See
[`../requirements/open-issues.md`](../requirements/open-issues.md) G7.

## Source detail

Deeper rationale (model relationships, indexing) currently in
[`../design/02-daemon.md`](../design/02-daemon.md) §Metadata model — folds into
this doc as the restructure completes.
