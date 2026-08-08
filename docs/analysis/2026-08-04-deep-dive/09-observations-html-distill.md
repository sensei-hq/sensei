## Anti-Patterns Screen: 406 Rows, Zero Legibility — Distilling the Raw HTML Dump
_The Patterns screen renders 406 near-identical rows with no description, no computed FTR delta, no action, and a "1×" badge that hides up to 301 real edits. This section reverse-engineers the 480 KB HTML dump on line 14 of the Observations doc and reconciles every number against the daemon's `inference.detected_patterns` table._

**User's observation.** From `docs/analysis/2026-08-04-Observations.md`, under **## Patterns**: _"Can't make out what the pattern is and what to do with it. Should be user friendly."_ Line 14 of the same file is a 480,184-byte raw HTML paste of the app's Anti-patterns list — evidence, not prose. This section treats that markup as a forensic artifact.

**What the data shows.**

The dump is a snapshot of the app's Anti-patterns list. Parsing its 406 `pattern-row` elements yields a strikingly uniform, low-signal surface — and the DB confirms the emptiness is real, not a render bug.

- **The header reads `Anti-patterns (406)`, and there are exactly 406 rows.** Every structural marker agrees: 406 `pattern-row-<uuid>` test-ids, 406 name cells, 406 instance badges, 406 FTR-delta cells.[^h1] All 406 UUIDs resolve in the DB, and **all 406 belong to a single project — `sensei`**.[^db1] So the app's "406" is *not* the DB-wide anti-pattern count; it is one project's list.

- **The `406` vs the known DB total of `932` is a project filter, fully reconciled.** The DB holds 932 anti-patterns across 128 projects; the `sensei` project alone has **411** (410 `rework:*` + 1 `correction-prone`).[^db2] The dump shows 406 — five fewer. Those five were all detected *after* the paste: the newest row in the dump is `2026-08-03 13:58`, while the five extras carry `detected_at` of `2026-08-04 19:17`–`22:31`.[^db3] The dump is a point-in-time capture of `sensei`'s anti-pattern list; the delta is pure recency, not a hidden filter.

- **405 of the 406 rows (99.75%) are named `rework: <absolute file path>`; exactly one is `correction-prone`.** The kanji glyph is `禁` (warning) on all 406 — no visual differentiation whatsoever.[^h2] The DB agrees: across all 932 anti-patterns, 922 are `rework:*` and 10 are `correction-prone`.[^db4] The "Anti-patterns" screen is, in practice, a flat list of reworked file paths.

- **Every row shows the same three tokens: `1×` (or `9×` once), `suggested`, `±0%`.** All 406 states are literally `suggested`; there are zero `enforced`/`rejected`/`deprecated`.[^h3] The DB confirms `lifecycle='suggested'` for 100% of the 932 anti-patterns[^db5] — the lifecycle machinery exists but nothing ever advances a pattern out of `suggested`.

- **The FTR delta is `±0%` on all 406 rows because it is never computed.** Of 1,478 rows in `inference.recommendations`, **0 have a `baseline_ftr`**, exactly **1** has a `current_ftr` (=1.000), and **0** have a non-zero delta.[^db6] `detected_patterns` has no FTR column at all. The "FTR delta vs project baseline" tooltip promises a comparison the pipeline never produces.

