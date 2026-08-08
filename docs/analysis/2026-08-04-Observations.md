# Product Surface & UX — Deep Analysis

_2026-08-04 · grounded in the live daemon DB and the app source_

This doc turns the raw field notes below into data-grounded findings about sensei's
**product surface** — the Atlas graph, the Patterns/Insights screens, the tool/content
registry, Libraries, and component consistency. It is the UX companion to
[metrics.md](2026-08-04-metrics.md); the reproducible evidence (with SQL and file:line
references) is in [`2026-08-04-deep-dive/`](2026-08-04-deep-dive/).

The earlier version of this file also contained a 480 KB raw DOM capture of the
Anti-patterns list, pasted as evidence. That capture has been distilled — with the exact
row/DB reconciliation — into deep-dive
[09](2026-08-04-deep-dive/09-observations-html-distill.md) and is summarized in §2.

---

## Raw field notes (preserved verbatim)

> - code graph for sensei project. it just looks like scatterd circles of different sizes.
>   I don't see the hierarchy. App v/s backend v/s crate/library etc — we had a better graph
>   structure with separation of docs, code, packages, modules etc. or hierarchical structure
>   packages/class/methods etc. however the graph does not seem to be represented well or maybe
>   the graph is not correct. We also had separation by sub projects or subtrees — that graph
>   structure is not visible in atlas.
> - We have a switch component in rokkit — why the odd on|off which does not have contrast?
> - We have a toggle component in rokkit used in some places but custom-rolled items in others.
>   No consistency in design.
> - Traceability repeats the same observation multiple times. Can't see detail and can't take
>   any action.
> - Libraries shows version conflict. Would be nice if we can generate a prompt as a handoff
>   action, or send to the intake page from here.
> - Libraries search followed by custom-rolled toggle instead of rokkit toggle for line items
>   (version conflict is barely visible); same for the [project] pill.
> - Patterns: Can't make out what the pattern is and what to do with it. Should be user friendly.
> - (follow-up) The graph used to work — somewhere during various fixes the structure broke.
>   I remember confirming I could see project structure: docs vs code, packages, modules, etc.

---

## 1. The Atlas code-graph regression

**This is the headline, and it is a real, datable bug — not a perception problem.** The
memory of "it used to work" is correct. The graph collapsed on three fronts, all landing in
the same week (~2026-07-12/13). Full evidence + file:line fixes in deep-dive
[10](2026-08-04-deep-dive/10-graph-indexing-regression.md).

**Front 1 — the `covers` edge duplication bomb (the big one).**
`insert_edge` has **no `ON CONFLICT`**, and both `build_connections` and
`reconcile_connections` re-emit the full doc×file cartesian product on every run. Result:
`covers` edges are duplicated **917.6×** — 1,714,911 rows collapse to **1,869 distinct
pairs**. `covers` is now **86% of all edges** and outnumbers structural edges
(imports/calls/extends/references) **6.5 : 1**. A single monorepo (`rokkit`) accounts for
93% of the duplication (2,528×). The explosion is datable: `covers` for the sensei cohort
jumped from 3,851 (week of 06-15) to **437,279 (week of 07-13)**. A force-directed layout
fed 1.7 M near-identical containment edges cannot help but render as a hairball of
same-colored circles.

**Front 2 — community clustering covers 1.1% of the graph.**
Only **5,443 of 476,988 nodes** carry a `community_id`. The 06-15 bulk index (94.6% of the
graph) has none. Worse, the clustering the Atlas view *would* color by is stale and
inflated: `inference.communities` claims 43,290 member nodes but only 5,443 point back
(7.9× inflation), because label-propagation emits non-deterministic labels and old rows
survive the re-index FK cascade. The daemon's own repo (`senseid`) has 95 community rows and
**zero** live members — it renders as ghost bubbles sized by a phantom `node_count`.
(Community adjacency also depends on the `implements` edge kind, which has **0 rows** in the
entire DB — a dead dependency.)

**Front 3 — the grouping levels you remember were never populated.**
The `node_kind` enum defines `package` and `section`; the `folder_kind` enum defines
`subtree` and `workspace_member`. **All produce zero rows.** So nothing nests above
`file`/`module`, and there is no sub-project boundary — exactly the "docs vs code /
packages / subtrees" separation you recall. The hierarchy that *does* exist is unused:
`node.parent_id` is populated on 91.5% of nodes (method→class→file), and `sensei.folders`
encodes the project/subtree tree (`root_id`, `parent_id`, `project_id`, `role`) — but the
viz ignores both. A source comment in `atlas-graph.svelte.ts:11-12` openly admits it colors
by **kind** because "the communities endpoint carries no per-node membership."

