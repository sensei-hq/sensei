# Governance Track — PRD Depth Review

> **Status:** Assessment (2026-10-14). Answers one question: *are the governance
> track's PRD specs deep enough to build from?* Findings only — no spec edits here.
> **Track:** epic [#91](https://github.com/sensei-hq/sensei/issues/91) — *Dōjō
> governance track: admin site + policies + preferences + skills/agents* — plus the
> governance half of the insight plane.
> **Specs reviewed:** M13 · M9 · M10 · T1 · T2 · the live legacy docs they fold.

---

## 1 · The specs in scope

| Spec | Lines | Scenarios | Verdict |
|---|---:|---:|---|
| **M13** Governance & Rules | 170 | 120 | ✅ **Deep** — the strongest spec in the track |
| **M9** Memory & Knowledge | 154 | 150 | ✅ **Deep** |
| **M10** Patterns & Conventions | 146 | 111 | ✅ **Sufficient** |
| **M11** Traceability & Doc Drift | 135 | 106 | ✅ Sufficient (adjacent) |
| **M12** Libraries | 129 | 147 | ✅ Sufficient (adjacent) |
| **T1** Dōjō Core | 159 | 443 | ✅ **Deep** |
| **T2** Dōjō Consoles | 146 | 170 | 🟡 **Thin on authoring** — role routing is deep, the authoring surface is one line |

Supporting design depth (not PRDs, but the source the specs fold):
`design/governance.md` (138), `design/instruction-delivery-model.md` (319),
`spec/pipeline/governance.md` (317), `features/governance/feature.md` (87),
`features/05-governance.md` (137).

The per-spec template (Purpose · scope · surfaces · **data invariants** · **signals
with worked examples** · done/wrong gates · scenarios · gaps) is honoured throughout
— these are not thin stubs. **The gap is not quality; it is coverage.**

---

## 2 · What is deep (no action)

**M13** carries the whole shipped rule model — rule = memory + 3 columns, tier-1
deterministic resolution (gather → enforcement desc → scope desc → mandatory-lock →
dedup, cached 5 min), tier-2 LLM consolidation, the four enforcement agents with
their audit tables, compliance packs, the personal `~/.sensei/rules.md` hot-reload,
and the solution cascade. Its signals table has worked examples and its wrong-gate
list is specific (priority inversion, mandatory relaxation, provenance break).
Scenarios are pinned to real test names.

**M9 / M10** mirror that depth for memory promotion and pattern sources.

**T1** is the deepest team spec: routing rules, the election-vs-mandate two-row
invariant, `NULLS NOT DISTINCT` idempotency, universal-strip anonymization, the
three-step resolution order (confidentiality → mandatory-lock → specificity), and
443 lines of scenarios.

---

## 3 · Where depth is missing

### 3.1 The Dōjō **authoring** surface has no module spec (the main gap)

Epic #91 is *"admin site + policies + preferences + skills/agents"*. The authoring
surface is fully designed in the mockup — `dojo-governance.jsx` — and stated in the
legacy dossier — `features/05-governance.md` §2 *"Scope-scoped authoring of rules /
skills / agents / commands + memory (mock only) … [ ]"*. But it appears in the PRDs
only as:

- **T2**, one in-scope line: *"Manage governance at org level: author rules/stance
  on the ladder"*; and one Signals row.
- **M13** covers authoring *rules*; **not** skills, agents, commands, or per-project
  memory as authored, shared, cascading artifacts.

Missing requirements (all in the mockup):

| Mockup element | Where it lives today | Spec coverage |
|---|---|---|
| **Scope ladder** for authoring (Personal · Company · Client · Team · Project · Stack; a free personal ladder + a paid company ladder) | `dojo-governance.jsx` `GV_SCOPES` | none |
| **Stance dials** — autonomy · sharing · review · anonymization, each a 3-stop choice with stated consequences | `STANCE` | M13 mentions "stance + autonomy gate" only |
| **Authored artifacts** — rules/guards/principles, skills, agents, commands, project memory — defined *here* vs *inherited* | `GV_DATA` + `GvSection` | M13 (rules only) |
| **Override / shadow display** — a local rule that shadows an inherited one is shown struck-through, not hidden | `GvRow` `_overrides` / `_overridden` | none |
| **Onboarding preview** — "what a new developer inherits", composed down the ladder by specificity | `showOnboard` block | none in PRD (T1 mentions inherit-on-bind) |
| **Constitution library** — "don't start from a blank page", browse/pull packs | `Browse library →` / `Add from library` | D-RULEPACK / D-SEED (decisions), not a module requirement |
| **Playbook-learning proposals** — accept a re-weighting → becomes a rule at this scope | `PlaybookReview` | M13 lists `accept_playbook_rule` / `list_playbook_rule_proposals` as MCP surfaces, not requirements |

**Recommendation:** a module spec (or a T2 subsection) for **Dōjō Governance
Authoring** with the scope ladder, the four stance axes, the five artifact kinds,
inherit/override rendering, the onboarding preview, and library browse. This is the
single largest depth gap in the track.

### 3.2 The **stance** model is under-specified

The mockup's stance is four axes × three stops, each with a consequence line:

| Axis | Stops | Consequence (low → high) |
|---|---|---|
| Autonomy | Gate often · Balanced · Run free | every risky step gates → only flagged commands gate |
| Sharing | Manual · Opt-in · Automatic | nothing leaves → past the bar leaves on a cadence |
| Review | 1 approval · 2 approvals · Consensus | one maintainer → all scope owners |
| Anonymization | (mock shows "always on" for client scope) | client work is never un-anonymized |

M13's feature file already pins the behaviour (`most_specific_scoped_stance_beats_default_and_lower_rungs`,
`strict_fails_closed_on_every_dial`, `composes_effective_rules_in_daemon_scope_vocabulary_strongest_first`),
but **the README never states the axes, the stop semantics, or the resolution
rule**. A builder reading only M13 cannot implement stance.