- **The `1×` badge is `jsonb_array_length(instances)`, not an edit count — and it hides the real magnitude.** `instance_count = jsonb_array_length(instances)` for 100% of anti-patterns (0 mismatches).[^db7] For `rework:*` each instance is one file, so the badge is almost always `1`. But every instance carries `total_edits` / `max_session_edits` in its jsonb. Aggregating the `sensei` rework instances: **410 files, 5,986 total edits, avg 14.6 edits/file, max 301.**[^db8] The single worst file, `crates/senseid/src/db/pg_store.rs`, shows **301 edits (98 in one session)** — rendered by the UI as an indistinguishable `1× · suggested · ±0%`.

  What the screen *should* rank by — top rework hotspots by real edit magnitude:

  | # | File | total_edits | max in 1 session | severity | enforcement | has_desc |
  |---|------|------------:|-----------------:|:--------:|:-----------:|:--------:|
  | 1 | `crates/senseid/src/db/pg_store.rs` | 301 | 98 | ∅ | ∅ | ✗ |
  | 2 | `docs/llm-spec/park/_run-state.md` | 210 | 197 | ∅ | ∅ | ✗ |
  | 3 | `.claude/…/memory/MEMORY.md` | 110 | 26 | ∅ | ∅ | ✗ |
  | 4 | `crates/mcp/src/lib.rs` | 90 | 22 | ∅ | ∅ | ✗ |
  | 5 | `crates/senseid/src/api/routes.rs` | 80 | 30 | ∅ | ∅ | ✗ |
  | 6 | `crates/senseid/src/api/gateway_init.rs` | 58 | 44 | ∅ | ∅ | ✗ |
  | 7 | `crates/senseid/src/dojo/client.rs` | 51 | 26 | ∅ | ∅ | ✗ |
  | 8 | `dojo/…/relay/[run_id]/+page.svelte` | 49 | 38 | ∅ | ∅ | ✗ |
  | 9 | `marketplace/catalog.json` | 48 | 48 | ∅ | ∅ | ✗ |
  | 10 | `crates/senseid/src/api/handlers/observatory.rs` | 46 | 26 | ∅ | ∅ | ✗ |

  <sub>Full top-25 queried; ranks 11–25 span 34–45 edits across `knowledge.rs`, `tasks/mod.rs`, `cli/main.rs`, `api.ts`, `appstate.svelte.ts`, etc.[^db8]</sub>

- **Zero rows carry a description, severity, enforcement, family, example, or fix — in the DB, not just the render.** Across all 932 anti-patterns: `has_description=0`, `has_severity=0`, `has_enforcement=0`, `has_family=0`, `has_example=0`, `has_fix_pattern_id=0`.[^db9] The dump rendered the detail panel for only the **2 expanded rows** (`aria-expanded="true"`), and both read _"No description captured for this pattern yet."_[^h4] That placeholder is not an exception — it is the guaranteed content of **all 932** rows if expanded. The `description`-completeness rate is **0.0%**.

- **The evidence *exists* — it is just stored in the wrong column and never surfaced.** `evidence` jsonb is empty (`[]`) for all 932, but `instances` jsonb is populated for all 932.[^db10] The `correction-prone` row's 9 instances each contain the actual user correction prompt — e.g. _"When you belong to a Dōjō you administer… this is incorrect. Administration depends on role…"_[^db11] That is a ready-made, human-readable description. The UI shows "No description captured" while sitting on the exact text a description would contain.

- **Each row has one button — the expand toggle — and no action affordance at all.** The markup contains exactly 406 `<button>` elements: 406 `pattern-toggle-*`, and zero apply/enforce/dismiss/reject/fix controls.[^h5] Meanwhile a recommendation layer already exists that *could* be the action: **572 of 1,478 recommendation titles reference "rework"**, and `action_type` includes `promote_pattern` (16), `revise_rule` (10), `write_skill` (13).[^db12] The Patterns screen surfaces none of it — the two data structures are disconnected in the UI. The `rows-with-action` rate is **0%**.

- **A large fraction of the "anti-patterns" are docs/memory churn, not code rework.** Of the 410 `sensei` rework files: 122 are `.md`, 105 `.rs`, 77 `.svelte`, 72 `.ts`; **31 live under `.claude/…/memory/`**.[^db13] Iterating on `MEMORY.md` (110 edits) or `_run-state.md` (210 edits) is expected doc evolution, yet it is flagged with the identical `禁` warning glyph as a genuinely churn-prone source file. There is no taxonomy separating "doc/spec iteration" from "code rework."

**Root cause / interpretation.**

