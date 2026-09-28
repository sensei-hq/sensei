# `sensei` — design notes

Why the tables in `database/ddl/table/sensei/` are shaped the way they are.

What a column MEANS lives in `comment on column`, where the database catalog can serve it. This file holds the other half: what was measured, what was tried and failed, and what must not be changed back. It is prose about decisions, which is why it is not in the DDL.


## `sensei.edges`

[`database/ddl/table/sensei/edges.ddl`](../../database/ddl/table/sensei/edges.ddl)


### `resolved_via`


THE EDGE-LEVEL VERDICT, as columns rather than as JSON.

The per-use verdicts stay in `props.occurrences.*[*].{rung,reason}` — an edge aggregates many use sites and each has its own span and its own outcome, so that list is variable-length and belongs in jsonb. These two are the REDUCTION over it (see `sensei.edge_verdict`), and a reduction is scalar and universal, which is what earns a column.

Why they are not read out of props instead: they were, and it did not work. Both were `props->>'rung'` / `props->>'reason'` at the TOP level, where no writer has ever put them, so `resolved_via` was NULL on all 1,640,215 placed edges and `unresolved_reason` NULL on all 2,428,016 missed ones — no match and no miss classifiable, in any language. Measured on 2026-09-26.

Exactly one of the two is set on any edge: a placed edge has a rung, a missed one has a reason. Both NULL means the writer recorded no verdict at all, which is the pre-v2 shape and is itself worth counting.

NO FOREIGN KEY to reason_codes, deliberately. The views join it LEFT so that a code with no seeded prose still surfaces raw rather than dropping the edge that carries it; an FK would turn that tolerance into a write failure, and a new Reason variant landing in Rust before its row lands in the seed would then fail the whole index instead of showing up as an unlabelled code.


## `sensei.files`

[`database/ddl/table/sensei/files.ddl`](../../database/ddl/table/sensei/files.ddl)


### `skip_detail`


The parser's VERBATIM message with line and column (R10.9, A9). `skip_reason` carries the CODE; this carries the text. "parse_error" tells an agent a file is broken; "x.rs:142: expected `}`" tells it what to do. It lives here rather than on a node's props because splitting the code and the detail across two tables recreates the two-copies-of-one-fact problem R10.9 exists to avoid.


### `parsed_at`


The file LIFECYCLE's missing bit (03 S4). The design names four states — discovered / parsed / unparseable / skipped — and `skip_reason` already separates the last two from the rest. What it cannot separate is `discovered` from `parsed`: both are skip_reason NULL, so a parse task that never ran looks exactly like one that succeeded and found nothing (a real state, R10.3). This column is that one bit and nothing more. parsed_at NULL, skip_reason NULL -> discovered  (stalled, if it lingers) parsed_at set,  skip_reason NULL -> parsed parsed_at set,  skip_reason set  -> unparseable | skipped, per the reason A four-value enum column was the other option and was rejected: it would restate what skip_reason already says, giving two writes of one fact that can disagree. See indexer.md R13.


## `sensei.libraries`

[`database/ddl/table/sensei/libraries.ddl`](../../database/ddl/table/sensei/libraries.ddl)


### `repository_url`


Where the library's SOURCE lives, from the registry's own record of it (02b S8). The input the github docs route needs, and what R11.2's upstreaming has to file against — blocked until now because only 2 of 1,121 rows carried any URL at all.

On `libraries`, not `library_versions`: a library's repository is part of its identity and does not change per release. Where a given release's DOCS were fetched from is a different question and lives on the version (`base_url` / `docs_url`).

NULL means the registry did not state one. NEVER derived from the package name — `github.com/<name>/<name>` is wrong far more often than right, and a fabricated URL is worse than none because something will fetch it.


## `sensei.library_content`

[`database/ddl/table/sensei/library_content.ddl`](../../database/ddl/table/sensei/library_content.ddl)


### `source`


PROVENANCE — where the row came from. 'manifest' | 'generated' for skills and agents; the fetch origin for pages. Distinct from `source_type`, which is the ROUTE. Conflating the two is the trap 02b S11 names: both are plausibly "source" and nothing type-checks the difference.


### `package_name`


WHICH PACKAGE this documents, when it documents one.

Skills and agents are library-level: rokkit's "styling" skill is about rokkit. DOCS are not. `rokkit` publishes `@rokkit/ui`, `@rokkit/core`, `@rokkit/actions` …, and the `List` page documents `@rokkit/ui` specifically. Without this column a reference to `@rokkit/ui` resolves to the library and then to ALL of its pages, and picking the right one falls back to matching `component` against a symbol name — a guess (R4).

NULL means library-level, which is a real and common state: an overview, a getting-started guide, an architecture page. NULL is "applies to the whole library", never "we don't know".

