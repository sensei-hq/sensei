# 巻 · Pipeline · Traceability

> **Status: roadmap (partial).** The deletion-of-symbol drift scanner is real and careful, but
> as of 2026-08-05: `traces_to` = 0 rows, `covers` is file-stem proximity (not identifier-level),
> drift is doc-**file** level (0 `section` nodes), and `expected/actual_signature` are NULL on
> all rows (no signature-diff / `drifted` status). Requirement-level traceability depends on the
> code-graph fixes (`section` nodes, deterministic ids). Verify against the code. See the
> [indexer capability roadmap](../../analysis/2026-08-05-indexer-capability-coverage.md).

**Owner files:**
- Scanner: `crates/senseid/src/db/pg_store.rs::scan_project_doc_drift`
- Extractor: `crates/senseid/src/analysis/doc_drift.rs` — identifier extraction (unit-tested; no DB round-trip)
- Persistence: `sensei.drift_items` (proposed table) + reads from `sensei.nodes`
- Fix action: `crates/senseid/src/api/handlers/traceability.rs`

## Purpose

Traceability answers *"do the docs still match the code?"* When a
README says `` `LoginService.authenticate()` `` and the codebase
has since renamed it to `authenticateWithCredentials`, the doc is
lying. Traceability catches those breaks by scanning doc nodes for
backtick-wrapped identifier mentions and cross-referencing them
against the live code graph.

The output is a list of **drift items** with a confidence score:

- **High-confidence drift** — the code identifier is gone entirely.
  Auto-applies a fix suggestion when the target rename is
  unambiguous.
- **Medium-confidence drift** — the identifier still exists but
  its signature changed. Presented to the user for review.
- **Low-confidence drift** — the mention might be a false positive
  (generic word, external identifier); presented in the "review
  batch" list.

Kanji is 巻 — *scroll / record*.

## Data invariants

- `sensei.drift_items` — one row per broken reference:
  - `id` uuid
  - `project_id` uuid
  - `doc_node_id` uuid (the doc where the mention lives)
  - `doc_path` text (readable path — `docs/README.md`)
  - `line_number` int
  - `mentioned_identifier` text (`LoginService.authenticate`)
  - `expected_signature` text (nullable — what it USED to be, if known)
  - `actual_signature` text (nullable — what it IS now, if a
    likely-match was found)
  - `confidence` enum `high | medium | low`
  - `state` enum `open | dismissed | fixed | resolved_auto`
  - `suggestion` text (proposed fix — model-generated via
    [[pipeline/insight-copy]] with `kind = drift_fix`)
  - `detected_at`, `resolved_at` timestamptz
- Signature comparison happens in
  `analysis/doc_drift.rs` (already unit-tested per the daemon
  code). The DB round-trip is only for persistence and cross-
  reference lookup.
- **Branch-scoped.** Drift items are recorded against the active
  branch (see [[pipeline/capture]] branch versioning). Switching
  branches flips the drift-item list to the target branch's
  state.
- `expected_signature` and `actual_signature` power the
  Traceability detail drawer's Expected-vs-Actual diff view. Both
  are nullable — a `broken` reference that doesn't have a likely
  match shows only `expected` (from the doc); the code side reads
  "no match found".

## Signals produced

