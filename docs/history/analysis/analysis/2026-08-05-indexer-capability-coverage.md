# Indexer capability coverage — patterns, search, traceability

_2026-08-05 · what the code-graph indexer is *meant* to do vs what it *actually* does, with a
grounded plan. Companion to [`pipeline/code-graph.md`](../spec/pipeline/code-graph.md)._

## Why this exists

The code-graph indexer has four stated purposes beyond "draw a graph": (1) detect
**patterns in use** (adapter/strategy/consumer/…), (2) flag **duplication** and **deviation**
from patterns, (3) be an **LLM-facing search** that beats `sed`/`grep`, and (4) be the
**source of truth for traceability + doc drift**. This doc audits each against the live code,
DDL, and DB — covered / partial / not — and says what to plan.

**The headline, and it is a depth finding.** Three shipped pipeline specs describe systems
that **do not exist**:

| Spec | Claims | Reality |
|---|---|---|
| [`pipeline/patterns.md`](../spec/pipeline/patterns.md) | a 5-source pattern engine (`crates/senseid/src/patterns/…`, DDL with `source`/`pattern_id`/`example_nodes`) | `crates/senseid/src/patterns/` **does not exist**; the code says so — `pattern_effectiveness.rs:4`: *"our detected patterns are behavioral … not best-practice patterns"* |
| [`pipeline/semantic-search.md`](../spec/pipeline/semantic-search.md) | `search/hybrid.rs`, `sensei.node_fts` tsvector, a `SearchResults{confidence,match_mode,fallback_hint}` contract | none of those files/tables/fields exist; search is a keyword router in `query.rs` |
| [`pipeline/traceability.md`](../spec/pipeline/traceability.md) | section-level drift, `trace_links`, `expected/actual` signatures, confidence model | drift is file-level; `traces_to`=0; `expected/actual_signature` NULL on all 1,961 rows |

For a product whose job is detecting doc drift, its own specs are drifted. **Reconciling these
three specs to reality is roadmap item 0** — otherwise every future plan built on them inherits
the same "reality ≠ spec" defect. This audit is the corrected map.

## Capability matrix

| Purpose | State | One-line reality |
|---|---|---|
| Architectural patterns in use (adapter/strategy/observer/…) | **not built** | 0 architectural detections; `detected_patterns` is 948 `rework:<path>` churn rows, `family` NULL for all |
| Duplication detection | **partial** | real cosine tool (`get_duplicates`) works on demand but persists **nothing** — 0 `duplicates`/`similar_to` edges |
| Deviation from patterns | **not built (structural)** | only behavioral churn (rework/correction-prone); no god-node/coupling/layering/dead-code; verifier gate absent |
| LLM search > grep | **partial** | semantic + structural + RRF fusion are real & beat grep for embedded symbols; **no FTS**, lexical arm is `name LIKE`, coverage 26 %, `search` is a keyword router |
| Traceability / doc drift SoT | **partial** | deletion-of-symbol drift is real (287 signals); `traces_to`=0, `covers` is file-stem proximity, drift is file-level, no signature diff |

---

## 1 · Pattern intelligence

### 1a. Architectural patterns in use — **not built**
- **Covered:** only naming/path/syntax heuristics. `classify_file_tag` emits `src|test|e2e|config`
  (processors/types.rs:71); `SymbolKind::Hook`/`Component` come from lexical rules (Svelte runes,
  Vue `use*`). MCP `get_patterns` is an **alias to `get_file_tags`** (mcp/src/lib.rs:976).
- **Not covered:** no recogniser reads `implements`/`extends`/`calls`/`imports` edges to classify
  adapter/strategy/observer/factory/repository/middleware. `match_pattern` is **BM25 file-path
  ranking** (codebase.rs:239) and never touches `detected_patterns`. `detected_patterns.family`
  is NULL for all 948 rows; `inference.god_nodes` **relation does not exist**.