TEXT, and deliberately NOT a foreign key to `library_packages`. A page can name a package that has not been grouped yet, and an FK would either reject the page or force a phantom `library_packages` row — the get-or-create failure R13 forbids one table over. Join opportunistically; a miss is an honest "we hold no grouping for this package".

INVARIANT, not enforceable by a single FK: the named package must belong to the SAME library as this row's version. `library_packages.library_id` and `library_versions.library_id` must agree. A page claiming a package of a different library is a manifest conflict and gets REPORTED (02b §4), never silently resolved.


## `sensei.memories`

[`database/ddl/table/sensei/memories.ddl`](../../database/ddl/table/sensei/memories.ddl)


### `spine_slot`


Spine-slot anchor (design 2026-07-18-memory-anchoring): which doc slot this memory belongs to, for slot-scoped retrieval. `feature` disambiguates scope (null = project-scope; set = docs/features/<feature>/). Both nullable = unanchored.


## `sensei.metric_deactivations`

[`database/ddl/table/sensei/metric_deactivations.ddl`](../../database/ddl/table/sensei/metric_deactivations.ddl)


### `metric_key`


The catalogue KEY, not a metric id. `sensei.metrics.id` differs between the two planes — separate databases loaded from the same staging file — so the key is the only value that survives the trip. Deliberately NOT a foreign key for the same reason a dōjō may name a metric this install has not seeded yet; an unknown key simply matches nothing.


## `sensei.metrics`

[`database/ddl/table/sensei/metrics.ddl`](../../database/ddl/table/sensei/metrics.ddl)


### `derives_from`


Metric keys whose relationship to THIS metric is definitional or mechanical rather than informative — the suppression list for correlation analysis. Measured on real data, an unfiltered ranking is topped by arithmetic: tokens_in_per_day vs tokens_per_day correlates 1.00 because the second CONTAINS the first, and session_duration vs the token counts sits at 0.89-0.92 because a longer session mechanically consumes more. Presenting those as insights would bury the genuine findings (spec_depth vs spec_deviation_rate at -0.54, throughput vs shallow-analysis at 0.77). Symmetric by convention: list the relationship on either side and the engine treats it both ways.


## `sensei.nodes`

[`database/ddl/table/sensei/nodes.ddl`](../../database/ddl/table/sensei/nodes.ddl)


### `file_id`


R13. The file this node is DECLARED in, as a key rather than a repeated path. ON DELETE RESTRICT, never CASCADE: removing a file row must not silently delete every declaration in it — a file's removal goes through reconcile, one declaration at a time, with inbound edges unresolved first (R10.8). Nullable, because PARTIAL (referenced, not yet declared) and EXTERNAL (`lib·`) nodes have no file by definition.

This foreign key is what makes the ORPHANED state UNREPRESENTABLE. 8,147 nodes previously named a file the scanner did not track and read as COMPLETE to every consumer; they were swept when this column landed, and the constraint is why they cannot come back.


## `sensei.personas`

[`database/ddl/table/sensei/personas.ddl`](../../database/ddl/table/sensei/personas.ddl)


### `github_login`


VERIFIED identity, set only by a completed OAuth sign-in. NULL means "we have not proven who this is", which is a different and more useful state than a plausible-looking label.


### `forge_token_state`


What we currently believe about this persona's FORGE token — GitHub's, not the dōjō session's. Two different credentials with two different lifetimes: the dōjō session refreshes on every use, while the GitHub token expires on a measured ~8-hour cycle. Nothing recorded the difference, so `GET /api/auth/status` reported `signedIn: true` for a whole morning while every forge call answered 401.

`unknown` is the DEFAULT and a real state, not a placeholder: a persona created before anything asked GitHub genuinely has no standing, and defaulting to `active` would claim a credential we have never tested.


### `forge_token_checked_at`


When we last learned anything about it. Distinguishes a standing that is current from one recorded days ago, and is deliberately NOT stamped when a probe could not reach the forge: claiming a check that told us nothing would make a stale belief look fresh.


## `sensei.reason_codes`

[`database/ddl/table/sensei/reason_codes.ddl`](../../database/ddl/table/sensei/reason_codes.ddl)


`normal` means it clears itself, so it addresses nobody. A remedy on a `normal` row is the contradiction that made `forge_visibility_unknown` look benign while it was permanently stuck.


## `sensei.schedules`

[`database/ddl/table/sensei/schedules.ddl`](../../database/ddl/table/sensei/schedules.ddl)


A CHECK, not a runtime fallback: a zero interval busy-loops a core, and the database is the right place to make that unrepresentable.


ISO weekdays: 1 = Monday … 7 = Sunday. Rejects anything else rather than silently never matching.
