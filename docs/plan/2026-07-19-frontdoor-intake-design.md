---
name: Front-door intake + recommender — design
date: 2026-07-19
status: design — approved in brainstorm 2026-07-19 (rev 2: layered guidance + DB-backed playbooks); implementation plan next
spec: docs/plan/operating-model.md §3.3 (Playbooks — the front door + the method catalog)
phase: Operating-model Phase 2, sub-spec #1 (front door first)
---

# Front-door intake + recommender — design

The first sub-unit of Operating-model **Phase 2**. Phase 2 has two subsystems — the
**front-door intake + recommender** and the **6-playbook method catalog**. Decomposition
call (brainstorm 2026-07-19): **front door first**; the playbooks start as *named routes*
here (name + when-to-use + an opening-tone stub) and each deepens into a full executable
method in later catalog specs.

## Problem

§3.3: every work chunk should enter through an **intake conversation** — intent →
clarifying dialogue → resolve ambiguity → **recommend-and-confirm a playbook with
reasoning**. The dialogue is "exactly the analyst mindset / `/sensei:idea` +
`/sensei:brainstorm` we already have — the gap is making it *always* the entry and making
it *emit a chosen playbook*." Net-new: an always-on entry point, the *guidance content*
that grounds the conversation, a recommender (chunk → playbook), and a persisted decision
the learning loop (§9) can attribute outcomes to.

## Decisions (brainstorm 2026-07-19)

1. **Surface = a `/sensei:intake` command + a `recommend_playbook` MCP tool.** The command
   is the explicit human entry; the tool exposes the recommendation programmatically. "Always
   the entry" is nudged via session-start guidance + a light hook (activation deferred).
2. **Recommender = hybrid.** A local-first LLM (via the gateway) classifies the chunk into
   **axis values**; a **deterministic rule set** maps axes → playbook + rationale.
3. **Axes are the primitive; "situations" are just rule labels.** Three *extensible* axes
   (`lifecycle`, `intent`, `risk`), with a rule set layered over them. Open for extension:
   add axis values, add playbooks, add rules — driven by what works. The §3.3 six situations
   become **seed rules**.
4. **Rules and playbooks are DB-backed and runtime-extensible.** `playbook` is a **text FK to
   a `sensei.playbooks` table**, not a frozen enum — adding a method (or a rule) is an INSERT,
   not a release. §9 / Dōjō tune both at runtime.
5. **A first-class `sensei.playbook_run`** records each intake→decision (axes + matched rule +
   playbook + rationale + nullable `outcome`), the attribution spine for §9.