The Patterns screen is a thin `<ul>` over `inference.detected_patterns` filtered to the selected project, and it renders whatever columns the daemon writes. The daemon's rework detector writes exactly one useful thing — an `instances` jsonb array of `{file, total_edits, max_session_edits}` — and leaves `description`, `severity`, `enforcement`, `family`, `example`, and `fix_pattern_id` null on 100% of rows. The UI faithfully renders those nulls: a placeholder for the missing description, a `severity`-less warning kanji, and a badge bound to `instance_count` (array length) rather than the `total_edits` field that actually measures rework. The most important number in the record (301 edits on `pg_store.rs`) is one JSON hop away from the badge and never makes it to the screen.

The `±0%` FTR delta is a promise with no backing computation. `detected_patterns` has no FTR field, and `recommendations` — the only table with `baseline_ftr`/`current_ftr` — has them null on 1,477 of 1,478 rows. The tooltip "FTR delta vs project baseline" is UI scaffolding for a metric the inference pipeline never fills, so it degrades to a constant `±0%`. This is worse than omission: it signals "measured, no effect" when the truth is "never measured."

The uniform `suggested` lifecycle reveals a dead promotion loop. 932 anti-patterns, 943 detected patterns, 1,478 recommendations, 1,947 drift items — yet only 11 durable `sensei.memories`. Nothing walks `detected_patterns` from `suggested` → `enforced`/`rejected`; the lifecycle enum and the `promote_pattern` recommendation type both exist but are not wired to a UI action, so the list can only grow. Because rework is keyed per-file-path with `instance_count=1`, the same underlying behavior ("this file gets reworked a lot") fragments into 410 single-instance rows instead of aggregating into a handful of ranked hotspots — the exact "can't make out what the pattern is" the user reported.

Finally, the detector's scope is too broad. Counting edits to `.claude/memory/*.md` and `docs/llm-spec/*.md` as anti-patterns (153 of 410 rework files are `.md`, 31 under `/memory/`) floods the list with expected doc iteration and dilutes the genuine code hotspots. With no severity, no `total_edits` ranking, and no doc/code split, a real 301-edit hotspot and a routine `MEMORY.md` note are visually identical.

**Recommendations.**

1. **(P0) Rank and label by `total_edits`, not `instance_count`.** In the Patterns loader (daemon `observatory`/`knowledge` handler feeding the Anti-patterns list) sort `rework:*` by `SUM((instances→>'total_edits')::int)` desc and render that number in the badge (`301×`, not `1×`). Add `max_session_edits` as a secondary "peak burst" chip. *Effect:* the screen immediately becomes a ranked hotspot list; `pg_store.rs` surfaces at the top instead of alphabetically buried.

2. **(P0) Derive a description at write time from what's already in `instances`.** When the rework detector writes a row, populate `description` — e.g. _"Reworked 301× across N sessions (peak 98 edits in one session)."_ For `correction-prone`, fold the instance `prompt` text into `description`. Backfill the existing 932 rows with a one-off pass over `instances`. *Effect:* description-completeness goes 0% → ~100% with zero new data collection; the "No description captured" placeholder disappears.

3. **(P0) Remove or truthfully populate the FTR-delta cell.** Either wire `recommendations.baseline_ftr`/`current_ftr` into the detector's per-pattern context and show a real delta, or drop the `pattern-ftr-delta-*` cell until it can be computed. Shipping a constant `±0%` under a "vs project baseline" tooltip is a fabricated-signal violation of the honest-empty rule. *Effect:* removes a misleading always-zero column.

4. **(P1) Add per-row actions bound to the existing recommendation layer.** Give each row `Enforce` (advance `lifecycle` → `enforced`), `Dismiss` (→ `rejected`), and `Create rule`/`Handoff` that opens the matching `rework`-titled recommendation (572 already exist) or routes to intake. *Effect:* `rows-with-action` 0% → 100%; closes the dead `suggested` promotion loop the user called out ("can't take any action").