**Why it looks like scattered circles:** Atlas auto-collapses any repo >500 nodes to a flat
community-bubble overview, colored by kind, sized by the inflated/stale `node_count`, over a
layout swamped by 900×-duplicated `covers` edges. Every one of the three fronts pushes toward
the same degenerate picture.

**Recommendations (all P0 unless noted).**
1. Make `covers` idempotent — unique index on `edges(folder_id, source_id, target_id, kind)`
   + `ON CONFLICT DO NOTHING` in `insert_edge` (`pg_store.rs:1711`); dedup the 1.7 M rows to
   ~1,869. Removes 86% of edge volume in one change.
2. Exclude `covers`/`references` from the force layout — position on `calls`/`imports`/
   `extends` only; keep containment as an on-demand overlay.
3. **Render the hierarchy that already exists** — group by `folder_id` + folder `parent_id`
   tree and node `parent_id`, so docs-vs-code / module / class / method nesting returns
   *without needing communities at all*.
4. **(P1)** Fix community durability (rebuild membership atomically, delete stale rows before
   re-upsert, preserve `community_id` across re-index) and backfill the missing grouping
   levels (`package`/namespace nodes; classify folders as `subtree`/`workspace_member` — the
   scanner already finds sub-projects).
5. **(P2)** Re-index the 06-15 bulk cohort so coverage reflects current code.

## 2. Patterns / Insights / Recommendations — unreadable and unactionable

> "Can't make out what the pattern is and what to do with it." / "Traceability repeats the
> same observation." Evidence: deep-dive
> [07](2026-08-04-deep-dive/07-insight-pattern-recommendation-ux.md) +
> [09](2026-08-04-deep-dive/09-observations-html-distill.md).

- **0 of 943 patterns have a description** (also `family`, `severity`, `enforcement`,
  `example`, `fix_pattern_id` are NULL for all). The "No description captured" the UI shows
  is an app-side fallback for a column that is empty everywhere. The raw material for a good
  description already exists one column over — `instances` jsonb carries `total_edits`,
  `max_session_edits`, and the offending prompt text — the write path just skips it.
- The Patterns screen is really a **flat per-file churn log** wearing a "patterns" label:
  922 of 943 are `rework:<path>` markers; the app's "Anti-patterns (406)" is the sensei
  project filtered. The `1×` badge is `jsonb_array_length`, not magnitude — so
  `pg_store.rs` shows "1×" while its one instance records **301 edits**. 122 of the 406 rows
  are `.md` files (31 under `/memory/`): doc iteration miscounted as code anti-patterns.
- **The insight loop is write-only.** 1 of 1,478 recommendations was ever acted on (0.07%);
  `action_detail`/`evidence`/`default_acp` are populated on **0**; only 87 (5.9%) carry a
  handoff `prompt`. There is **no `created_at` column**, so the most natural triage sort for
  a 1,477-deep queue — age — isn't even computable.
- **`±0%` FTR-delta is a fabricated signal.** The cell ships a constant "±0%" under a "vs
  project baseline" tooltip, but `baseline_ftr` is populated on 0 of 1,478 rows and the
  delta is never computed. Per the honest-empty rule, a looks-measured-but-isn't cell is
  worse than an empty one.
- **Repetition is structural, not cosmetic.** 273 broken drift rows collapse to 98 distinct
  details (top: "Mentions InferenceAdapter" ×35); `expected/actual_signature` are NULL on all
  1,947 rows so identical observations can't be auto-deduped; and 502 files appear as *both*
  a `rework:` pattern **and** a `High rework:` recommendation — the same fact rendered twice.

**Recommendations (P0).** Require a non-null `description` at pattern write-time, synthesized
from `instances` (backfills all 943 to ~100% with zero new collection); roll up drift by
`(folder_id, detail)` before the UI (273 → ~98 actionable rows); require a `prompt` on any
actionable recommendation or don't emit it; remove or truthfully compute the `±0%` cell; rank
rework rows by `Σ instances.total_edits`, not `instance_count`, and add per-row
Enforce/Dismiss/Handoff actions wired to the existing (disconnected) recommendation layer.

## 3. Tool & content utility, and the missing registry

> "MCP tools are already covered … what we don't have is a clean registry of plugins,
> skills, agents." Evidence: deep-dive
> [06](2026-08-04-deep-dive/06-tool-content-utility-registry.md).

- The **"70% ignored" headline is largely a measurement artifact**: 68% of all 23,638
  verdicts are `ignored` purely because the model's next message didn't quote the tool's
  output ("no fragment overlap", "reaction is Stop"). The metric structurally penalizes
  scheduling/bookkeeping/output tools — `ScheduleWakeup` and `StructuredOutput` score
  `0.000`. The two utility subsystems even **contradict each other**: `tool_insights` labels
  `Read` a "win/workhorse" while the same row's `metrics` records `ignoredPct 0.735`.
