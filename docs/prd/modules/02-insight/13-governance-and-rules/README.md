# Governance & Rules · M13

**Plane:** insight · **Status:** [roadmap/02-insight.md](../../../../roadmap/02-insight.md) ·
**Serves:** solo developer ([persona 1](../../../03-personas.md#1--solo-developer--the-wedge)),
team developer ([persona 2](../../../03-personas.md#2--team-developer--the-inheritor))

## Purpose

Governance is how the pair (developer + assistants) agrees on the **rules of
engagement**. Rules are not a flat list: each concern (security, architecture,
testing, style) is its own **priority ladder**; when two rules conflict the higher
rung wins regardless of scope, with scope riding along as a tag — not the structure.
The assistant loads the resolved rules at session start and is *re-grounded* after
context compaction, so guidance sticks on long runs; the verifier blocks risky
actions that violate a P0 mandatory rule — no silent bypass. A solo user's rules come
from `~/.sensei/rules.md` plus what daemon learned (memories → rules); Dōjō-shared
rules are the Team plane on top of the same primitives. The user feels this module as
*the assistant stays honest to how we work here, even after the context got long*.

> **Status honesty (2026-10-14).** Two claims in this doc are aspirational: the
> `sensei.rule_violations` / `sensei.governance_overrides` audit tables **do not exist**
> — the only enforcement table that ships is `sensei.rule_check_runs` (checker runs),
> and there is no override store with `expires_at`. `sensei.rule_ladders` and
> `sensei.rules` never shipped either (the *rule = memory + `namespace_id` /
> `enforcement` / `origin`* model is correct). Paths: the handler is
> `api/handlers/knowledge.rs` (`get_rules`), resolution is
> `db/pg_store/governance.rs::resolve_rules_raw`, and `crates/senseid/src/governance.rs`
> is a **single file** (no `governance/` dir). See gap `G-GOV-AUDIT`.

## In scope / out of scope

- In: the rules model and resolution (priority ladders, tighter-scope tiebreak,
  mandatory-lock, and the **one canonical resolution order** —
  [design/governance.md §Resolution order](../../../../design/governance.md#resolution-order-the-one-order));
  the personal rules file (`~/.sensei/rules.md`, hot-reloaded);
  promotion paths into rules (from memories · patterns · chosen options · derived);
  the enforcement agents (checker · resolver · verifier · approver) with their
  audit trail (`sensei.rule_check_runs` ships today — the `rule_violations` /
  `governance_overrides` tables are **designed, not built**, gap `G-GOV-AUDIT`);
  compliance packs (HIPAA / PCI-DSS / SOC2 / GDPR / common baseline) as importable
  mandatory
  ladders; the default constitution as **bundled local packs** (seeded by dbd
  staging imports, not a procedure call — D-SEED); **stance + autonomy gate**
  (the dials are authored per scope — axes, stops, defaults and the strict posture
  live in [T2 §Stance](../../04-team/02-dojo-consoles/README.md#stance--three-dials-and-a-fixed-floor))
  and **tier-aware rule injection** — delivered and kept sticky per
  [Instruction delivery & stickiness](#instruction-delivery--stickiness); the
  solution-scope rule cascade.
- Out: the Dōjō share plane — triage, distribution, tenant consoles, identity/SSO,
  leak-guard, Collective commons (T1–T4, shares the same rule primitives up-stream);
  the MCP `get_rules`/`get_layered_context` *consumer* surfaces in the assistant
  (interface here, delivery in the Assistant-plane modules).

## Surfaces

- **MCP** — `get_rules(scope)` (resolved, ordered), `get_layered_context` (grounding
  assembly), `context_pack`, `run_checkers` (checker), `set_stance`, `promote_memory` /
  `accept_playbook_rule` / `recommend_playbook` / `list_playbook_rule_proposals`
  (promotion path). The `governance.check` · `governance.effective_rules` ·
  `governance.verify` · `governance.record_override` namespace **does not exist** — a
  rule is (re)checked via `run_checkers`, and there is no verify/override tool
  (gap `G-GOV-AUDIT`).
- **API/daemon** — `crates/senseid/src/api/handlers/knowledge.rs` (`get_rules`,
  `materialize_global_rules`) plus `handlers/playbook.rs` / `handlers/stance.rs`,
  `crates/senseid/src/governance.rs` (Tier-1 deterministic resolution,
  `structure_ruleset`/`RawRule`/`ResolvedRule`),
  `crates/senseid/src/db/pg_store/governance.rs::resolve_rules_raw` (cached 5 min).
- **App** — Preferences → Rules (ladders per concern, mandatory/advisory flags,
  retire), Project Patterns promote surface, Insights violation cards.

## Data invariants

- **A rule is a memory with three added columns** (shipped model): `namespace_id`,
  `enforcement` (`advisory | recommended | required | mandatory`), `origin`
  (`learned | authored | promoted | federated | dojo` — the real `memories.origin`
  values). No parallel rules table. The older `sensei.rule_ladders` + `sensei.rules`
  pairing **never shipped** (no such DDL); the ladder concept lives on as the
  priority+impact ordering on the memory row.
- **Tier-1 resolution is deterministic** (gather → order by enforcement desc, scope
  level desc → mandatory-lock → dedup); cached 5 min; runs on every
  `get_layered_context` call. **Tier-2** LLM consolidation (cherry-pick merge) is on
  rule-change/explicit-refresh only, drafts need user approval.
- **Mandatory rules cannot be overridden by a tighter scope.** A violating
  tighter-scope rule surfaces as a violation candidate, flagged.
- **Personal file.** `~/.sensei/rules.md` parsed at boot into the store as
  `origin = 'authored'` memories; hot-reloaded by the M3 root-watcher within the
  debounce window. (`memory_scope` has no `user` rung today — user-level rules land as
  global / `namespace_id`-scoped rows; see `G-MEM-SCOPE`.)
- **Priority + scope, never scope alone.** Within a ladder: `P0 > P1 > P2 > P3`,
  then impact (`critical > … > low`); same priority+impact contradiction → tighter
  scope wins (`global | project | stack | task_type | module`) — no `user`/`org` rung
  exists in `memory_scope` today (same caveat as M9).
- **Enforcement.** Checker (on capture events, writes `sensei.rule_check_runs` with
  `verdict` + `output_tail` — the only enforcement table that ships today); Resolver
  (effective ruleset for the context); Verifier (**hard gate by default** — P0
  mandatory violation blocks, P1 warns, P2/P3 advisory); Approver (a
  `--no-verify`-class bypass requires a documented reason + approver identity for org
  rules — **no `governance_overrides` store / `expires_at` exists yet**, gap
  `G-GOV-AUDIT`).
- **Compliance packs** import as mandatory `P0` ladders; adopting a pack enables its
  checkers.
- **Sources of rule rows** are recorded in `origin` and the promotion provenance
  (`promoted:memory:{id}`, `promoted:pattern:{id}`, `chosen:option:{category}`,
  `derived`, `dojo:{org}:{artifact}`, `user_added`, `remote`).
- **Solution cascade** (multi-repo): global → org → solution → project → module,
  walk-merged by the resolver; solution rules may not be overridden by member
  projects when `mandatory`.
- **Stance is authored elsewhere, resolved here.** The dials (autonomy · sharing
  · review) and the non-authorable anonymization floor are specified in
  [T2 §Stance](../../04-team/02-dojo-consoles/README.md#stance--three-dials-and-a-fixed-floor);
  this module owns their **effect** — autonomy decides how far an agent goes
  before it gates to a human, the strict posture refuses every autonomous action
  unless explicitly permitted, and the stance resolves most-specific-first (a
  project's beats its team's, which beats the default). The rule model is not
  restated there, and the stance semantics are not restated here.

## Instruction delivery & stickiness

Specified from [design/instruction-delivery-model.md](../../../../design/instruction-delivery-model.md)
(**Option C**). M13's Purpose leads with "re-grounded after context compaction,
so guidance sticks on long runs" — this is that requirement, made testable.

- **Delivery splits by enforcement tier, and enforcement *is* the push/pull
  switch.**
  - `mandatory` + `required` → **pushed** into `additionalContext` at session
    start, so the model never has to ask for the constitution.
  - `advisory` → **pull**, live via `get_rules` — the token-budget release valve
    that keeps the always-injected block small. *(Whether `recommended` also
    pushes or stays pull is an open decision — design doc §7.2.)*
- **Re-assert beats the decay.** A single push slides up-context and is
  summarised away at compaction ("loads once, then forgotten"), so:
  - **PreCompact re-injects the full `mandatory`/`required` tier** — *not* the
    thinner `<sensei-refocus>` summary (S5).
  - **The managed `CLAUDE.md` / `AGENTS.md` pointer block is the durable anchor**
    — the backstop for hostless ACPs that never run a hook, and after compaction.
  - **Drift-triggered re-inject** (the *recommended* trigger — an open decision,
    design doc §7.3) re-emits the mandatory tier when a drift signal fires rather
    than on a fixed cadence, so stickiness doesn't cost instruction fatigue and
    token bloat.
- **Observable mandates are enforced, not merely reminded.** The existing
  PreToolUse `gate` denies the violating tool call for the small set of
  `mandatory` **and** tool-observable rules — fail-closed for that set only,
  fail-open everywhere else. This is the one surface that enforces rather than
  reminds. *(Enable or not is an open decision: design doc §7.1.)*
- **Skills stay the secondary, procedural layer.** Skills/agents/mindsets remain
  pull-by-design; a rule pack **may** ship the skill that *satisfies* a rule
  (e.g. a compliance pack's `pii-redaction`), but a rule's authority never routes
  through a skill load.
- **What carries the tier.** Delivery adds two **optional** fields to the rule
  shape (both proposed, not yet in schema): a `checker?` — a machine predicate
  that makes a rule enforceable at the gate (present ⇒ the gate can deny; absent
  ⇒ delivery-only) — and a `skill?` — the procedure that satisfies it. A rule
  pack carries them plus identity/version, `area`, its scope binding, and a
  **default enforcement per rule** (an adopter may raise a tier, never lower it
  below the pack's `mandatory` floor).
- **Known preconditions** (from the design doc): the default constitution must be
  seeded and pullable first, or the model injects an **empty** set; mandatory
  ≠ enforced at the boundary until the gate is activated; and everything else is
  fail-open, so stickiness is contingent on daemon uptime.

## Signals shown

| Element | Meaning | Worked example |
|---|---|---|
| `get_rules(project=X)` | effective merged list, ladder-ordered | 3 Security P0 · 1 Testing P1 · 2 Style advisory |
| Violation card | rule check against captured behaviour | "echo suppressed lint on auth.rs — P0 bypass flagged" |
| Verify result | allow/block with specific reason | block: "commit violates P0 'no secrets in commits'" |
| Override row *(not built)* | audit trail with expiry | "--no-verify accepted; expires tomorrow, approver: you" |
| Ladder panel | per-concern rules with mandatory flags | Preferences → Rules |
| Pushed tier | what is always in context from turn 0 (no `get_rules` call) | `4 mandatory · 2 required pushed at session start` |
| Re-assert | the tier re-injected at compaction | `full mandatory tier re-injected — not the refocus summary` |
| Drift re-inject | the mandatory tier re-emitted when drift fires | `drift at 12 untouched tool calls → mandatory tier re-emitted` |

## Done gate

- `~/.sensei/rules.md` parses into the store with correct ladder + priority + scope;
  editing it while the daemon runs hot-reloads within the debounce window.
- `get_rules(project=X)` returns the priority-ordered merged list; a P0 project rule
  never loses to a P2 project rule; mandatory rules survive tighter-scope conflicts
  (the conflicting tighter rule is flagged).
- A battle-tested memory promoted to a rule carries `origin = 'promoted'`; retiring the
  source memory archives/retires the rule.
- A fresh install resolves the default constitution **offline**: the three
  `default-*` packs (`default-principles` mandatory, `default-architecture`,
  `default-process`) auto-adopted at the global-dojo namespace, `stack-templates`
  seeded-but-not-adopted — all via the dbd staging imports
  (`import_rule_packs` / `import_rule_pack_rules` / `import_rule_pack_adoptions`),
  no procedure call (gap C-2, resolved by D-SEED: the zero-caller
  `dojo.seed_default_governance` was retired).
- A P0 mandatory violation blocks the action; a `--no-verify` bypass without an
  override row never ships silently; adopting the HIPAA/PCI/SOC2 pack enables its
  checkers.
- A `mandatory` global/project rule is in the session's context **from turn 0** without
  the model calling `get_rules`; an `advisory` rule is **not** pushed and arrives only
  on pull (there is **no org/user rung** in `memory_scope`).
- After compaction the **full** mandatory/required tier is re-injected — not just
  the thinner refocus summary — and a drift signal re-emits the mandatory tier
  without a fixed cadence.
- A mandatory, tool-observable rule **denies** the violating tool call at the gate
  (fail-closed for that set only; everything else stays fail-open).

```bash
mcp_call get_rules --project=sensei \
  | jq '.rules | group_by(.ladder) | map({ladder: .[0].ladder, ranked: [.[] | {priority, text}]})'
touch ~/.sensei/rules.md   # then within the debounce window:
psql -A -t -c "select max(modified_at) from sensei.memories
                where origin = 'authored'" -d sensei
```

## Wrong gate

- **`get_rules` returns insertion order, not priority** — resolver not applying the
  ladder.
- **A P2 project-scope rule beats a P0 project-scope rule** — priority inversion.
- **`~/.sensei/rules.md` edits require a restart** — hot-reload broken.
- **Promoted rule stays active after its source memory retires** — provenance link
  broken.
- **A project-scope rule leaks into another project's `get_rules`** — scope filter.
- **A P0 violation didn't block the action** — verifier not consulted, or the wrong
  scope's ruleset was read.
- **A bypass shipped without an override row or an expiry** — audit trail broken.
- **All promoted memories collapse into a default Style ladder** — the source memory
  didn't carry a ladder hint.
- **A `mandatory` rule must be pulled before it is seen** — the tier split
  regressed; the constitution is advisory-in-practice again.
- **After compaction only the refocus summary survives** — re-assert regressed.
- **The ruleset is re-injected on every tool call** — the drift trigger was
  replaced by a fixed cadence (instruction fatigue, token bloat).

## Scenarios

- [`scenarios/governance.feature`](scenarios/governance.feature) — resolution,
  personal file, promotion, enforcement, packs, and delivery/stickiness
  (`NO TEST` / `# gap: instruction-stickiness`).

## Open gaps

- C-2 (registered in [reorg §12](../../../../reorg.md)) — default governance
  bundle: `seed_default_governance()` had zero callers. **Resolved by
  [D-SEED](../../../../decisions.md) (SHIPPED):** the procedure is retired;
  the constitution ships as bundled local packs seeded by dbd staging imports
  (three `default-*` packs auto-adopted at global-dojo, `stack-templates` + the
  ponytail discipline opt-in per D-PACK-KIND). Follow-on: nothing surfaces the
  ponytail pack for adoption yet, so opt-in means "unreachable" in practice.
- Post-compaction re-grounding is 🟡 (governance feature status) — **now
  specified** ([Instruction delivery & stickiness](#instruction-delivery--stickiness),
  from [design/instruction-delivery-model.md](../../../../design/instruction-delivery-model.md)
  Option C) but **unbuilt**: PreCompact re-injects the thinner refocus block rather
  than the full mandatory tier, there is no drift-triggered re-inject, the gate is
  fail-open/unregistered, and the default constitution still isn't seeded/pullable
  on a fresh install. Tier-2 LLM consolidation and budget/stance controls remain
  designed-not-built ([design/governance.md](../../../../design/governance.md)
  §Budgets & controls, §Local-model inclusion).
- **Delivery-model open decisions** ([design/instruction-delivery-model.md](../../../../design/instruction-delivery-model.md)
  §7, Dev-only) — enable a fail-closed mandatory gate (7.1), which tiers push vs
  pull (7.2), and the re-assert trigger (7.3). Specified as requirements in
  [Instruction delivery & stickiness](#instruction-delivery--stickiness); the
  switches are not flipped.
- **`G-GOV-AUDIT`** — `sensei.rule_violations`, `sensei.governance_overrides` have no
  DDL; the only enforcement table is `rule_check_runs`. Verifier gate + override
  store are designed, not built.

## Related

- [spec/pipeline/governance](../../../../spec/pipeline/governance.md) — ladders,
  enforcement, solution cascade (legacy depth, folds here; see design doc for the
  shipped model).
- [design/governance](../../../../design/governance.md) — the shipped rule-model
  (rule = memory + 3 columns), tier-1/tier-2 resolution, the **one canonical
  resolution order**, the namespace model.
- [design/instruction-delivery-model.md](../../../../design/instruction-delivery-model.md)
  — how the resolved set reaches a session and *stays* (Option C: tiered push +
  re-assert + durable anchor), specified in [Instruction delivery &
  stickiness](#instruction-delivery--stickiness).
- [features/governance/feature.md](../../../../features/governance/feature.md) —
  grounding/stickiness story · [features/05-governance.md](../../../../features/05-governance.md) —
  the Dōjō plane (folds with T1–T4, not here).
- [spec/pipeline/memory](../../../../spec/pipeline/memory.md) — promotion path ·
  [spec/pipeline/patterns](../../../../spec/pipeline/patterns.md) — verifier
  conformance.
- [prd/02-objectives.md](../../../02-objectives.md) · [prd/01-vision.md §The core loop](../../../01-vision.md#the-core-loop).
- Crate: `crates/senseid/src/governance.rs`, `crates/senseid/src/db/pg_store/governance.rs`,
  `crates/senseid/src/api/handlers/knowledge.rs`, `crates/senseid/src/api/handlers/playbook.rs`,
  `crates/senseid/src/api/handlers/stance.rs`.