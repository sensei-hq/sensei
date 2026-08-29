# 紋 · Pipeline · Patterns

> **Status: roadmap (not built).** This describes an intended pattern engine. As of
> 2026-08-05 the owner files below **do not exist** (`crates/senseid/src/patterns/`), and
> `inference.detected_patterns` holds behavioral churn (`rework:<path>`, `family` NULL), not
> architectural patterns. Verify against the code before relying on this. See the
> [indexer capability roadmap](../../analysis/2026-08-05-indexer-capability-coverage.md).

**Owner files:**
- Detection: `crates/senseid/src/patterns/detect.rs`
- Codebase pattern recognisers: `crates/senseid/src/patterns/codebase/`
- Library pattern extractor: `crates/senseid/src/patterns/library/`
- Registry ingester: `crates/senseid/src/patterns/registry/` (patterns.dev + custom)
- Architectural options: `crates/senseid/src/patterns/options/`
- Derived-from-usage: `crates/senseid/src/patterns/derived/`
- Persistence: `inference.detected_patterns`, `sensei.pattern_registries`, `sensei.pattern_choices`
- MCP: `crates/mcp/src/tools/get_patterns.rs`, `match_pattern.rs`,
  `get_pattern_options.rs`, `get_project_conventions.rs`

**Companion design doc:** `docs/archive/ideas/17-pattern-knowledge.md` (2026-04-17)

## Purpose

Patterns are how the assistant knows *how to build here*. Without
pattern knowledge, the assistant makes suboptimal choices and the
user has to correct it — the exact loop sensei exists to shorten.
Patterns come from **five sources**, each answering a different
question:

| # | Source | Question it answers |
|---|---|---|
| 1 | **Codebase** (detected) | What shapes already exist in this repo I should follow? |
| 2 | **Library** (from docs / usage) | What are the conventions for using this library correctly? |
| 3 | **Industry / Registry** (curated) | What are the established patterns for problems like this? |
| 4 | **Architectural options** (chooseable) | What are my options here, what are the tradeoffs? |
| 5 | **Derived** (from project usage) | What has this project consistently done — what's the implicit standard? |

They also come in three modes:

- **Design / positive patterns** — shapes to follow (adapter,
  observer, rokkit's data-driven-component convention, i18n via
  messages collection).
- **Anti-patterns** — shapes to avoid (duplication, spaghetti
  coupling, god objects, broken layering, dead code).
- **Optimization opportunities** — patterns that suggest a
  measurable improvement (n+1 query, sync-on-hot-path, missing
  index).

Kanji is 紋 — *pattern / crest*.

## Data invariants

