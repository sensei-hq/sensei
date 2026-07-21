---
type: design
---

# Playbook — module

Behind-the-scenes design for the front door: intake, classification, and
playbook recommendation that precedes any work chunk (feeds
[Working style](../features/04-project.md)). Absorbs
`docs/features/front-door/design.md` and `decisions.md` — those files remain
the fuller narrative; this is the code map.

## Axes + rule matrix (pure core)

- Crate: `crates/senseid/src/playbook.rs`.
- `Lifecycle` (`greenfield`/`stable`), `Intent` (`explore`/`ux`/`feature`/
  `enhancement`/`bug`), `Risk` (`low`/`high`) — `Axes` bundles the three, each
  with `as_str`/`parse`.
- `Rule { lifecycle, intent, risk, playbook, priority, .. }` — wildcard axis =
  empty string. `recommend(axes: &Axes, rules: &[Rule]) -> Recommendation`
  picks the highest-priority matching rule, falling back to `gsd` with a
  "no rule matched" rationale (`Recommendation.defaulted`).
- `is_trusted(risk, n, ftr) -> bool` — the auto-select gate (§ below).
- `learn(stats, rules) -> LearnPlan` — pure reweight/propose policy, no I/O
  (see Learning loop).
- All pure functions — unit-tested directly in this module, no DB/gateway
  needed.

## Classification (chunk text → axes)

- Handler: `crates/senseid/src/api/handlers/playbook.rs`.
- `classify_chunk(state, chunk, has_existing_code, blast)` calls the gateway's
  `reasoning` chain; on **any** error, timeout, or unparseable/invalid
  response it falls back to `heuristic_axes(text, has_existing_code, blast)`
  (keyword + blast-radius heuristic, pure fn, unit-tested). Both paths return
  `classified_by` (`"gateway"`/`"heuristic"`) and `model_fallback: bool`,
  recorded on the run so the analyzer can measure gateway pull-through.
- Direct-axes calls (`lifecycle`/`intent`/`risk` all present in the request
  body) skip classification entirely — always `classified_by = "manual"`.

## Endpoints (data contract)

- Routes: `crates/senseid/src/api/routes.rs` (`/api/playbook/*`).
- `GET /api/playbook/guide` → `get_intake_guide` — serves `frame` + per-axis
  prompts (`sensei.intake_guide` via `pg.list_intake_guide()`) + the catalog
  (`pg.list_playbooks()`). Fetched, not hardcoded, so it's org-tunable.
- `POST /api/playbook/recommend` → `recommend_playbook` — accepts either
  pre-classified axes or free-text `chunk`; runs `recommend()` against
  `pg.list_playbook_rules()`; persists a `playbook_run` row unless
  `preview: true` (`should_persist`); `confirm: true` marks it `confirmed`.
  Response is enriched with the playbook's `opening_tone`/`when_to_use`
  (`pg.get_playbook`) and, for low-risk axes only, an auto-select check via
  `pg.playbook_combo_trust` + `playbook::is_trusted`.
- `GET /api/playbook/rule-proposals`, `POST /api/playbook/rule/{id}/accept` →
  `list_rule_proposals` / `accept_rule` — surface + accept `source='learned'`
  proposals (§ Learning loop).
- `GET /api/playbook/model-stats` → `model_stats` — gateway vs heuristic
  pull-through, keyed off `classified_by`/`model_fallback`.
- Persistence: `crates/senseid/src/db/pg_store.rs` — `insert_playbook_run`,
  `list_playbook_rules`, `playbook_combo_trust`, `attribute_playbook_outcomes`,
  `playbook_combo_stats`, `apply_learn_plan`.

## Two surfaces, one endpoint set

- **CLI/agent** — `marketplace/plugins/sensei/commands/intake.md`
  (`/sensei:intake`). Procedure: call `get_intake_guide` → clarifying dialogue
  → `recommend_playbook(lifecycle, intent, risk, session_id)` → if
  `auto_select`, re-call with `confirm="true"` and announce; else state
  playbook + rationale, get explicit confirm (mandatory on `risk=high`), then
  `confirm="true"`. MCP tool wiring: `crates/mcp/src/lib.rs` (`get_intake_guide`,
  `recommend_playbook`, `list_playbook_rule_proposals`, `accept_playbook_rule`
  around lines 229–248, 555–581, tool allow-list ~947).
- **App** — `app/src/routes/(observatory)/intake/+page.svelte` +
  `intake.svelte.js` (state class `IntakeState`, owns `phase`/`recommend`/
  `confirm`). Loader seeds the guide once (`untrack`); recommend leg calls
  the daemon with `preview: true`; confirm leg re-sends the already-classified
  axes with `confirm: true` — exactly one `playbook_run` row per chunk.
  Session-less by design: passes the app's active session id if one exists,
  else `null` (not FTR-attributed — see below).
- **Five UI states** (`describe`/`loading`/`recommended`/`recorded`/`error`)
  live in `IntakeState.phase`; both surfaces implement the same state names,
  the app in the component above, the CLI implicitly in the procedure steps.

## Data model

- DDL: `database/ddl/table/sensei/{playbooks,playbook_rules,playbook_run}.ddl`.
- `playbooks` — text PK `name`; runtime-extensible catalog (`title`,
  `when_to_use`, `opening_tone`). Seed: `database/import/staging/playbooks.jsonl`.
- `playbook_rules` — ordered rule set incl. `priority`, `base_priority`,
  `source` (`seed`/`learned`), `enabled`. Seed:
  `database/import/staging/playbook_rules.jsonl`.
- `playbook_run` — one row per confirmed/preview-skipped recommendation:
  `session_id` (nullable, FK `activity.sessions`), axes, `rule_id`, `playbook`,
  `rationale`, `confirmed`, `classified_by`, `model_fallback`, `outcome`,
  `outcome_ftr`.

## Learning loop + auto-select

- Task: `crates/senseid/src/tasks/handlers/learn_playbooks.rs`
  (`TaskKind::LearnPlaybooks`), enqueued hourly by
  `crates/senseid/src/tasks/analyzer_scheduler.rs` (line ~99) alongside the
  other global analyzer passes; dispatch in `crates/senseid/src/tasks/mod.rs`.
- Pipeline: `attribute_playbook_outcomes()` (join FTR from the run's session,
  confirmed + FTR-scored runs only) → `playbook_combo_stats()` → pure
  `playbook::learn(stats, rules)` → `apply_learn_plan()`. Idempotent —
  reweights are computed off immutable `base_priority`, proposals upsert via
  a learned partial-unique index.
- Auto-select gate: `playbook::is_trusted(risk, n, ftr)` — `risk == Low` AND
  `n >= 10` AND `ftr >= 0.8` (deliberately stricter than reweighting — see
  `decisions.md`). High-risk never queries trust at all.
- Learned rule proposals are `source='learned'`, `enabled = false` until a
  human accepts via `POST /api/playbook/rule/{id}/accept`.

## Not yet built

- Intake/run history (S4) and playbook-learning-review (S5) screens — no
  route exists yet in `app/src/routes/(observatory)/` or `dojo/`; see
  `docs/features/front-door/design.md` §7 and `decisions.md` open decisions
  for placement (leaning Dōjō for S5 accept/reject).
- Axis-correction affordance on the recommendation card (override → re-recommend).
- Nudge hook activation for `/sensei:intake` — hook exists, ships off by
  default (separate gated decision).
</content>
