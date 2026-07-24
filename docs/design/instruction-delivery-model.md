---
type: design
status: draft (pending review)
---

# Instruction Delivery Model — how instructions and capabilities reach and stick in a session

How governance **rules** (the constitution + scoped guardrails) and sensei
**capabilities** (skills / agents / mindsets) get into an assistant session and
*stay* shaping behaviour for the whole session. This unifies two things that are
designed and built separately today but are the same problem — *"how does an
instruction reach the model, and how does it keep mattering"* — and it names the
stickiness failure (**"loads once, then forgotten"**) as the thing to fix.

Related: [`governance.md`](governance.md) (how rules resolve + where the code
lives), [`assistants.md`](assistants.md) (ACP adapters, MCP delivery, hooks),
[`../architecture/concepts/governance.md`](../architecture/concepts/governance.md)
(the two-axis model), `marketplace/plugins/sensei/hooks/session-start` (the one
push channel today).

---

## 1. The problem in one paragraph

Two axes must reach the model: **rules** (constitution + scoped guardrails —
governance) and **capabilities** (skills / agents / mindsets — procedure). Both
have the same weakness: the parts that matter most are delivered **pull, not
push** (the model must choose to fetch them), or **push-once** at session start
(and then decay up-context / get summarised away at compaction). A `mandatory`
org rule is honoured in the data model and in `structure_ruleset`'s
mandatory-lock, but at the *session boundary* it is advisory-in-practice: nothing
puts it in front of the model unless the model elects to pull it. Skills are the
same by design — a skill body loads only when the model matches its description.
This doc picks a delivery model that makes the non-negotiable tier **push +
re-assert**, keeps situational capability **pull**, and defines what a rule and a
rule pack must carry to support it.

---

## 2. Delivery surfaces that exist today

Six surfaces carry instructions into a session. They differ on **push vs pull**,
**what content they carry**, and **whether they survive compaction**.

| # | Surface | Push or pull? | Content it carries | Survives compaction? | Mechanism |
|---|---|---|---|---|---|
| S1 | **`~/.sensei/rules.md`** (materialized file) | **push** (read by the SessionStart hook) | **only** `user` + `general` scope + genuinely-global rules | no (re-read only if hook re-runs) | daemon renders on boot (`render_rules_md`, `resolve_global_rules`); on-demand `POST /api/knowledge/rules/materialize` |
| S2 | **`get_rules` MCP tool** | **pull** (model must call it) | full per-repo resolution across ALL scopes (org/client/tech/team/project/repo) + mandatory flags | n/a (live each call) | MCP → `GET /api/knowledge/rules` → `resolve_rules_raw(folder_id)` → `structure_ruleset` |
| S3 | **`CLAUDE.md` / `AGENTS.md` pointer** | **push once** (upserted managed block at boot) | a one-line durable pointer to `~/.sensei/rules.md` | **yes** (static text in the repo/global file) | `upsert_pointer_in_claude_md` |
| S4 | **SessionStart hook** | **push** at session start | global rules head, project guardrails, a *lean* mindset/persona reminder, MCP tool decision-guide | no (does not re-run) | `hooks/session-start` → `additionalContext` |
| S5 | **PreCompact hook** | **push** on compaction | a *thinner* `<sensei-refocus>` block (rules head + mindset one-liner + a few tool bullets) | yes (fires *at* compaction) | `hooks/pre-compact` |
| S6 | **On-demand skill / agent load** | **pull** (model self-elects, or user invokes) | a skill body, or a subagent's procedure + tool-scope | no (loads per invocation, isolated context) | Claude Code reads only frontmatter into an index; body loads on description match / `/sensei:agent use <name>` |

Two facts to hold onto:

- **S1 is a slice, not the whole.** Only `user` + `general` reach the session
  automatically. Everything *scoped* — i.e. the actual governance value — arrives
  only via S2 (pull) or not at all.
- **S4 is the only always-on push channel.** Everything else is either pull
  (S2, S6), static-once (S3), or compaction-triggered (S5).

---

## 3. The stickiness problem and its root causes

"Not sticky" decomposes into three failure shapes, each with a root cause from
the maps:

### 3a. Pull-vs-push (the model must self-elect)
- **Scoped rules are pull-only.** S1 pushes only `user`+`general`; org / client /
  technology / team / project / repository rules require the model to call
  `get_rules` (S2). The SessionStart hook injects a *reminder* to call it and
  `render_rules_md` prints "call the `get_rules` MCP tool" — but nothing forces
  it. **Root cause:** delivery of the governance-bearing scopes is
  model-discretionary.