- `inference.detected_patterns` — one row per detected instance:
  - `id` uuid, `project_id` uuid, `folder_id` uuid nullable,
  - `pattern_id` text (`codebase.adapter`, `lib.rokkit.data_driven_component`,
    `registry.patterns_dev.compound_components`,
    `option.string_handling`, `derived.rest_naming`,
    `anti.duplication`, `opt.n_plus_one`, …),
  - `source` enum `codebase | library | registry | option | derived | anti | opt`,
  - `family` enum `design | anti | opt` (option is
    always in `design`; anti/opt are their own families),
  - `library_id` uuid nullable (populated for `source=library`),
  - `registry_id` uuid nullable (populated for `source=registry`),
  - `instances` int (count of instances found — 0 for
    registry/option that haven't been picked yet),
  - `example_nodes` uuid[] (references into `sensei.nodes`),
  - `confidence` numeric 0..1,
  - `ftr_delta_observed` float nullable,
  - `state` enum `detected | promoted | dismissed`,
  - `detected_at`, `state_changed_at` timestamptz,
  - `signature` text (dedup by instance shape).
- `sensei.pattern_registries` — one row per external registry:
  - `id` uuid, `name` text (`patterns.dev`), `url` text,
    `categories` text[], `indexed` bool, `last_ingested_at`.
- `sensei.pattern_choices` — one row per architectural option a
  user has picked:
  - `id` uuid, `project_id` uuid, `option_category` text
    (`string_handling`, `data_loading`, `state_management`, …),
    `chosen_pattern_id` text, `chosen_at`, `chosen_by`,
    `rationale` text.
- `sensei.promoted_patterns` — patterns explicitly promoted to
  rule via the [[pipeline/governance]] ladder.
- All human-facing titles / bodies / snippets go through
  [[pipeline/narration-cache]].

## The five sources

### 1. Codebase (detected)

Small AST/graph passes over `sensei.nodes` + `sensei.edges`.
Recognisers per pattern (adapter, plugin, observer/subscriber,
strategy, factory, repository, decorator, trait/mixin).

**Custom codebase patterns** — emerge from community detection
((memory: project_ingest_scan_bug_batch) memory). A labeled cluster
where members expose the same trait / shape becomes a
`codebase.custom.*` pattern. User can name it from Project
Patterns.

Example: sensei's `custom.assistant_adapter_trait` — every
assistant family is an adapter, one file per family, following
the trait in `adapters/mod.rs`. This is what stops a new
assistant integration from becoming spaghetti.

### 2. Library (from docs / usage)

Every library sensei tracks (see [[pipeline/libraries]]) can
carry usage patterns — conventions specific to that library:

- **Rokkit** — data-driven-component pattern (Props interface
  with items/options/fields; ProxyTree/ProxyItem internals;
  Navigable/Navigator for keyboard; data-attribute HTML
  conventions).
- **Kavach** — auth flow conventions.
- **dbd** — schema migration conventions.

Extraction, ordered by preference:

1. **From library docs** — if `llms.txt` / component docs
   include a patterns section, ingest it during the library
   ingestion step (see [[pipeline/libraries]]).
2. **From codebase usage** — the indexer sees how the library is
   used in the project; cross-reference with the library's API
   to derive conventions.
3. **From user declaration** — user points at a reference
   component; sensei extracts the pattern from it and asks
   clarifying questions.
4. **From library author** — `patterns.yaml` shipped with the
   package. The ideal source; sensei defines a small schema for
   library authors to adopt.

### 3. Industry / Registry (curated)

External curated collections like patterns.dev. Configured in
`.sensei/pattern-registries.yaml`:

    registries:
      - name: patterns.dev
        url: https://www.patterns.dev
        indexed: true
        categories: [rendering, design, performance]

Ingested via [[pipeline/libraries]] ingestion machinery —
each pattern becomes a row in `inference.detected_patterns` with
`source: registry` and `instances: 0` (until it's chosen /
matched into the project).

Each pattern includes: name, category, problem it solves, when
to use, when NOT to use, tradeoffs.

### 4. Architectural options (chooseable)

Some patterns are **choices with tradeoffs**, not detections.
The user picks one and the assistant follows it consistently.

Option categories with worked examples (from the archive doc):

- **String handling**: hardcoded / messages collection /
  runtime i18n. Tradeoff: setup vs. flexibility.
- **Data loading**: inline / SSR loader separation / client-side
  fetch / reactive streams. Tradeoff: simplicity vs. capability.
- **State management**: local state / context / signals / store.
- **Routing**: file-based / config-based / declarative.
- **Testing**: unit-only / integration-heavy / e2e-only /
  mixed pyramid.
- **Error handling**: throw / result-type / callback-with-error.

Flow:

1. **First time** in a fresh area: assistant calls
   `get_pattern_options(category)`; picks or asks the user to
   pick.
2. **User picks**: recorded in `sensei.pattern_choices`;
   propagated as a rule in [[pipeline/governance]] with source
   `chosen:option:{category}`.
3. **Subsequent times**: assistant reads
   `get_project_conventions()` first; sees the chosen option;
   follows without asking again. If a task looks *different*
   (e.g., a throwaway mockup), the assistant surfaces the
   difference before proceeding.

### 5. Derived (from project usage)

The most powerful source: what the project has ALREADY done. The
indexer detects consistent patterns:

| Signal | Derived pattern |
|---|---|
| All pages load data in server loaders | SSR data-separation is the standard |
| All strings go through `$t(…)` | i18n is active |
| All API routes follow RESTful naming | REST conventions |
| All components use rokkit's Props interface | rokkit data-driven-component convention |
| Tests use `describe/it/expect` with fixtures in `__fixtures__/` | Test structure pattern |

Derived patterns become **implicit guardrails** — surfaced from
`get_project_conventions()` and applied without the assistant
having to be told.

**Auto-promotion policy** (open decision — bias: yes with a
threshold): after N consistent usages (default 5) the derived
pattern advances from `detected` to `promoted` as a rule with
`source: derived`. User can override / adjust.

## Assistant-context surface

Patterns are the single most important input the assistant sees
at session start when it's about to add a new feature. The MCP
tools:

- `get_patterns(project, source?)` — returns patterns filtered
  by source. Default: all sources, ordered by
  `derived > codebase > library > registry > option`.
- `get_pattern_for(name)` — the specific pattern + all its
  instances. Used by the assistant to see *how the pattern is
  used here today*.
- `match_pattern(description, sources?)` — given a task
  description, returns ranked matches across all sources with
  tradeoffs. Central call for locate-step.
- `get_pattern_options(category)` — for architectural choices,
  returns options with tradeoffs.
- `get_project_conventions()` — returns derived patterns +
  chosen options for this project. This is what makes the
  assistant "know how you work here".

**Contract:** any time the assistant is about to introduce
something new (adapter, endpoint, page, test), it calls
`match_pattern(task)` first. If a codebase / library / derived
pattern applies, follow it. If only a registry pattern or
architectural option applies, present with tradeoffs.

## Anti-patterns

Same detection framework, three primary anti-pattern recognisers:

- **Duplication** — reuses `get_duplicates` MCP tool
  ((memory: project_p2_sweep_2026_07)). Similarity > threshold →
  anti-pattern instance.
- **Spaghetti coupling** — coupling metric > threshold.
- **Broken layering** — imports crossing layers the wrong way.
- **God object** — struct with method+field count above
  threshold.
- **Dead code** — reachability analysis. See
  (memory: project_p2_sweep_2026_07) `#24 dead-code drop`.

## Optimization opportunities

- **N+1 query** — loop over records with per-iteration query.
- **Sync-on-hot-path** — sync IO in a request handler with an
  async sibling.
- **Missing index** — join column with no index, > N rows.
- **Missing cache** — repeated call with same arguments and pure
  result.

## Signals produced

| Signal | Consumer |
|---|---|
| `get_patterns(project)` full catalog | assistant context |
| `get_project_conventions()` derived + chosen | assistant at session start |
| Anti-pattern detections | Insights Now column violation cards |
| Optimization opportunities | Insights Soon column |
| Pattern conformance verify at commit / tool call | [[pipeline/governance]] verifier gate |
| Promoted patterns → rules | [[pipeline/governance]] |
| `sensei.pattern_choices` writes | derived-rule with `source: chosen:option:{cat}` |

## Done gate

- Every project with > 1000 nodes has codebase design patterns
  detected where they exist (adapter / plugin / observer /
  subscriber are the ones most likely to show up in this repo).
- Libraries with `llms.txt`-style pattern sections have their
  library patterns ingested when `add_library` runs.
- At least one industry registry (patterns.dev) is ingestible;
  the resulting patterns appear via `match_pattern`.
- Architectural option choices persist and propagate as rules;
  the assistant reads them via `get_project_conventions()` on
  subsequent sessions.
- Derived patterns surface after N consistent usages.
- `get_patterns(project=sensei)` returns the assistant-adapter
  trait pattern with all its instances.
- The verifier ([[pipeline/governance]]) blocks a commit that
  introduces a new file matching an anti-pattern signature.
- Promoted patterns cross into `sensei.rules` at the right
  ladder / priority.

## Wrong gate

- **Assistant builds a new adapter without calling
  `get_patterns` first.** Contract not enforced; the anti-
  spaghetti guarantee fails.
- **Rokkit's data-driven-component pattern isn't ingested from
  `~/Developer/rokkit`.** Library pattern extraction regressed
  (see (memory: project_mcp_libdocs_rokkit)).
- **Same pattern detected in two sources isn't deduped when the
  assistant reads `get_patterns`.** Assistant sees two entries,
  gets confused.
- **Architectural option chosen once but the assistant asks
  again next session.** `pattern_choices` not read at
  session start.
- **Derived pattern promoted to rule but not visible in
  `get_project_conventions()`.** Read-path bug.
- **Detection re-runs on every tick even when nothing
  changed.** Incremental gate missing.
- **Anti-pattern (duplication) blocking a legitimate refactor
  that's temporarily duplicating before consolidating.**
  Verifier needs to accept a "pending consolidation" override
  with expiry.
- **Registry pattern applied blindly without checking whether
  the codebase has its own convention.** Ordering wrong;
  derived and codebase should win over registry.

## Related

- [[pipeline/libraries]] — library pattern extraction
- [[pipeline/analyzer]] — schedules pattern detection
- [[pipeline/memory]] — memory ↔ pattern relationship
- [[pipeline/insights]] — pattern conformance recommendation
- [[pipeline/governance]] — promoted patterns become rules;
  verifier enforces conformance
- [[pipeline/capture]] — new features must adhere to detected
  patterns (adapter architecture is the canonical example)
- [[pipeline/mcp-surface]] — `get_patterns` / `get_pattern_for`
  / `match_pattern` / `get_pattern_options` /
  `get_project_conventions` tools
- [[screen/project-patterns]] — human review + promote surface
- (memory: project_ingest_scan_bug_batch) (memory) — community detection
- (memory: project_mcp_libdocs_rokkit) (memory) — library ingestion