- **Plan:** build `crates/senseid/src/patterns/codebase/` — a recogniser pass over `nodes`+`edges`
  (trait-impl fan → adapter; subscribe/notify → observer; dispatch table → strategy; …) that
  writes rows with a populated `family` and a stable `signature`; add a `source`/`pattern_id`
  column so codebase patterns are distinguishable from churn markers.

### 1b. Duplication — **partial (computed, not persisted)**
- **Covered:** `find_duplicates`/`find_duplicates_scoped` (pg_store.rs:1574) — a real cosine
  self-join over function/method embeddings ≥0.92, project-wide, exposed via `get_duplicates`.
- **Not covered:** **0** `duplicates`/`similar_to` edges are ever written (the enum supports both);
  `derive_signals` never calls it, so no `duplication` anti-pattern rows land; nothing trends it
  or feeds Insights/verifier. It's an ephemeral tool result.
- **Plan:** an indexing/analyzer-tick pass that persists results as `duplicates`/`similar_to`
  edges **and** an `is_anti_pattern` row — making duplication a queryable, trendable graph fact.
  (Depends on embedding survival — §4.)

### 1c. Deviation from patterns — **not built structurally**
- **Covered:** behavioral only — `rework:<file>` churn and `correction-prone` folders
  (analyze.rs:458); `derive_conventions` *describes* the dominant naming style (codebase.rs:445)
  but never flags a violation.
- **Not covered:** god-object (no `god_nodes` table; `communities.god_node_ids` 0/1,814), coupling,
  broken layering, dead code, N+1 — all absent. The spec's verifier gate ("block a commit
  matching an anti-pattern signature") has no convention→signature→verify path.
- **Plan:** once 1a produces a *known* convention with a signature and it reaches
  `lifecycle='rule'`, add a drift checker that flags violations into `inference.drift_items`
  with `fix_pattern_id` → the constructive pattern (FK exists). Add god-node (fan-in×fan-out),
  coupling, layering, dead-code as analyzer steps.

---

## 2 · LLM search better than grep

- **Covered (real, beats grep for embedded symbols):** semantic NN via pgvector cosine + HNSW
  (`semantic_search_nodes`, pg_store.rs:1511; `match_embeddings` fn); **Reciprocal Rank Fusion**
  of lexical+semantic (`fuse_rankings`, query.rs:574); structural `get_callers`/`get_callees`
  over `calls` edges + the `symbols`/`call_graph` views; `context_pack` adds a content-grep arm
  (query.rs:312) — the one place "≥ grep" is structurally guaranteed. MCP exposes `search`,
  `context_pack`, `get_callers`, `get_callees`.
- **Not covered:** **no FTS** — 0 tsvector columns anywhere; the "lexical" arm is `ILIKE '%q%'`
  over `name`+`signature` only (never `content`), i.e. **weaker than grep on bodies/comments**.
  MCP `search` is a brittle keyword-substring **router** (`unified_query`, query.rs:26), not the
  spec's hybrid; no `confidence`/`match_mode`/`fallback_hint`; no confidence-gated grep fallback.
  Embedding coverage **26.4 %**, skewed (`module` 0 %, `const` 3.5 %, `type` 3.6 % vs `method`
  95 %) — concept queries about types/constants can't be answered semantically.
- **Plan:** (a) carry embeddings across reindex + drive coverage to ≥60 % (§4); (b) add a real
  `tsvector` column over `name+signature+docstring+content` with a GIN index, wired as a fusion
  arm so search covers bodies/comments; (c) replace the keyword router with a true multi-mode
  merge and surface the `confidence`/`match_mode` + grep-fallback contract.

---

## 3 · Traceability + doc drift (source of truth)

- **Covered:** a careful drift scanner — `scan_doc_drift` (doc_drift.rs:18, analyzer-scheduled)
  flags a doc's backtick identifier `broken` only when it *was* a real symbol (via
  `symbol_names` history) and no longer resolves — killing false positives (unit-tested). 287
  live `broken` signals, idempotent. `covers` edges feed the `doc_coverage` view.