6. **Modifiable conversational guidance is layered** (global + per-axis + per-playbook), stored
   as data so one source drives both an LLM questionnaire (now) and a schema form (later) —
   **both rendered in Sensei** (the per-user interaction layer). Dōjō is the cross-team
   **control plane** that *authors/governs* this content (org overrides), not a renderer:
   - **global frame** — grounds the whole intake (org-tunable);
   - **per-axis elicitation** — how to determine `lifecycle`/`intent`/`risk` (drives the
     questionnaire/form; the axis's allowed values come from its enum);
   - **per-playbook opening tone** — sets the tone of the next stage once a playbook is chosen.
   Rules stay pure axes→playbook (their `rationale` explains the choice); a per-rule tone
   override is a deferred YAGNI.

## 1. Data model (new DDL, `sensei` scope, via `dbd`)

### Axis enums (stable, additively extensible)

- `sensei.chunk_lifecycle`: `greenfield`, `stable`.
- `sensei.chunk_intent`: `explore`, `ux`, `feature`, `enhancement`, `bug`.
- `sensei.chunk_risk`: `low`, `high`.

Axis *values* change rarely, so enums (extended additively via `dbd`) are fine. If runtime
axis-extension is ever needed, migrate an axis to a lookup table (deferred). NB: dbd deploys
enum variants **alphabetically**, not declaration order — key on the label, never on ord.

### `sensei.playbooks` — the runtime-extensible method registry

| column | type | notes |
|---|---|---|
| `name` | text pk | stable id (`vibe`, `mockup_first`, `spec_driven`, `gsd`, `change_flow`, `debug_flow`) |
| `title` | text not null | display name |
| `when_to_use` | text not null | one-line guidance |
| `opening_tone` | text not null | modifiable prompt that sets the next stage's tone (stub in v0) |
| `method_ref` | text null | pointer to the full method (filled by later catalog specs) |
| `enabled` | bool not null default true | |
| `source` | text not null | `builtin` \| `org` \| `learned` (check) |
| `created_at` | timestamptz default now() | |

Seeded with the 6 named routes. Adding a method later = INSERT (no release).

### `sensei.playbook_rules` — the open-for-extension rule set

| column | type | notes |
|---|---|---|
| `id` | uuid pk | |
| `name` | text not null | human-readable "situation" label |
| `match_lifecycle` | `sensei.chunk_lifecycle` null | NULL = wildcard |
| `match_intent` | `sensei.chunk_intent` null | NULL = wildcard |
| `match_risk` | `sensei.chunk_risk` null | NULL = wildcard |
| `playbook` | text not null (fk `playbooks.name`) | the recommendation |
| `rationale` | text not null | shown at confirm |
| `priority` | int not null | higher wins on multi-match |
| `enabled` | bool not null default true | |
| `source` | text not null | `builtin` \| `org` \| `learned` (check) |
| `created_at` | timestamptz default now() | |

Index: `playbook_rules_match_idx on (enabled, priority desc)`.

**Seed rules (`builtin`), encoding §3.3** — priority resolves overlaps (high blast-radius
forces deep design regardless of intent):

| name | lifecycle | intent | risk | playbook | priority |
|---|---|---|---|---|---|
| clear + high blast-radius | — | — | high | `spec_driven` | 100 |
| greenfield, objective fuzzy | greenfield | explore | — | `vibe` | 60 |
| greenfield, UX-heavy | greenfield | ux | — | `mockup_first` | 60 |
| stable product, bug | stable | bug | — | `debug_flow` | 60 |
| stable product, enhancement | stable | enhancement | — | `change_flow` | 50 |
| known feature, low risk | — | feature | low | `gsd` | 40 |

### `sensei.intake_guide` — the modifiable conversational guidance

| column | type | notes |
|---|---|---|
| `id` | uuid pk | |
| `kind` | text not null | `frame` \| `axis` (check) |
| `axis` | text null | for `kind='axis'`: `lifecycle`/`intent`/`risk` (else null) |
| `prompt` | text not null | grounding (frame) / elicitation question (axis) |
| `help` | text null | optional guidance/examples shown with the question |
| `enabled` | bool not null default true | |
| `source` | text not null | `builtin` \| `org` \| `learned` (check) |
| `created_at` | timestamptz default now() | |

Seeded with one `frame` row + three `axis` rows (lifecycle/intent/risk). The axis rows' *options*
are the corresponding enum's values; the `prompt`/`help` are what the questionnaire (or form) shows.

### `sensei.playbook_run` — the chunk decision record

| column | type | notes |
|---|---|---|
| `id` | uuid pk | |
| `session_id` | fk existing session table, not null | ties the run to the session + its conversation history (a classifier signal source); §9 correlation |
| `feature` | text null | feature slug when the chunk maps to a dossier |
| `lifecycle` | `sensei.chunk_lifecycle` not null | classified axis |
| `intent` | `sensei.chunk_intent` not null | classified axis |
| `risk` | `sensei.chunk_risk` not null | classified axis |
| `rule_id` | uuid null (fk playbook_rules) | matched rule (null = default fallback) |
| `playbook` | text not null (fk `playbooks.name`) | the confirmed choice |
| `rationale` | text not null | shown at confirm; persisted for audit |
| `confirmed` | bool not null default false | recommend-and-confirm outcome |
| `outcome` | text null | §9 attribution target — populated later by the learning loop |
| `created_at` | timestamptz default now() | |

Index: `playbook_run_session_idx on (session_id)`.

## 2. Recommender resolution (pure over the rule set)

`recommend_playbook(axes) -> { playbook, rationale, rule_id }`:

1. Read `enabled` rules ordered by `priority desc`.
2. A rule **matches** when every non-null `match_*` equals the corresponding axis value
   (null match column = wildcard).
3. First match (highest priority) wins → `{playbook, rationale, rule_id}`.
4. **No match** → default `gsd` with a "no rule matched — defaulted" rationale and
   `rule_id = null`; the run is flagged so rule-set gaps surface (never silent).

Pure and testable: expressible as one SQL `WHERE` (`match_x IS NULL OR match_x = $x`)
`ORDER BY priority DESC LIMIT 1`, or a pure function over a fetched rule vec. The resolver
takes the rule set as input, so it is DB-source-agnostic.

## 3. Classification — `classify_chunk`

LLM classification (gateway, local-first; heuristic fallback), **grounded by `intake_guide`**
(the `frame` + per-axis `prompt`/`help`), of the intake dialogue + project context into the
three axis values:
- `lifecycle` — primarily from project state (spine present / brownfield vs greenfield; drift).
- `intent` — from the dialogue.
- `risk` — informed by the **code-graph blast-radius** (callers/community reach) + intent.

Output is validated against the axis enums before it reaches the resolver.

## 4. `/sensei:intake` command — orchestration

`load intake_guide → run questionnaire (frame + per-axis prompts) → classify_chunk →
recommend_playbook → recommend-and-confirm → persist playbook_run → surface the chosen
playbook's opening_tone to prime the next stage`.

**Delivery duality:** the questionnaire is driven by `intake_guide` **data**. `/sensei:intake`
renders it as an **LLM-guided questionnaire** (this spec). A future **Sensei app** surface (the
desktop Tauri/SvelteKit `app/`) renders the same rows as a **schema form** (later) — same data,
no separate content. Both renderers are **Sensei** (per-user interaction); **Dōjō** only
governs/authors the content across teams.

**Recommend-and-confirm:** show playbook + rationale; one-tap confirm. Human-in-the-loop is
required when `risk = high`; low-risk auto-confirm is deferred (trust). On confirm, write the
`playbook_run` (`confirmed=true`) and surface the playbook's `opening_tone`.

## 5. Always-the-entry

- **Session-start guidance** names `/sensei:intake` as the entry for any new work chunk.
- **A light PreToolUse/session hook** detects substantive work starting without a `playbook_run`
  for the session and *nudges* into intake (non-blocking).
- **Hook activation is a separate gated decision** (relay B-gate posture): built OFF by default;
  Jerry decides activation. This spec ships the nudge logic, not its enforcement.

## 6. Learning-loop seam (§9)

`playbook_run.outcome` (nullable) is the attribution point. This spec only *records* runs; a
later §9 unit populates `outcome` and inserts/reweights `source='learned'` rows in
`playbook_rules` (and can tune `intake_guide`/`playbooks`) — the system learns by **growing the
DB content**, not editing frozen code. No §9 logic here; the seam is the schema.

## Units & interfaces (isolation)

| Unit | Responsibility | Interface | Depends on |
|---|---|---|---|
| DDL | axis enums + `playbooks` + `playbook_rules` + `intake_guide` + `playbook_run` + seeds | schema | dbd |
| `recommend_playbook` (pure) | axes + rules → playbook | `fn(axes, &[Rule]) -> Recommendation` | enums |
| pg_store | CRUD for playbooks/rules/intake_guide; run insert/read | `list_rules`, `list_playbooks`, `list_intake_guide`, `insert_playbook_run`, … | DDL |
| `classify_chunk` | guide + dialogue + context → axes | `fn(...) -> Axes` (LLM + heuristic fallback) | gateway, graph, intake_guide |
| MCP `recommend_playbook` | expose recommender | tool → resolver | resolver, pg_store |
| `/sensei:intake` | orchestrate the front door (questionnaire → confirm → run) | command | all of the above |
| always-the-entry | session-start guidance + nudge hook | guidance text + hook (OFF) | playbook_run read |

## Testing

- **Resolver (pure):** one assertion per seed rule; priority tie-break; wildcard match; no-match → `gsd` default + flagged.
- **DDL:** applies via `dbd` (enums + 4 tables + indexes); seeds present (6 playbooks, 6 rules, frame + 3 axis guides).
- **pg_store:** rules/playbooks/intake_guide list; `playbook_run` insert→read round-trips axes + rule_id + playbook; playbook + rule_id FK integrity.
- **classify_chunk:** fixture dialogues/contexts → expected axes; heuristic fallback when gateway is unavailable.
- **MCP `recommend_playbook`:** tool round-trip returns expected playbook + rationale.
- **`/sensei:intake`:** the flow (guide → classify → recommend → confirm) produces a persisted, confirmed `playbook_run` and surfaces the opening tone.

## Scope / deferred

**In:** axis enums; `playbooks` (+6 seed, named routes with opening-tone stubs); `playbook_rules`
(+6 seed); `intake_guide` (+frame +3 axes); `playbook_run`; the pure resolver + `recommend_playbook`
MCP tool; `classify_chunk`; `/sensei:intake` command with the LLM questionnaire +
recommend-and-confirm; always-the-entry guidance + a nudge hook (OFF by default).

**Out (own follow-ups):** the 6 playbooks as full executable methods (catalog specs, via `method_ref`);
the **Sensei app** schema-**form** renderer of `intake_guide` (same data, second Sensei surface);
auto-select-on-trust; §9 outcome population + `learned`-row generation; hook *activation*; the
**Dōjō control-plane** authoring/governance of org-level rules/playbooks/guide overrides
(`source='org'`); per-rule tone override; runtime axis-extension (lookup tables).

## Resolved (post-review 2026-07-19)

- **`session_id` = a real FK to the existing session table.** Beyond attribution, this lets the
  classifier/recommender **draw on the session's conversation history** to infer the axes, and lets
  §9 correlate outcomes back to the run. (Exact table/PK pinned in Plan 1.)
- **Split into two implementation plans** (below).
- **Renderers are Sensei, control is Dōjō** (plane correction): the questionnaire (CLI) and the later
  form (Sensei `app/`) are per-user Sensei surfaces; Dōjō governs the shared content.

## Implementation plans (split)

**Plan 1 — Data + recommender core.** DDL (axis enums + `playbooks` + `playbook_rules` +
`intake_guide` + `playbook_run` + seeds) + pg_store CRUD (`list_playbooks`/`list_rules`/
`list_intake_guide`/`insert_playbook_run`) + the pure `recommend_playbook` resolver + the
`recommend_playbook` MCP tool + `classify_chunk` (LLM + heuristic fallback, session-history aware).
Fully testable without the command. `playbook_run.session_id` FK confirmed here.

**Plan 2 — Intake command + always-the-entry** (depends on Plan 1). `/sensei:intake`: load
`intake_guide` → run the LLM questionnaire (frame + per-axis) → `classify_chunk` → `recommend_playbook`
→ recommend-and-confirm → persist `playbook_run` → surface the chosen playbook's `opening_tone`.
Plus session-start guidance naming intake as the entry, and the nudge hook (built OFF by default).