5. **(P1) Split doc/spec iteration from code rework, and add severity.** Tag rework instances by path class (code vs `.md`/`/memory/` doc) and set `severity` from `total_edits` thresholds (e.g. ≥50 high, ≥20 medium). Default the Anti-patterns view to code + high/medium. *Effect:* removes ~153 `.md` rows of noise; the `禁` glyph finally means something.

6. **(P2) Aggregate rework across files into module/folder hotspots.** Roll 410 per-file rows into `folder_id`-scoped groups (`crates/senseid/src/api/*` reworked 300+× total) so the list has the hierarchy the Atlas/graph observation also asks for. *Effect:* collapses a 406-row wall into a browsable, dedup'd hotspot tree.

**Proposed metrics & instrumentation.**

| Metric | Definition / formula | Source (table.column) | Cadence | Current gap |
|---|---|---|---|---|
| Description-completeness % | `count(description<>'') / count(*)` over anti-patterns | `inference.detected_patterns.description` | project + daily | **0.0%** (0 / 932) — column never written |
| FTR-delta-computed % | `count(baseline_ftr IS NOT NULL AND current_ftr IS NOT NULL) / count(*)` | `inference.recommendations.baseline_ftr`, `.current_ftr` | project + daily | **~0%** (0 baseline, 1 current of 1,478); UI shows constant `±0%` |
| Rows-with-action % | rows exposing an enforce/dismiss/fix control ÷ rows rendered | UI + `detected_patterns.lifecycle`, `.fix_pattern_id` | screen render | **0%** (406 buttons = 406 expand toggles; 0 fix links) |
| Rework magnitude (true) | `SUM((instances→>'total_edits')::int)` per pattern | `inference.detected_patterns.instances` jsonb | session → project | Hidden: badge shows array length (`1×`), not edits (max 301) |
| Lifecycle-progression % | `count(lifecycle<>'suggested') / count(*)` | `inference.detected_patterns.lifecycle` | project + weekly | **0%** (932 / 932 stuck `suggested`) |
| Severity-coverage % | `count(severity IS NOT NULL) / count(*)` | `inference.detected_patterns.severity` | project + daily | **0%** (0 / 932) |
| Doc-vs-code rework ratio | code rework files ÷ total rework files | `detected_patterns.instances→>'file'` path class | project + daily | Uncomputed; ~37% of `sensei` rework (153/410) is `.md`/memory noise |

---

### SQL appendix

[^h1]: `sed -n '14p' 2026-08-04-Observations.md > /tmp/antipatterns.html` then Python `re.findall` over the blob: `data-testid="pattern-row-([0-9a-f-]+)"` → 406; header via `grep -oE 'Anti-patterns[^<]*'` → `Anti-patterns (406)`.
[^h2]: `re.findall(r'text-\[13px\] text-ink truncate">([^<]+)</p>')` → 406 names; prefix-count: 405 `rework:*`, 1 `correction-prone`; `class="kanji…">([^<]+)` → `{'禁'}` only.
[^h3]: `re.findall(r'font-mono uppercase tracking-wide">([^<]+)')` → all `suggested`; ftr cells `pattern-ftr-delta-…>([^<]+)` → all `±0%`; instance badges `<span class="font-mono">(\d+)×` → Counter `{'1':405,'9':1}`.
[^h4]: `aria-expanded="true"` → 2; `pattern-detail-*` divs → 2; `grep -oF 'No description captured'` → 2 (only expanded rows render the detail panel).
[^h5]: `grep -oE '<button'` → 406 total; `pattern-toggle-*` → 406; grep for `apply|enforce|dismiss|reject` per-row → 0.

