# Parallel lanes: week of 2026-11-03 (60% → 80%)

The coordinator (the reviewer session) owns this file. Each lane reads **its own section and §1–§3**.
The goal and the reasons for the order are in `docs/review/2026-11-03-orientation.md`.

## 0. Decisions in force (user, 2026-11-03)

| # | Decision |
|---|---|
| D47 | A **target** is a metric threshold. A target set to `block` *gates* work. The UI says "target missed (blocking)". "Gate" stays relay's human-approval word. |
| D48 | Target ladder: **Dōjō → Workspace → Repository → Sprint → Task type → Task**, most specific wins. Teams report results and own no targets. No Epic level in v1. |
| D49 | Tracker connections: **dōjō-owned for org workspaces** (Wave C, needs the #310 secret store); the daemon path stays for solo/personal use. New adapters (GitHub Issues, ADO) go on the dōjō path. |
| D50 | Ticket writeback (ac7822d2b) is **opt-in per workspace, off by default**. This amends D3. |
| D51 | The menu order in orientation §5 is approved: Cowork mocks it, then implements it. |
| F4 | A viewer pool with fewer than 2 contributors is withheld (supersedes D10 for viewer pools). |
| D52 | **A solo developer gets everything free**, blocking targets included, on their personal dōjō. **Paid targets are an org feature, and only for private repositories.** An org's public repositories get targets free. This follows D46's line (org private = subscribed) and narrows D24's "gating is paid" to org-private. |
| D53 | **The dōjō is the client; there is no client separation** (reaffirms D34). A session in a repository is governed by **that repository's dōjō** alone: its rules and its targets. No employer floor carries into another dōjō. A person's own `user`-rung rules still apply locally, below the dōjō by the scope ladder. Remove T3:113/225 and T2:118 ("client binding", "client's tracker and gates", "employer floor") and the membership-contexts line at objectives:76–78. |
| D54 | **Who sets targets:** Dōjō + Workspace levels = admin; Repository = maintainer+ on it; Sprint / Task type / Task = maintainer+ on any repository in the workspace. Leads read results per person (D38); viewers read aggregates only (D8). The manager persona splits: a member manager → maintainer or lead; an outside manager or client → viewer. |
| D55 | **No pairing.** The relay acts only through the signed-in person's dōjō session, with explicit, revocable permissions. R6 is rewritten to say this; vision:115 stands. |
| D56 | **The knowledge promotion path runs opposite to the governance ladder:** user → repository → workspace → dōjō → global shared. Each step outward is a promotion; it is never a precedence. Crossing out of a dōjō into global shared keeps k ≥ 3 contributors (D26/T1). `memory_scope` must gain the `user`, `repository` and `dojo` steps (today: global, workspace, stack, task_type, module). That is a schema item for the learnings lane. |
| D57 | **Governance precedence = the code** (`pg_store/governance.rs:61,339`: enforcement desc, then scope desc): the **stronger enforcement wins** (mandatory > required > recommended > advisory, and mandatory is never overridden); between rules of equal enforcement, the **most specific scope wins** (general → user → dōjō → workspace → repository). A repository can tighten a dōjō rule but cannot weaken it. (Corrected 2026-11-03: L0 had first stated it as "mandatory final, else most specific".) |
| D58 | **Lead** = the team lead (D38). `decisions.md:151` (Lead as the client-confidentiality role) is marked superseded. Add a Lead persona: sees teammates' metrics per person across the workspaces the lead can read; sets targets only if also a maintainer. |
| D59 | **Pricing** (P1–P3): SSO and audit are **free** (audit is governance, D19). The subscription is **per org, priced by the seats who work on its private repositories**; contributors to public repositories and viewers are never counted. The relay's team on-call queue (R8) is **free**. |
| D60 | **Vocabulary:** "solution" was the earlier name for a **workspace**, "org" in a ladder is the **dōjō**, "module" in a ladder is the **repository**. The multi-repository cascade is general → dōjō → workspace → repository. |
| D61 | **A missed blocking target is information plus a handoff, not a hard stop** (F1). Sensei says which targets are missed and what to fix, and hands the fix to the assistant to clear them. On the dōjō side a **lead or admin accepts the deviation** with a note, and may open a follow-up ticket for the issues the miss identified. Opening a ticket writes to the tracker, so it follows D50: opt-in per workspace; otherwise Sensei drafts the ticket for the lead to file. |
| D62 | **Targets reach the assistant through MCP** (F2), together with the ticket's own rules, at the start of work on a ticket. |
| D63 | **A ticket is the unit of work handed to the assistant** (F3). The sync lists the tickets assigned to the user. The user hands the assistant a ticket key; the assistant fetches it through a **tracker-agnostic MCP tool served by the daemon**. That fetch **marks the start of work on the ticket** and links the session to it. Branch/commit/prompt key matching stays as the fallback, for an agent that fetched the ticket elsewhere (e.g. `gh issue view`). |
| D64 | **A personal dōjō is created at sign-in** (F4). |
| D65 | **Learning verbs:** one verb, **Share →**, along the D56 promotion path; **Reinforce / Retire**, driven by Impact's verdict; "skill" becomes a memory kind (F5). |
| D66 | **Two misses of a blocking target on the same cause propose a memory** at the user step; it is offered for promotion up the D56 path once it helps (F6). |
| D67 | **"What applies here"** on the desktop Dōjō screen: the resolved rules and targets for the current workspace, each with its source (F7). |
| D68 | **A minimal viewer shell this week** (F8): the granted workspaces, **metrics first**, to judge whether the overall workflow is healthy; target results in aggregate; nothing else. |
| D69 | **The dōjō Home is a "needs you" queue** (F9): missed targets awaiting acceptance, a sync that has stopped, approvals. No email or Slack this week. |
| D70 | **Menus** (a, b, c): "Needs you" becomes **Act**; Constitution moves into the dōjō's **Govern** group; setup's "Join your dōjō" step is **skippable**. |
| D71 | **Ticket data is read live** (Q1): `get_ticket` and "assigned to me" are live tracker reads made with the person's own access. No person data is stored; the `tracker_tasks` mirror is unchanged. For GitHub Issues the daemon uses the person's own forge sign-in, read-only, until dōjō-owned connections (D49) exist. |
| D72 | **warn vs block** (Q2): a `warn` miss is shown and needs no acceptance. A `block` miss is handed to the assistant, enters the "needs you" queue (D69), and needs a lead's or admin's acceptance (D61). |
| D73 | **"The ticket's rules"** (Q3) = the repository's resolved rules (D57) plus the ticket's own acceptance criteria. There is no ticket level in governance. |
| D74 | **Neutral Impact verdict** (Q4): the memory is kept by default, and Retire is offered. |
| D75 | **The paid plan is called "Pro"** (Q5, user). Messages name it: "Targets on private repositories need Pro." **The price is "to be announced"** until a payment provider exists; the `$12` code constant is not a decision. |
| D76 | **"What applies here"** (Q6) lives on the desktop Dōjō screen, scoped to the current workspace; Workspace › About links to it. |
| D77 | **SCIM is free**, along with SSO (Q7, D59). |
| D78 | **The dead client routing** (`crates/senseid/src/dojo/routing.rs` and its two `.feature` scenarios) is removed (Q8, D53). |
| D79 | **Govern shows to everyone, with a minimum role per item** (the D70 Constitution move): Constitution for every member; Triage, Approvals and Knowledge for maintainer and up. |
| D80 | **Narrower target levels may only tighten** (user): a level below can raise the bar or turn `warn` into `block`, never lower either. Loosening needs the role that set the wider level (D54). Resolution takes the strictest applicable level, which keeps targets consistent with D57's "stronger enforcement wins". |
| D81 | **Target measurement** (user, accepting the spec's Q1–Q5): the rule-check target is absolute (zero failing hard rules on the change's tree); a hard rule is a checker rule whose resolved enforcement is mandatory or required, resolved as `get_rules` resolves it (adoption overrides included); baselines are per repository, metric and path filter; sprints and task types use the tracker's own keys (no invented issue-type mapping); a coverage reading counts once 50% of guarded files appear, and a graph reading once the index is complete (`folder_completeness.subtree_complete`). |
| D82 | **"Moving the wrong way"** (user, WP5): compare the second half of the window with the first. A repository is flagged when a metric that has a direction (`sensei.metrics.direction` higher_better or lower_better) moved the wrong way by **≥ 10%**, and only when **each half has at least 5 days** of readings. `neutral` metrics are never flagged. The panel names the worst repository as its next step. Direction reaches the dōjō wire through L1's own `workspace_metric_series`. |
| D83 | **No fabricated values, anywhere** (user, 2026-11-03): showing a made-up number, list, plan or status when the real data is unavailable is misinformation. Every layer (SQL, daemon, MCP, both apps, shared packages) surfaces a failed read as an error state and never as a plausible value. The system-wide audit (`coord/audits/fabrication-2026-11-03.md`) assigns each failure-mask to its owning lane, and a ratchet guard in `make test-fast` stops new ones. Honest-empty is correct only when the data truly is empty. |
| D84 | **S6 (diagram history) waits until every sync works** (user): after #314's production verification, not this week. S6's Q1–Q6 stay open until then. |
| D85 | **The billing screen is Cowork's new mockup work of 2026-11-03** (user). The earlier mockups in `docs/mockups/Sensei/` are its reference. The decision-driven fixes (Pro, "to be announced", nothing free listed as paid) apply. |

The coordinator writes these into `docs/decisions.md` after Muse's WP4 commit lands. That file is in
Muse's index right now.

## Phase 0: the docs first (user, 2026-11-03)

The lanes start only once the PRD, the journeys and decisions.md reflect D47–D58 and the open
journey/flow answers. L0 runs this sweep itself; L6 keeps only the roadmap status cells, `backlog.md` and
the dead `Spark.svelte`.
- Pass 1 (running): vision, objectives, personas and every module except T3 and M9, plus the
  "Decision it drives / next step" column across all Signals tables.
- Pass 2 (after the flow answers): T3 (targets, D48/D52/D54 and what "blocking" blocks), M9 (D56
  promotion path and verbs), and `docs/journeys/*`.
- Then decisions.md (D47–D58 + F4), once Muse's WP4 commit releases the file.

## 1. How lanes work (every lane, no exceptions)

1. **One worktree and one branch per lane.** No lane edits the main checkout
   (`~/Developer/sensei-hq/sensei`); it belongs to the coordinator. Commit the slice you have in progress
   there first (your own paths only, `git diff --cached --stat`), then:
   ```bash
   git -C ~/Developer/sensei-hq/sensei worktree add ~/Developer/sensei-hq/lanes/<lane> -b lane/<lane> origin/develop
   cd ~/Developer/sensei-hq/lanes/<lane> && (cd dojo && bun install) && (cd app && bun install)
   ```
2. **Stay inside your own paths (§2).** To change a path another lane owns, write the change you need in
   your handoff; do not make it.
3. **TDD.** Watch the test fail locally, then commit it together with its implementation. Never commit a red
   test on its own. Gate every commit on the real checks: `bun run check` + `bun run test` (dōjō/app),
   `cargo test -p senseid` (full), `make test-fast`.
4. **Rebase on `origin/develop` every morning,** and again before each handoff.
5. **Never run these** (the coordinator's job): `make install*`, `make bump/release`, `dbd` against the
   live daemon DB (`localhost:5432/sensei`), `git push` to develop or main, edits to `docs/CHECKPOINT.md`
   or `docs/decisions.md`.
6. **Local Supabase (`:54322`) is shared.** Reconcile it only while holding the lock:
   `mkdir ~/.sensei/locks/supabase && dbd reconcile … ; rmdir ~/.sensei/locks/supabase`. If `mkdir`
   fails, someone else holds it, so wait. Use dbd reconcile/import, never hand-applied psql.
7. **Your checkpoint lives in `docs/lanes/<lane>.md`:** one current entry, under 40 lines, with slice,
   done/remaining, next command, open questions, known-broken. It replaces `docs/CHECKPOINT.md` for lanes.
8. **Handoff = push your branch + write a report in `~/Developer/sensei-hq/coord/inbox/`.** Follow
   `~/Developer/sensei-hq/coord/README.md`: tests first (phase A, disabled, then wait for the test review),
   then implement (phase B). Reviews come back in `coord/reviews/`; progress is in `coord/TRACKER.md`. Tell
   the user only "reported". **Cowork:** start each mockup from its data brief, `coord/briefs/L4-<item>.md`.
9. **Every screen-facing change** follows presentation | state | load and uses rokkit components first.
   Colours, fonts and shapes come only from rokkit/UnoCSS named tokens: no `<style>` colour literals, no
   `font-family`, no inline `style=` colours. Fix the look upstream in rokkit. Every panel names its next
   step. These are review blockers.
10. **New dependencies or a shared-hotspot change (§3):** list them in the handoff before you rely on them.

**Dev ports:** Muse dōjō 5174 · Claude Code B dōjō 5176 · Cowork dōjō 5175. Only Cowork runs
`make app-dev`; the other lanes use vitest/harness specs.

## 2. Lanes, owned paths, and order of work

### L0 · Coordinator (this session): plan, specs, review, merge, deploy
Owns: `docs/plan/**`, `docs/review/**`, `docs/spec/dojo/targets-and-gates.md`, `docs/CHECKPOINT.md`,
`docs/decisions.md`, merges to develop, local + live reconcile, `make install`, release.
1. Today: the **targets & gates spec**, in the detail Muse works best from (DDL, functions, routes, tests by
   name, acceptance on live data). It unblocks L3.
2. Review each handoff the same day; merge; reconcile; install; live-check.
3. Day 2–3: the **learnings loop spec**, for L1 once #314 is done.
4. Day 5: the release checklist, then cut the release from main.

### L1 · Muse: sync (spec: `docs/plan/dojo-sync-handoff.md`)
Owns: `crates/senseid/src/tasks/{dojo_sync,graph_push}.rs`, `crates/senseid/src/db/pg_store/sync.rs`,
`crates/senseid/src/dojo_client/**`, `dojo/src/routes/v1/you/**`,
`dojo/src/routes/v1/t/[origin]/[org]/workspaces/**`, `dojo/src/lib/server/{workspaces-data,repositories,graph-ingest,metrics-ingest}.ts`,
`dojo/src/lib/workspace-metrics-*`, `dojo/src/routes/(app)/org/[slug]/workspaces/[id]/**`, DDL
`database/ddl/**/dojo/{all_my_repositories,register_repositor*,*_metric_series,repository_*}` and their
tests under `database/tests/dojo/**`; `docs/lanes/L1.md`;
`dojo/src/lib/components/screens/ScrWorkspacePreview.svelte`; for WP5 round 2 also `kit/MetricCards.svelte` (new),
`kit/Spark.svelte`, `components/Spark.svelte`, `my-metrics-view.ts`, `ScrMyMetrics.svelte`.
1. **WP4:** apply review round 2 (F4 `having count(*) >= 2`, a shared `isUuid`, F8), unstage
   `packages/codemap/focus.*`, commit in the main checkout, then move to the lane worktree.
2. **WP8** (D46, `all_my_repositories`), then re-run the WP4 suite.
3. **WP9 Part A** (set-based `register_repositories`).
4. **WP5** workspace metrics screen. Leave a slot per series for its target (it fills once L3 lands).
   The screen must show a next step: say which repository is moving the wrong way.
5. **#314** production sync verification. The coordinator does the install; Muse writes the query
   script and the expected counts.
6. Then: the learnings loop (spec from L0).

### L2 · Claude Code A: diagrams + architecture metrics
Owns: `packages/codemap/**`, `app/src/routes/(workspace)/workspace/[id]/diagrams/**`,
`crates/senseid/src/analysis/**`, `crates/senseid/src/api/handlers/diagrams.rs`,
`crates/senseid/src/api/diagram_cache.rs`, new `crates/senseid/src/tasks/handlers/metrics/architecture*.rs`,
**new rows only** at the end of `database/import/staging/metrics.jsonl`;
`crates/senseid/src/db/pg_store/tests/metric_rows_tests.rs`, `database/ddl/view/sensei/workspace_metric_daily.ddl` (S4 2c).
For #156: `crates/senseid/src/tasks/handlers/metrics/mod.rs`, a new `pg_store` read for module → repository, and one
entry per new metric in `docs/features/metrics/catalog.md` (the `the_catalog_documents_exactly_the_registered_metrics` guard).
1. Commit the uncommitted `packages/codemap/*` work (the S7 tail), then move to the lane worktree.
2. **S4 2c:** check the ratio pooling double count and fix it. Conservation test: pooled numerator equals
   the sum of the per-commit numerators, counted once per commit.
3. **#156 architecture metrics** as metric rows computed from the analysis the lenses already use:
   cycle count, the largest cycle, zone-of-pain module count, hidden-coupling pairs above threshold,
   complexity outliers. Reuse `crate::analysis`; do not reimplement it. Each metric ships with its
   "what to do" copy (the next step), reused through `insight_copy`.
4. Then S6 (diagram history), only after the user says yes.

### L3 · Claude Code B: metrics that act, then targets
Owns: `app/src/routes/(workspace)/workspace/[id]/metrics/**`; **for ActionItems, the whole vertical slice:** `database/ddl/table/inference/recommendations.ddl` (+ its tests), `crates/senseid/src/api/handlers/workspace_detail.rs` (the recommendation handlers), `crates/senseid/src/db/pg_store/patterns.rs`, `crates/senseid/src/tasks/handlers/generate.rs`, and the matching functions in `app/src/lib/api.ts`/`types.ts` (additive only: §3); new `database/ddl/**/{sensei,dojo}/target*`
and `metric_gating*`, new `database/import/staging/metric_gating.jsonl`, new
`crates/senseid/src/targets/**`, the target routes under `dojo/src/routes/v1/t/[origin]/[org]/targets/**` and
`dojo/src/lib/server/targets-data.ts`.
1. Now, while L0 writes the spec: **ActionItems act, as one cohesive unit** (user, 2026-11-03): DDL → store → handler → API → state → screen, in one lane so the contract never splits across sessions. API changes are **additive** (new fields or routes, never a renamed or removed one), because L5's overview refactor reads the same recommendations API; L5 rebases onto it. Schema changes converge with `dbd reconcile` under the Supabase/daemon lock rules (§1.6); the live daemon DB is L0's.
   **ActionItems act.** Today `ActionItems.svelte` has 0 buttons or
   links. Each item gets the one action it implies: open the file or lens, open the memory, propose a rule
   (the existing `playbook_rules` path), or snooze. Do the state in `action-items.ts`, the specs first,
   and the presentation last.
2. Once the spec lands: **targets**. Gating eligibility as its **own** table and import
   (`metric_gating.jsonl`, keyed by metric key), so it never edits L2's `metrics.jsonl` lines. Then the
   target store and resolver (D48 ladder), the daemon pull, evaluation per sprint over the linked
   tickets, the result push, and the desktop "target missed" notice through `@sensei/ui` Notice (it does
   not fork it).

### L4 · Claude Cowork: UX/mockups → UI
Owns: `packages/ui/**`, `docs/mockups/**`,
`app/src/lib/components/{EmptyState,ScreenState,DaemonStatusBanner,Notice*}*`,
`dojo/src/lib/components/kit/{EmptyState,LoadError,Banner}*`, `app/src/routes/(preferences)/settings/dojo/**`,
`app/src/routes/(observatory)/dojo/**`, `app/src/routes/(observatory)/share-review/**`,
`app/src/routes/(observatory)/observatory-nav.ts`, `(preferences)/settings/settings-nav.ts`,
`(workspace)/workspace/[id]/workspace-sidebar-view.ts`, `app/src/routes/(config)/**`, `dojo/src/lib/nav.ts`,
the billing and sync-status screens (`ScrBilling*`, `ScrSharing*`) and `dojo/src/routes/(app)/org/[slug]/[section]/**`.
1. **Notice family:** finish it, and take in empty state, load error and banner, so there is one
   `@sensei/ui` family that both the app and the dōjō use. Delete the per-app copies.
2. **Dōjō sync screen revamp:** every repository row says why it is in that state and what to do (the
   WP1 verdict + remedy).
3. **Menu reorder (D51):** rail Today · Workspaces · Act · Dōjō (one entry merging connections, sharing
   and share review) · Review · Settings. Setup gets a skippable "Join your dōjō" step. Then the workspace
   window and the dōjō org order (orientation §5).
4. **Targets UI mockups:** the "Set targets" workspace tab and the failing-target path (sprint scorecard
   row → desktop notice → the fix). Build them once L3's API is in.
5. **Billing screen** (D46: show the plan and what is entitled, honestly; there is no payment provider,
   so do not pretend there is one).
6. If time: the **viewer shell** (granted workspaces → metrics/reports only).

### L5 · OpenCode: presentation | state | load refactors (no behaviour change)
Owns exactly: `app/src/routes/(observatory)/{instruments,libraries,knowledge-sources,activity-logs}/**`,
`app/src/routes/(preferences)/settings/{workspaces,extensions}/**`,
`app/src/routes/(workspace)/workspace/[id]/{about,overview}/**`, the state modules beside them, and (L0 ruling)
`app/src/lib/knowledge-sources.svelte.ts`, `app/src/lib/state/mcp.svelte.ts`,
`app/src/lib/components/settings/{workspaces-section-state.svelte.ts,WorkspacesSection.svelte}` (the last two only after
Cowork's Notice commit lands). Page order: libraries → knowledge-sources → settings/extensions → activity-logs →
about → overview (after L3's ActionItems) → settings/workspaces (after Notice) → instruments.
For each page, one commit per page:
- Move the fetch out of `onMount`/`$effect` into `+page.ts` `load` (or a page state when slow, per
  rule a3b74a4bd).
- Move every write into the state class.
- The component only renders and calls state methods.
- Replace `confirm()` (overview) with the in-page confirm pattern already used elsewhere.

Each page's spec must pass **before and after** with no edit to its assertions; add specs where none
exist, before the move. Order: libraries → knowledge-sources → settings/workspaces → settings/extensions
→ activity-logs → about → overview → instruments (1,035 lines, last, split into components).
Do not touch `(observatory)/+layout.svelte`; list it in the handoff.

### L6 · Freebuff: docs consistency + dead code
Owns: `docs/prd/**`, `docs/roadmap/**`, `docs/backlog.md`, `docs/journeys/**`, `docs/lanes/L6.md`.
Work through orientation §6 in this order:
1. Template: add the required column **"Decision it drives / next step"** to "Signals shown" in
   `docs/prd/modules/README.md`, then fill it in for the 17 tables that have no action word.
2. Projects → workspaces in M15, M16, T2 and T3 (paths and examples, against the real route names).
3. Apply D53 (remove client binding: T3:113/225, T2:118, objectives:76–78) and D54 (split the manager
   persona; who sets targets, into T3). Apply D58: add the Lead persona, and **list** decisions.md:151 for
   L0 (do not edit decisions.md).
4. Apply D57 to M13:11–12. Write the D48 ladder and D54 into T3. Apply D52 to T4 and T2:58 (the paid
   tier). Apply D55 to objectives R6. Apply D56 to M9:35 and vision:191 (the promotion path). Fix
   T3:155 (code hosts are not trackers), self-hosting (D24) and the solo persona (D46/D52). Fix P2's
   `website/` path (#346). Add D40.
5. Roadmap 01–05 status cells and `backlog.md`, from the roadmap audit in the orientation §2 table.
6. ~~Delete `Spark.svelte`~~: withdrawn. It is live (`kit/Spark.svelte:2` wraps it); L6 verified this and correctly did not delete it.

Every edit cites the decision (D-number) or file:line it follows. **No new product decisions:** anything
unclear goes in the handoff as a question.

### L7 · Integrations helper: trackers (D49)
Owns: `crates/senseid/src/tracker/**`, `app/src/routes/(workspace)/workspace/[id]/{tickets,integrations}/**`,
and, for Wave C, new `dojo` tracker DDL/routes under `tracker*`.
1. D50: make the writeback opt-in per workspace (off by default), with a test that a default workspace
   writes nothing.
2. Put GitHub Issues and ADO on the dōjō-owned path (D49). This needs #310 (the secret store) first.
   Agree its spec with L0 before writing code.
3. L3 reads `sensei.sprints`, `tracker_tasks` and `tracker_task_sprints` read-only. Announce any change to
   their columns in the handoff.

### L8 · D83 cleanup: the unowned failure-masks (agent: to be assigned)
Source: `~/Developer/sensei-hq/coord/audits/fabrication-2026-11-03.md` (Appendix A, all 69 HIGH with read,
misinformation, fix). Owns the files of each HIGH row marked UNOWNED, one batch at a time, taking a file
only while no other lane holds it (L0 confirms each batch's file list before it starts).
0. **The guard first** (audit §6): `scripts/check-no-fabrication.py` + `scripts/fabrication-baseline.json`
   (per rule per file; only goes down) + `scripts/fabrication-allow.tsv` (reviewed exceptions, matched by
   line text; a stale row fails), with a `--test` self-check, wired into `make test-fast`, `make test` and
   CI. It lands before any fix, so the baseline captures today and nothing new gets in.
1. **Batch A, data loss:** `bootstrap/config.rs:329`, `cli/main.rs:677`, `api/handlers/workspace.rs:334`,
   `metadata/frontmatter.rs:144`, `assistants/helpers.rs:141`, `cli/managed.rs:73`. A failed or unparseable
   read **refuses to write**; it never overwrites with defaults.
2. **Batch B, governance and identity:** `knowledge.rs:1292` (shared with L1's pack-rule item: L1 owns
   it, L8 skips it), `stance.rs:72`, `advance_run.rs:531`, `auth.rs` (1014, 1026–27, 1151, 1260, 1277,
   1412–42), `repo_scan.rs:734–45`, `pg_store/sessions.rs:1204/1293`, `pg_store/mod.rs:1091`,
   `import_assistant_events.ddl:31,35`, the seat writes (`publish_run.rs:115`; the dōjō relay route is
   L1's). Fail closed on safety and identity.
3. **Batch C, misleading numbers:** `pg_store/metrics.rs:223,1341` + `workspace_quality_signals.ddl:40`
   (no data is "not measured", never 0%), `tool_insights.rs:30` (no `filter_map(ok)` + zip), the CLI
   verdicts (reuse MCP's `daemon_result` split; no second copy), `watchdog.rs:88`, `tool_discovery.rs`,
   `indexer/community.rs:381`.
MEDIUM/LOW (409 rows) are left to the ratchet: each lane that touches such a file lowers its count
(the boy-scout rule). They are not a separate project this week.
Lanes fix their own rows as a "D83" item: L1 ×7, L3 ×4, L4 ×5, L7 ×1 (Appendix A lists them).

## 3. Shared hotspots (append-only, small, rebase before you hand off)

| File | Rule |
|---|---|
| `crates/senseid/src/api/routes.rs`, `tasks/mod.rs`, `crates/mcp/src` tool registry | add your lines only, in one hunk per lane |
| `app/src/lib/{api,types}.ts` | add new functions/types at the end; never rename a type there (renames are L0's call) |
| `database/design.yaml`, `Cargo.toml`/`Cargo.lock`, `bun.lock` | list the change in the handoff; L0 merges |
| `database/import/staging/metrics.jsonl` | L2 appends rows; nobody else edits it |
| `docs/decisions.md`, `docs/CHECKPOINT.md` | L0 only |

## 4. Merge order the coordinator follows

L1 WP4 → L2 S7 tail → L4 Notice family (it touches components that L3/L5 render) → L5 page by page → L6
docs → L1 WP8 → L1 WP9A → L3 ActionItems → L2 S4 2c → L2 #156 → L3 targets → L4 targets UI → L1 WP5 → L1 #314.
A later merge rebases onto the earlier ones. A failing gate stops that lane's merge only.