- **Skills/agents are pull by design.** A skill body loads only when the model
  matches its `description` to the task; if the model doesn't recognise the
  trigger, `zero-errors-policy` / `knowledge-capture` / `test-gen` never load.
  There is no mechanism that *forces* a skill in. **Root cause:** skills are the
  wrong carrier for anything non-negotiable — routing a constitution through a
  skill would require the model to elect to load its own governance.

### 3b. Load-once-forgotten (push decays up-context)
- The SessionStart block (S4) is a single message at position 0. As the
  conversation grows it slides up-context and the model's attention to it drops
  turn by turn. Nothing re-asserts it. The rules, mindset reminder, and tool
  guidance are stated **exactly once**. **Root cause:** a one-time injection has
  no re-assertion cadence.
- The **lean-pointer bet leaks.** SessionStart injects "apply Analyst → Developer
  → Acceptance Tester, run `/sensei:agent <name>`" instead of the full question
  sets; the substance only binds if the model *chooses* to run the subagent or
  *remembers* the one-liner as it scrolls away. **Root cause:** deferring
  substance to on-demand agents trades token budget for stickiness.

### 3c. On-demand-only + the compaction cliff
- On compaction the original SessionStart block can be summarised away. S5
  mitigates but re-injects a **thinner** block — global rules, personas, and the
  full tool decision-guide are dropped. **Root cause:** the refocus block is a
  summary, not the constitution.
- **Non-Claude ACPs never run the hook at all.** For those, only the static
  CLAUDE.md/AGENTS.md pointer (S3) survives — and a pointer still requires the
  model to go *read* the file (pull). **Root cause:** push is host-specific;
  the durable anchor is a pointer, not the content.
- **Isolated subagents don't persist insight back.** A mindset subagent returns
  only its report; the mindset it wore doesn't transfer to the main thread, which
  reverts to the lean pointer on the next turn.

### 3d. Cross-cutting root causes (from the maps)
- **The default constitution reaches nobody today.** `seed_default_governance()`
  has zero callers, the `dojo` schema is excluded from the daemon deploy, and no
  default global-dōjō `knowledge_source` is auto-registered — so a fresh install
  renders "_No global rules yet_". Even a perfect delivery model injects an empty
  set until the seed + pull-subscription path exists.
- **Mandatory ≠ enforced at the boundary.** The mandatory-lock only affects
  ordering/dedup *inside* a resolved set; it does not block a session or a tool
  call. Delivery is advisory even for `mandatory`.
- **Everything fails open.** `gate`, `nudge`, `forward`, and hook telemetry are
  fail-open; if the daemon is down, `get_rules`/`get_patterns` return nothing and
  the model proceeds ungoverned. Stickiness is contingent on daemon uptime.
- **Dual rules-file models coexist.** Repo-local `.sensei/rules.md` (hook +
  `init`) vs DB-materialized `~/.sensei/rules.md`. The per-repo file is legacy and
  should retire to a single materialized artifact + live MCP.

---

## 4. Options (with tradeoffs)

### Option A — Keep pull-first (status quo, tightened wording)
Leave scoped rules on S2 (`get_rules`), keep S1 for `user`+`general`, keep the
SessionStart *reminder* to pull. Improve only the prose nudges.

- **Pros:** zero per-turn token cost; freshest possible scoped resolution (live
  each call); no host-side push machinery to build.
- **Cons:** does not fix stickiness at all — the governance-bearing scopes remain
  model-discretionary; `mandatory` stays advisory-in-practice; compaction still
  drops everything but the pointer. This is the failure we are trying to leave.

### Option B — Push the resolved per-project ruleset at session start (single push)
Have the SessionStart hook resolve the CWD repo and inject the **full
`get_rules` output** (the per-repo `resolve_rules_raw` result, not just
`user`+`general`) into `additionalContext`. One push, richer content.

- **Pros:** the actual scoped constitution is in-context from turn 0 without the
  model electing to pull; small, well-understood change to one hook; reuses the
  existing resolution engine end-to-end.
- **Cons:** still **push-once** — decays up-context (3b) and is summarised away at
  compaction (3c) unless paired with re-assertion; still Claude-host-specific
  (non-Claude ACPs fall back to the S3 pointer); token cost grows with ruleset
  size (needs the advisory tier held back — see §5).

### Option C — Enforcement-tiered push + re-assert + durable anchor (recommended)
Split delivery **by enforcement tier** and give each tier the surface that matches
its authority, then add a re-assertion story:

- `mandatory` + `required` → **always pushed** (SessionStart injects the resolved
  per-project set; PreCompact re-injects the **full** mandatory/required tier, not
  a summary; the CLAUDE.md/AGENTS.md pointer is the durable anchor for hostless
  ACPs). Re-assert on a **drift signal** (not a fixed cadence) to counter
  up-context decay without token bloat.
- `advisory` + (optionally) `recommended` → **on-demand** via S2 (`get_rules`) and
  procedural skills — situational guidance the model pulls when relevant.
- **Skills/agents/mindsets stay the secondary procedural layer** on the pull path
  (they are situational by design); a stack or compliance **rule pack MAY ship a
  skill** for the procedure that satisfies a rule, but the rule's authority never
  routes through the skill.
- For the handful of truly non-negotiable, **tool-observable** mandates, the
  existing PreToolUse `gate` can enforce (deny the violating tool call) — the only
  surface that *enforces* rather than reminds.

- **Pros:** matches the two-axis data model exactly (enforcement decides the
  surface); the non-negotiable tier is push + re-assert + durable, so it survives
  the whole session including compaction; advisory stays cheap and pull;
  capabilities keep their situational nature; one enforcement gate covers the
  observable mandates.
- **Cons:** most moving parts (hook change + PreCompact upgrade + drift signal +
  optional gate activation); the gate is fail-open/unregistered today and a
  fail-closed mandatory gate risks false-denies and daemon-down hard-blocks;
  requires the seed/pull-subscription gap (3d) closed first or it injects an empty
  set.

---

## 5. Recommended model

**Primary = an enforcement-tiered RULESET, resolved per project and PUSHED at
session start; SKILLS as the secondary procedural layer; re-assertion for
stickiness.** (Option C.)

1. **Resolve per project, once, deterministically.** SessionStart resolves the
   CWD repo through the existing engine (`resolve_rules_raw(folder_id)` →
   `structure_ruleset`), producing the ordered set across the repo's member
   namespaces + always-on `general`/`user`, with the mandatory-lock applied.

2. **Push by tier.**
   - **`mandatory` + `required` → injected always** into `additionalContext` at
     session start. These are the constitution + hard guardrails; the model must
     never have to ask for them.
   - **`advisory` (and, by config, `recommended`) → on-demand** via `get_rules`
     (S2) — the model pulls situational guidance when a task calls for it. This is
     the token-budget release valve: the always-injected block stays small.

3. **Re-assert for stickiness.**
   - **PreCompact re-injects the FULL mandatory/required tier** (not a summary) so
     the constitution survives the compaction cliff.
   - **Durable anchor:** the CLAUDE.md/AGENTS.md managed pointer (S3) stays as the
     hostless-ACP + post-compaction backstop; it points at the resolved artifact.
   - **Drift-triggered re-inject** (via the existing hook telemetry path) re-emits
     the mandatory tier when a drift signal fires — not on a fixed cadence, to
     avoid instruction fatigue and token bloat.

4. **Skills stay secondary and procedural.** Skills/agents/mindsets remain
   pull/on-demand (situational by design). A **rule pack may carry a `skill`** —
   the procedure that *satisfies* the rule (e.g. a compliance pack ships a
   `pii-redaction` skill) — but the rule's authority is delivered by the ruleset
   push, never by the skill load.

5. **Enforce the observable mandates.** Activate the existing PreToolUse `gate`
   for the small set of `mandatory`, tool-observable rules (fail-closed only for
   that set; fail-open elsewhere) so a mandatory violation is denied, not merely
   un-reminded.

6. **Close the source gap first.** None of the above delivers anything until the
   default constitution is seeded and pullable: invoke `seed_default_governance`
   on the Worker/D1 side and auto-register the global-dōjō as a
   `knowledge_source` on install so `run_pull_loop` lands the baseline as
   `general`-scope memories. Retire the legacy per-repo `.sensei/rules.md` to a
   single materialized artifact + live MCP.

### Mapping onto the real schema

The model rides the schema that already exists — no new axis:

- **Enforcement tier = `sensei.enforcement` enum** (`advisory < recommended <
  required < mandatory`, ascending so `ORDER BY enforcement DESC` surfaces
  strongest first). This enum *is* the push/pull switch: `mandatory`/`required`
  push; `advisory`/`recommended` pull. The `mandatory` comment already names it
  "the non-overridable constitution tier."
- **Scope + precedence = `sensei.scopes.level`** (`general=0 … repository=70`) via
  `sensei.namespaces` (instances) and `sensei.folder_namespaces` (a repo is a
  *member of a set*, not a tree). Resolution orders by `level` desc within each
  enforcement tier; the mandatory-lock means a narrower scope can refine but never
  weaken a `mandatory` rule.