| Signal | Consumer |
|---|---|
| `open` drift count per project | Project Overview stat (`Doc drift`) |
| `open` high-confidence drift | Insights Now column (violation card) OR auto-fix action if enabled |
| Drift list per project / global | Observatory Traceability screen + Project Traceability screen |
| `expected` vs `actual` diff | Traceability detail drawer |
| Repeated drift on the same identifier | Recommendation candidate (see [[pipeline/insights]] source #5) |

## Scanning

`scan_project_doc_drift(project_id)` runs on the analyzer tick when
either:

- Doc nodes changed since last scan (a README was edited), OR
- Code nodes changed since last scan (a function was renamed), OR
- The daily full-refresh window opened (see [[pipeline/analyzer]]).

Per doc node:

1. Extract backtick-wrapped identifiers via
   `analysis::doc_drift::extract_identifiers()`.
2. For each candidate identifier:
   - Look up in `sensei.nodes` scoped to the same project + active
     branch.
   - **Match found + signature identical** → not drift; skip.
   - **Match found + signature differs** → medium confidence,
     record `expected` (from doc context if available) and
     `actual` (from live graph).
   - **No match** → check for a probable rename (fuzzy match on
     name, or move detection via `git log --follow`). If found,
     high confidence with a suggested fix. If not, low
     confidence.
3. Idempotent upsert — the same doc scanned twice produces the
   same rows.
4. Resolved rows (identifier now resolves back to a live node)
   transition to `state = fixed` at the next scan without user
   action; the row stays for a short retention so the Impact
   screen can credit the fix.

## Fix action

`POST /api/traceability/{drift_id}/apply-fix`

- For `resolved_auto`-eligible items (high confidence, unambiguous
  rename), the daemon can apply the fix directly by editing the
  doc file with the corrected identifier.
- For everything else, the endpoint records the user's decision
  (`fixed` with `note`) and leaves the doc edit to the user.
- Applying a fix schedules a follow-up drift scan on the affected
  doc so the row transitions to `fixed → resolved_auto` when the
  next scan confirms.

## Done gate

- On Jerry's live data, `scan_project_doc_drift` returns
  `{broken_added, resolved, docs_scanned}` and the counts stay
  stable across ticks when nothing changed.
- The Project Overview `Doc drift` stat matches
  `count(*) from sensei.drift_items where state = 'open' and project_id = X`.
- A README rename from `foo()` to `bar()` produces a
  high-confidence drift within one tick of either doc or code
  update.
- A rename with clear evidence (git-log follow) auto-fixes when
  auto-fix is enabled in Preferences; otherwise it surfaces as a
  suggested-fix card the user accepts.
- Drift-item copy (title, suggestion) comes through
  insight-copy with `kind = drift_title` / `drift_fix` — fallback
  templates otherwise.
- Branch switch flips the drift list to the target branch's
  state.
- Fixed drift rows advance to `resolved_auto` on the next scan
  without user action.

Optional check:
```
psql -A -t -c "select confidence, state, count(*)
                 from sensei.drift_items
                 where project_id = (select id from sensei.projects where name = 'sensei')
                 group by confidence, state" -d sensei
```

## Wrong gate

- **`Doc drift` stat on Project Overview shows 3, but the
  Traceability screen list shows 5 items.** Aggregate query and
  list query diverged.
- **A backtick word like `` `TODO` `` or `` `TBD` `` becomes a
  drift item.** Extractor is too aggressive; add a stopword list
  in `analysis/doc_drift.rs`.
- **Auto-fix rewrites a doc but the git-log evidence was weak.**
  Auto-fix threshold should be strict; the "unambiguous rename"
  bar is `follow_count >= N` AND `name_similarity >= 0.85`.
- **Drift items persist after the identifier reappears.** The
  resolve pass isn't running.
- **Branch switch doesn't update the drift list.** Branch column
  filter regressed; the active view stays on the previous branch.
- **The same identifier shows up as drift in every doc that
  mentions it.** Cross-referencing is per-doc but the mention
  should also be de-duped when the underlying code change
  affects many docs uniformly.
- **`expected` and `actual` signatures are both null.** Extraction
  succeeded but neither side got populated; renders as a useless
  card.

## Bidirectional linking

Traceability isn't just "docs mention code identifiers." It's a
**two-way binding**: docs reference code AND code references
docs. Both directions need to survive edits.

- Doc → code: extracted from backtick identifiers, links, and
  `docs/README.md`-style anchors.
- Code → doc: extracted from block-doc comments referencing
  design docs (`// see docs/design/02-daemon.md`) and from
  git-commit trailers (`Refs: docs/analysis/...`).
- Persisted in `sensei.trace_links` as one row per link with
  `direction`, `source_node_id`, `target_node_id`,
  `confidence`, `detected_at`.

`get_trace_links(node)` returns everything pointing at (or
pointed at by) a node — used by:

- Traceability screen expansion drawer.
- [[pipeline/context-delivery]] ranking (traceability boost —
  docs that link a code path get their referenced code loaded
  into context).
- Solution-scope architecture view — cross-repo doc→code edges.

## Doc-doctor (lint for docs)

The traceability scanner runs during the analyzer tick and
emits **lint findings** — not just drift, but doc quality:

- **Stale doc** — doc unchanged in > N days AND references code
  that changed in that window. Threshold configurable (default
  90d).
- **Undocumented public** — a public function / type / API
  endpoint with no doc reference. Only when the folder has a
  `documented_at`-level policy (see enforcement modes).
- **Inconsistent** — same identifier referenced with different
  parameter shapes in two docs.
- **Ambiguous** — an identifier that resolves to multiple nodes;
  the doc doesn't disambiguate.
- **Broken link** — the identifier doesn't resolve at all AND
  no likely rename found.

`sensei.doc_lint_findings` — one row per finding + severity
(`error | warning | info`).

Findings surface:

- On Insights (Now / Soon) when severity is warn / error.
- On the Traceability screen as a filter.
- Optionally, during `/sensei:validate` phase as a gate.

## Auto-generated docs (fallback)

For code without any doc coverage, sensei can generate a doc
stub via [[pipeline/inferencing]] `reasoning` chain:

- Reads the function signature + surrounding context (up to a
  bounded token budget).
- Emits: a one-paragraph description, the params, the returns,
  and side effects (from function-shape analysis in
  [[pipeline/testability]]).
- Stub written to `sensei.doc_stubs` — **not** the actual
  source file until the user approves.
- User can promote a stub to a real doc from the Traceability
  screen; sensei writes to the appropriate location (repo
  README, `docs/…`, or an inline block comment based on the
  project's derived convention).

## Enforcement modes

Per project (settable in Project → About OR a project-scope
rule in [[pipeline/governance]]):

- **audit-only** — findings surface as info; nothing blocks.
- **warn** — findings surface with a warn severity; validate
  phase passes with warnings.
- **block** — findings surface with error severity; validate
  phase fails until resolved.

Default: `warn`. Compliance packs (HIPAA/PCI/SOC2) may raise
specific doc-lint rules to `block`.

## Related

- [[pipeline/capture]] — doc + code node ingestion; branch versioning
- [[pipeline/analyzer]] — schedules `scan_project_doc_drift`
- [[pipeline/insights]] — repeated drift becomes recommendation source #5
- [[pipeline/insight-copy]] — drift titles + fix suggestions
- [[screen/observatory-traceability]] — primary consumer
- [[screen/project-traceability]] — project-scoped view + Expected-vs-Actual drawer
- [[screen/project-overview]] — stat consumer