- **Tool identity is not stable across renames** — the *same* svelte-autofixer has its
  history forked across `mcp__svelte__*` (55 verdicts) and `mcp__plugin_svelte_svelte__*`
  (116). 52.6% of registered tools have zero verdict data. 49.2% of `tool_insights` rows are
  pure filler (NULL variant + NULL title).
- There is **no unified registry**. Content lives in five disjoint silos
  (`assistant_tools` 114, `project_commands` 361, `library_skills` 4, `library_agents` 2,
  `mcp_tool_manifests` 6) with no shared provenance (sensei-provided vs library-derived vs
  insight-generated), version, or utility score. The registry is really an append log:
  it imported 2 dead MCP servers from a Zed `settings.json` (`connection_state=error`) and
  never evicts them.
- **Leak scan** came back clean (0 secrets in manifests; 0/6 servers with a populated env) —
  but there is **no `leak_scan_status` column anywhere**, so today's safety is luck, not a
  monitored control.

**Recommendations.** Redefine utility as an **outcome** signal (downstream edit / state-
change / phase-unblock), not fragment overlap; add a migration-stable `tool_identity` +
alias table so utility survives renames; build **one `content_registry`** unifying
tools/skills/agents/commands/MCP with provenance + version + utility + `leak_scan_status`;
run a leak-scan + over-broad-grant gate on every manifest re-probe and library sync; feed
utility + `last_used_at` into tool selection so the working harness resurfaces (the amnesia
in [metrics §3](2026-08-04-metrics.md#3-amnesia-durable-memory)).

## 4. Libraries — version conflict needs a handoff affordance

> "Libraries shows version conflict … generate a prompt as a handoff action, or send to
> intake." Evidence: deep-dive
> [07](2026-08-04-deep-dive/07-insight-pattern-recommendation-ux.md).

**113 libraries are pinned at conflicting versions** (e.g. `typescript` at 13 versions
across 21 folders), and the data model has **no resolution/action column** — the conflict is
detected and then stranded. The user's ask is exactly right and cheap to deliver.

**Recommendation (P1).** Add `sensei.library_conflicts` as a first-class table (version set,
folder list, status, link to a fixing `library_update` recommendation) with a **"generate
handoff prompt / send to intake"** button on the Libraries screen. The conflict already
exists in `referenced_libraries`; it just needs an action surface. (The barely-visible
conflict and the custom-rolled line-item toggle are §5.)

## 5. Component consistency (rokkit Switch / Toggle)

> "Why the odd on|off which does not have contrast?" / "rokkit toggle in some places,
> custom-rolled in others. No consistency."

This is a real UI-consistency debt and it is dogfooding-visible: the app mixes rokkit's
`Toggle`/`Switch` with hand-rolled toggles (Libraries line items, the `[project]` pill,
version-conflict rows), and the rokkit `Switch`'s `on|off` label lacks contrast in light
mode. It isn't in the daemon DB — it's a component-usage audit — but the repo already carries
the right reviewers for it: `.claude/agents/rokkit-components-reviewer.md` (customization-tier
+ typography) and `.claude/agents/rokkit-styles-reviewer.md` (named-token / contrast /
dark-mode). Recent commits (`d006ecc5`, `96ff9d66`) already moved native `<select>`/segmented
toggles onto rokkit `Select`/`Toggle` and fixed WCAG-AA contrast — this is the same thread,
not yet finished.

**Recommendations (P1).**
1. Run `rokkit-components-reviewer` over the app to find every hand-rolled toggle/switch and
   converge them on the rokkit component at the lowest customization tier that works.
2. Run `rokkit-styles-reviewer` on the `Switch` `on|off` state for the light-mode contrast
   defect (named tokens, not raw opacity), and add a Playwright light/dark snapshot so the
   contrast regression can't silently return.

## 6. Traceability repetition (tie-back)

The "traceability repeats the same observation" complaint has the same root as §2 and the
graph regression: `doc_coverage` is a **1.7 M-row** view over the duplicated `covers` edges,
and drift is stored insert-only without a signature. Fixing `covers` idempotency (§1) and
drift upsert-with-signature ([metrics §5](2026-08-04-metrics.md#5-regression--churn-new-lifecycle-themes))
collapses the repetition at the source rather than hiding it in the UI.

---

## Cross-cutting: one week, three regressions

The Atlas ship (`ff8299af`, 2026-07-12), the D2 version-rescan (`2f6f1de9`, 2026-07-12), and
the `covers` duplication explosion (week of 2026-07-13) all land together. The lesson for the
product is the same one metrics.md draws for the agent: **sensei collects and detects far more
than it verifies.** The graph, the patterns, the recommendations, and the tool registry each
render *something*, but none of them is checked against "is this the true, deduplicated,
actionable state?" — which is precisely what the honest-empty and verify-the-outcome rules
demand. The fixes above are mostly reconnecting a last mile that already exists.