**Recommendation:** add a "Stance" block to M13 (or the new authoring spec) with the
four axes, the 3-stop semantics, "most specific wins", and fail-closed strict.

### 3.3 Instruction **stickiness** is designed, not specified

`design/instruction-delivery-model.md` (319 lines) is the deepest thinking in the
track — push-vs-pull, the compaction cliff, and the recommended
**enforcement-tiered push + re-assert + durable anchor** model. In the PRDs it
surfaces only as M13's Open gap:

> *"Post-compaction re-grounding is 🟡 … tier-2 LLM consolidation and budget/stance
> controls remain designed-not-built."*

So the hardening model is a design doc, not a requirement. That is defensible while
unbuilt, but it means **no module owns "rules stay sticky after compaction"** — the
exact thing M13's Purpose leads with (*"re-grounded after context compaction"*).

**Recommendation:** lift Option C into M13's in-scope (or a dedicated
`instruction-delivery` requirement section), with the re-assert trigger and durable
anchor as testable claims.

### 3.4 Cross-module resolution order is stated two ways

- **T1** (`Data invariants`): ① confidentiality first → ② mandatory locks → ③
  specificity.
- **M13** (`Data invariants`): priority ladder `P0 > P1 > P2 > P3`, then impact, then
  tighter scope → and *"mandatory rules cannot be overridden by a tighter scope."*

These are the same engine described from two angles, but **neither says how
confidentiality interacts with priority** (does confidentiality beat a P0? is
anonymization a rule at all?). The mockup's client scope shows *"Client
anonymization · always on"* as adjacent-to-but-not-part-of the dial set.

**Recommendation:** one paragraph, in one place, stating the single resolution
order across confidentiality · enforcement · priority · scope; link it from both
M13 and T1.

### 3.5 The Dōjō **sync** gap is a correctness risk under a governance spec

`F-3` — *"Dōjō sync never worked"* (P1) — sits under T1. If rules resolve locally and
the Dōjō fold-in is broken, the "shared" half of governance silently no-ops. T1's
Purpose claims the resolved constitution "just works"; the wrong-gate list does not
currently include *"the Dōjō's rules never actually reach `get_rules`"*.

**Recommendation:** add that wrong gate to T1 and a scenario that a Dōjō-authored
rule appears in the member's resolved set (today it cannot be pinned).

### 3.6 A shipped-but-unreachable pack

M13's Open gaps already records it: the ponytail pack is **seeded but not adopted**,
and *"nothing surfaces the pack for adoption yet, so opt-in means 'unreachable' in
practice."* That is a real product hole in the governance track (the pack exists, no
UI reaches it). The mockup's "Browse library →" is exactly the missing surface.

**Recommendation:** fold this into the authoring spec (§3.1) as the library entry
point, and file it as a gap row if it isn't one already.

---

## 4 · Depth verdict, per the original question

| Question | Answer |
|---|---|
| Is M13 deep enough for the **solo/rule-model** half? | **Yes.** Deep, specific, test-pinned. Its only content gap is stance + stickiness (§3.2–3.3). |
| Are M9 / M10 deep enough? | **Yes.** |
| Are T1 / T2 deep enough for **membership + routing**? | **T1 yes; T2 yes for role routing.** |
| Is the **Dōjō governance authoring surface** (epic #91) specced? | **No.** It is mock + one T2 line. Scope ladder, stance dials, the five artifact kinds, inherit/override display, onboarding preview and the constitution library are unowned (§3.1). |
| One-line verdict | **The rule *engine* is well specified; the *authoring surface* is not.** Add one authoring module spec and three small insertions (stance, stickiness, resolution order), and the track is complete. |

---

## 5 · Recommended additions (smallest set)

1. **New module spec — Dōjō Governance Authoring** (T2 subsection or its own
   `05-…`): scope ladder · stance · authored artifact kinds · inherit/override
   rendering · onboarding preview · constitution library. *(§3.1)*
2. **M13 — add a Stance block** and a Stickiness block, or link the authoring spec.
   *(§3.2–3.3)*
3. **One canonical resolution-order paragraph** (confidentiality · enforcement ·
   priority · scope), linked from M13 and T1. *(§3.4)*
4. **T1 — new wrong gate + scenario** for "a Dōjō-authored rule reaches the
   member's `get_rules`". *(§3.5)*
5. **Gap row** for the unreachable opt-in pack (ponytail) if not already tracked.
   *(§3.6)*

None of these require a decision on top of the existing ones (D-RULEPACK, D-SEED,
D-LOCAL-PACKS, D-PACK-KIND, D-INJECT already settle the mechanics) — they are
specification work, not design work.

---

## 6 · Related

- [`governance.md`](governance.md) — the governance module design (scopes, rules, identity, budgets).
- [`instruction-delivery-model.md`](instruction-delivery-model.md) — stickiness and the push model (§3.3).
- [`../spec/pipeline/governance.md`](../spec/pipeline/governance.md) — legacy pipeline depth (ladders, enforcement, solution cascade).
- [`../features/05-governance.md`](../features/05-governance.md) — the Dōjō plane dossier (folds into T1–T4).
- [`../prd/modules/02-insight/13-governance-and-rules/README.md`](../prd/modules/02-insight/13-governance-and-rules/README.md) · [`../prd/modules/04-team/02-dojo-consoles/README.md`](../prd/modules/04-team/02-dojo-consoles/README.md).
- [`../mockups/Sensei/lib/dojo/dojo-governance.jsx`](../mockups/Sensei/lib/dojo/dojo-governance.jsx) — the authoring surface (visual source, never edited).