- **A rule IS a memory** — `sensei.memories.{namespace_id, enforcement, origin,
  source_id}`. No parallel rules table. Delivery reads the same rows the analyzer
  writes.
- **Published/federated rules = `dojo.shared_rules`** (`enforcement` column,
  `namespace_id → sensei.namespaces`, monotonic `seq` cursor). This is the
  registry the pull loop consumes; the seed baseline lands here and pulls down as
  `general`-scope memories. (Note the known `seq`-on-update divergence: republish
  /retract must advance `seq` via an in-DB trigger or a re-published rule won't
  re-surface in a puller's delta.)

---

## 6. What a RULE and a RULE PACK carry

To support tiered push + optional enforcement + optional procedure, a **rule**
carries (all but the last two already exist on `memories`):

| Field | Purpose | Schema today |
|---|---|---|
| **area** | the domain the rule governs (security, testing, style, compliance, …) — for grouping, drift-signal targeting, and pack membership | `memories.tags` (GIN-indexed) / `memories.category` |
| **scope** | *where* it applies + precedence | `namespace_id → namespaces(scope_key, level)` |
| **enforcement** | *authority* → decides push vs pull surface | `memories.enforcement` (`advisory…mandatory`) |
| **content / impact** | the rule text + "what breaks if you skip it" | `memories.content`, `memories.impact` |
| **origin / source** | provenance (learned/authored/promoted/federated/dojo) | `memories.origin`, `memories.source_id` |
| **checker?** *(new, optional)* | a machine check that makes the rule **enforceable at a PreToolUse gate** (a tool-name/argument predicate, or a command that must pass) — present ⇒ the gate can deny; absent ⇒ delivery-only | not yet — proposed |
| **skill?** *(new, optional)* | the procedural **skill** that *satisfies* the rule (loaded on-demand when the rule is relevant); the rule stays the authority | not yet — proposed |

A **rule pack** is a curated, versioned bundle of rules shipped as a unit (a stack
pack — "Rust", "SvelteKit" — or a compliance pack — "SOC2", "PII"). It carries:

- **identity + version** — name, semver, so adoption and updates are trackable.
- **area** — the pack's domain, for the catalog and for drift targeting.
- **scope binding** — the scope(s) it attaches to on adoption (a stack pack binds
  at `technology`; a compliance pack typically at `organization`/`client`).
- **default enforcement per rule** — each rule's tier, so adopting the pack
  populates push-vs-pull correctly (adopter may raise, but never lower below the
  pack's `mandatory` floor).
- **checker(s)?** — optional machine checks for the pack's enforceable rules.
- **skill?** — optional: a stack or compliance pack **may ship one skill** (the
  procedure), registered on adoption; the pack's rules remain the authority.

This is the missing author/catalog side of the schema: the resolve/deliver side is
built, but there is no `rule_packs` table or `/v1` catalog today — adopting a pack
is the natural way an org populates its `mandatory`/`required` tiers.

---

## 7. Open decisions (Jerry only)

1. **Enable a fail-closed mandatory gate?** Activate the existing PreToolUse
   `gate` for `mandatory`, tool-observable rules — fail-closed for that set
   (daemon-down or checker-fail ⇒ deny). Trade: real enforcement vs false-denies
   and a hard-block when the daemon is down. (Everything else stays fail-open.)

2. **Which enforcement tiers push vs pull?** Recommended: `mandatory` + `required`
   push; `advisory` + `recommended` pull. Decide whether `recommended` should
   also push (stronger stickiness, more tokens) or stay pull (leaner).

3. **Re-assertion trigger.** Drift-signal-triggered re-inject (recommended) vs a
   fixed every-N-tool-calls cadence vs PreCompact-only. Trade: stickiness vs
   token cost / instruction fatigue.

4. **Seed + auto-subscribe the default constitution?** Invoke
   `seed_default_governance` (Worker/D1 side) and auto-register the global-dōjō as
   a default `knowledge_source` on install. Without this, the whole model injects
   an empty set. Decide: opt-in vs on-by-default for a fresh install.

5. **Build the rule-pack author/catalog side?** Add a `dojo.rule_packs` table +
   `/v1` catalog + adopt/toggle mutation, and add `checker?` / `skill?` to the
   rule shape. Trade: enables org-authored governance and stack/compliance packs
   vs net-new schema + Worker routes (currently fixtures-only).

6. **Retire the legacy per-repo `.sensei/rules.md`?** Collapse to a single
   materialized artifact + live MCP (remove the hook read + `sensei init`
   creation), or keep the dual model. Trade: single source of truth vs a
   familiar in-repo file.
