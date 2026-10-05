# ADR — a metric's SQL lives with its definition, not in `PgStore`

Status: **accepted** 2026-10-05. Decided by the user against the measurements
below. Closes #227.

---

## 1. The rule this is an exception to

`CLAUDE.md`:

> Every deliberate deviation from the established architecture (e.g. not using
> `sensei-bootstrap` from a crate that depends on it, duplicating a type that
> already exists in a shared crate, maintaining a separate hardcoded list that
> the daemon should own) must be raised with the user and documented before
> implementation.

`crates/senseid/src/db/pg_store/` **is** the persistence layer: 606 `pub async
fn` across 24 production modules. Handlers, tasks and the MCP server call
`state.pg.<method>()`, and `api/routes.rs` contains zero raw SQL. That is the
architecture, and it holds everywhere except the population below.

## 2. What is outside it, decomposed

Raw `sqlx_core::query*` in `crates/senseid/src`, counted rather than totalled:

| | count |
|---|---|
| inside `#[cfg(test)]` modules | 273 |
| `tasks/test_support.rs` (the whole module is `#[cfg(test)]`) | 29 |
| **production, outside `PgStore`** | **30** |

The 30 are one cluster plus two strays:

| where | count | what it is |
|---|---|---|
| `tasks/handlers/metrics/*` | 25 | one metric's definition each |
| `api/gateway_config_loader.rs` | 4 | the startup catalog read |
| `federation/mod.rs` | 1 | one lookup |

Three of the 25 are not queries at all on inspection: `metrics/mod.rs` holds
`SELECT current_date` and two `format!` helpers that build a *fragment*
(`day_filter`), so they carry no schema dependency.

## 3. The decision

**The metric computers keep their SQL.** `PgStore` owns DATA ACCESS; a metric
computer owns a DEFINITION that happens to be expressed in SQL.

The distinction is not stylistic. Each of these is:

- **one metric's definition**, written beside the prose stating what it measures,
  how it is keyed and what it excludes;
- **assembled at runtime** — `format!` splicing `super::day_filter(ANCHOR, as_of)`,
  so the windowed and point-in-time forms share one statement;
- **used exactly once**, returning a row type (`DayAgg`, `DayRework`, `DayTtur`)
  that exists for that metric alone.

Folding them in would put 25 single-caller aggregations in
`db/pg_store/metrics.rs`, separate each from the paragraph explaining it, and
express the dynamic composition through a thin accessor that adds a parameter for
every knob. The layer's value — one place to find and migrate data access —
is not served by a method called from one line.

`gateway_config_loader.rs` and `federation/mod.rs` are a weaker case and were
offered as a split; the decision was to leave them, so they are covered by the
same reasoning only insofar as they are stable catalog reads. If either grows a
second caller it should move.

## 4. How the risk is held instead

The argument for folding them in was never aesthetic. It was:

> Every query outside the layer is a query the next schema change can miss. #211
> dropped `folders.project_id` and four readers inside `PgStore` were migrated by
> searching that one directory.

That risk is real and is now held by two mechanisms, measured rather than assumed.

### 4a. Execution — which covers the `format!`-built ones

Each file's first query was broken so it could not PLAN, then only that file's
tests were run. RED means the suite covers it. Measured 2026-10-05:

| file | before | now |
|---|---|---|
| `metrics/session_outcomes` | COVERED | COVERED |
| `metrics/architecture` | COVERED | COVERED |
| `metrics/autonomy` | COVERED | COVERED |
| `metrics/churn` | COVERED | COVERED |
| `metrics/knowledge` | COVERED | COVERED |
| `metrics/planner` | COVERED | COVERED |
| `metrics/session_process` | COVERED | COVERED |
| `metrics/usage` | COVERED | COVERED |
| `metrics/quality` | COVERED | COVERED |
| `metrics/cost` | **not covered** | COVERED |
| `api/gateway_config_loader` | **not covered** | COVERED |
| `federation/mod` | not covered | statically covered (4b) |

Nine of twelve were already exercised. The two gaps closed in `c04e1d18`.

### 4b. Planning — which covers the literal ones

`scripts/check-sql-against-schema.py` PREPAREs every literal SQL statement in the
tree against a live schema, and now runs in CI
(`.github/workflows/rust.yml`, after `dbd deploy --scope default`).

**Its blind spot is exactly this population, so it must not be read as covering
it.** A statement assembled with `format!` reaches the script as a fragment and
is counted as unverifiable rather than passed. Of the 30: **12 are literals** and
covered here; **18 are `format!`-built** and covered only by 4a.

Verified both directions: breaking `federation`'s literal query makes the script
report it by file and line; breaking `cost`'s `format!` query does not.

## 5. What a future reader must do

- Adding a metric? Its SQL goes with it. Add a test that EXECUTES it and asserts
  the value, not merely that the call returned `Ok` — the two pre-existing
  `cost` tests both returned `Ok` without ever reaching the query.
- Adding anything else that reads the database? It goes in `PgStore`. This ADR
  is an exception for metric definitions, not a general licence.
- Changing a column? `check-sql-against-schema.py` will name the literal
  statements. It will NOT name the metric computers — run `cargo test -p senseid
  --bin senseid metrics` for those.

`tasks/handlers/metrics/mod.rs` carries a pointer to this file so the next reader
does not read the cluster as drift.