[^db1]: `\copy` the 406 row-UUIDs into a temp table; `SELECT count(*) FROM html_rows h JOIN inference.detected_patterns d ON d.id=h.id` → 406; grouped by project → `sensei` 406.
[^db2]: `SELECT CASE WHEN name LIKE 'rework:%' THEN 'rework:*' ELSE name END, count(*) FROM inference.detected_patterns d JOIN sensei.projects p ON p.id=d.project_id WHERE d.is_anti_pattern AND p.name='sensei' GROUP BY 1` → `rework:*` 410, `correction-prone` 1.
[^db3]: `SELECT name, detected_at FROM inference.detected_patterns d JOIN sensei.projects p ON p.id=d.project_id WHERE d.is_anti_pattern AND p.name='sensei' AND d.id NOT IN (SELECT id FROM html_rows) ORDER BY detected_at DESC` → 5 rows, `2026-08-04 19:17`–`22:31`; dump max `detected_at` = `2026-08-03 13:58`.
[^db4]: `SELECT CASE WHEN name LIKE 'rework:%' THEN 'rework:*' ELSE split_part(name,':',1) END, count(*) FROM inference.detected_patterns WHERE is_anti_pattern GROUP BY 1` → `rework:*` 922, `correction-prone` 10.
[^db5]: `SELECT lifecycle, count(*) FROM inference.detected_patterns WHERE is_anti_pattern GROUP BY 1` → `suggested` 932 (only row).
[^db6]: `SELECT count(*), count(*) FILTER (WHERE baseline_ftr IS NOT NULL), count(*) FILTER (WHERE current_ftr IS NOT NULL), count(*) FILTER (WHERE baseline_ftr<>current_ftr) FROM inference.recommendations` → 1478 / 0 / 1 / 0.
[^db7]: `SELECT count(*) FROM inference.detected_patterns WHERE is_anti_pattern AND instance_count <> jsonb_array_length(instances)` → 0.
[^db8]: `WITH ri AS (SELECT (i->>'total_edits')::int te FROM inference.detected_patterns d JOIN sensei.projects p ON p.id=d.project_id CROSS JOIN LATERAL jsonb_array_elements(d.instances) i WHERE d.is_anti_pattern AND d.name LIKE 'rework:%' AND p.name='sensei') SELECT count(*), sum(te), round(avg(te),1), max(te) FROM ri` → 410 / 5986 / 14.6 / 301. Top-25 = same CTE, `ORDER BY te DESC LIMIT 25`.
[^db9]: `SELECT count(*) FILTER (WHERE description IS NOT NULL AND btrim(description)<>''), count(*) FILTER (WHERE severity IS NOT NULL), count(*) FILTER (WHERE enforcement IS NOT NULL AND btrim(enforcement)<>''), count(*) FILTER (WHERE family IS NOT NULL), count(*) FILTER (WHERE example IS NOT NULL AND btrim(example)<>''), count(*) FILTER (WHERE fix_pattern_id IS NOT NULL) FROM inference.detected_patterns WHERE is_anti_pattern` → all 0 of 932.
[^db10]: `SELECT count(*) FILTER (WHERE evidence<>'[]'::jsonb), count(*) FILTER (WHERE instances<>'[]'::jsonb) FROM inference.detected_patterns WHERE is_anti_pattern` → 0 / 932.
[^db11]: `SELECT jsonb_pretty(instances) FROM inference.detected_patterns WHERE name='correction-prone' AND project_id=(SELECT id FROM sensei.projects WHERE name='sensei')` → 9 instances, each `{prompt, session, folder_id}` with real correction text.
[^db12]: `SELECT action_type, count(*) FROM inference.recommendations GROUP BY 1` → `library_update` 806, `audit_stale` 566, `create_agent` 43, `enrich_memory` 24, `promote_pattern` 16, `write_skill` 13, `revise_rule` 10. `count(*) FILTER (WHERE title ILIKE '%rework%')` → 572.
[^db13]: `WITH ri AS (SELECT (i->>'file') file FROM inference.detected_patterns d JOIN sensei.projects p ON p.id=d.project_id CROSS JOIN LATERAL jsonb_array_elements(d.instances) i WHERE d.is_anti_pattern AND d.name LIKE 'rework:%' AND p.name='sensei') SELECT count(*) FILTER (WHERE file LIKE '%/memory/%'), count(*) FILTER (WHERE file ~ '\.(md|mdx)$'), count(*) FROM ri` → memory 31, md 122, total 410; ext breakdown: md 122, rs 105, svelte 77, ts 72.