- **Not covered:** `traces_to` = **0** (never produced — no requirement→code tracing);
  `covers` is **file-stem proximity** (`docs/api/auth.md`↔`src/api/auth.ts`), the N×M match that
  makes 1.77 M edges — it says nothing about *which symbol*; drift is **doc-file level** (0
  `section` nodes; every `drift_items.doc_node_id` is a `doc` file); `expected/actual_signature`
  **NULL on all 1,961 rows** so the `drifted` (signature-changed) status is never produced, and
  `doc_coverage.drifted` is just `code.modified_at > doc.modified_at` (77 % noise).
- **Plan:** emit `section` nodes (§4) → attribute drift to a requirement, not a file; populate
  `expected/actual_signature` and produce `drifted` by comparing the doc's mentioned signature
  to the live node; add a `traces_to` producer (section → the symbols it names); replace
  stem-proximity `covers` with identifier-level covers.

---

## 4 · Shared prerequisites — why these all wait on the code-graph fixes

Three fixes in [`pipeline/code-graph.md`](../spec/pipeline/code-graph.md) are the common
bottleneck; the audits confirmed each is real, not assumed:

1. **Embedding survival across reindex (D3).** Today `process_file` deletes+reinserts a changed
   file's nodes with new UUIDs and NULL embedding (process.rs:754), and `line_start` in the
   identity key mints new rows on any line shift — the root cause of 26 % coverage and unstable
   search. Blocks: semantic search recall (§2), duplication persistence (§1b).
2. **Deterministic, line-independent node ids (D3 identity evolution).** Stable ids are the
   prerequisite for stable search results and stable drift/pattern FKs.
3. **`section` node emission (D5b).** 0 rows today. Blocks: requirement-level traceability (§3),
   section-level drift, and `traces_to`.

So the code-graph spec is not just an Atlas fix — it is the **foundation** the pattern, search,
and traceability capabilities are all built on. Sequencing them before D3/D5b would repeat the
"build on sand" mistake.

## Roadmap (ordered, with dependencies)

0. **Reconcile the three drifted specs** (patterns / semantic-search / traceability) to reality —
   or mark them `roadmap`, not `draft`, in the [spec index](../spec/README.md). *(No code; stops
   future plans inheriting fiction.)*
1. **Land `pipeline/code-graph.md` D1–D6** — correctness/idempotency/durability + embedding
   survival + `section` nodes. *(Foundation for everything below.)*
2. **Persist duplication** (§1b) — `duplicates`/`similar_to` edges + anti-pattern rows. *(needs #1
   embeddings.)*
3. **FTS + search contract** (§2) — tsvector arm + confidence/fallback + real hybrid `search`.
4. **Section-level traceability + signature drift** (§3) — `traces_to` producer, identifier-level
   `covers`, `expected/actual_signature`. *(needs #1 section nodes.)*
5. **Structural pattern recogniser** (§1a) — `patterns/codebase/`, populate `family`. *(needs #1
   graph; enables #6.)*
6. **Deviation + structural anti-patterns** (§1c) — god-node/coupling/layering/dead-code +
   convention-violation verifier. *(needs #5 known conventions.)*
7. **Lifecycle progression** — 948/948 patterns stuck at `suggested`; wire the
   `suggested → gap → rule` promotion so detections become enforceable.

## Verification note

Every "reality" claim here is from the live daemon DB (Postgres :5432) and the current tree on
2026-08-05, cross-checked by two capability audits. Where a capability is "not built," that was
confirmed by a failing `ls`/grep or a `0`-count query, not inferred. Counts drift as indexing
runs; the *shape* (0 architectural patterns, 0 persisted dup edges, 0 sections, 0 `traces_to`,
no FTS) is structural, not a sampling artifact.
