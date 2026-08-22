# Insight acceptance → materialization (accept turns intent into a durable artifact)

Status: SPEC (design). Grounded in the live recommendation corpus across sensei,
rokkit, gateway, dbd/dbd-rs, torii.

## 1. Problem

Sensei generates rich, LLM-reasoned recommendations, but **accepting one
materializes almost nothing**. Today (verified):
- `inference.recommendations` accept = a status flip (`pending → accepted`) +
  an FTR-`MeasureVerdicts` enqueue. The one exception, `promote_pattern`, flips
  `detected_patterns.lifecycle='rule'` — still not a written rule.
- The rec's *intent* (`revise_rule`, `write_skill`, `create_agent`,
  `enrich_memory`, `cross_project`) is delegated to a human/ACP via the free-text
  `prompt` column. Nothing durable is created.

Yet the seed is already present on every rec: `prompt` (for `create_agent` it is a
complete agent system-prompt — e.g. dbd's *"You are an Architectural Review Agent
for dbd-rs… before any code is accepted…"*), `why`, `based_on` (pattern/trace
provenance), `action_detail`. **The gap is purely the write-on-accept step.**

The dōjō side already documents the exact boundary we're crossing:
`sensei.dojo_inbox` auto-lands `principle`/`pattern` → `sensei.memories`, but
**defers `skill`/`agent`/`prompt`/`guard` as "consent-sensitive install"**
(Apply = no-op-with-reason). This feature implements those deferred kinds for the
local recommendation plane.

## 2. Objective

One dispatcher — `materialize(recommendation, scope, enforcement)` — invoked when a
user Accepts, that converts a rec into the right durable artifact by `action_type`,
at a user-chosen **scope** and (for rules) **enforcement tier**, with a
**preview + consent** step for anything that writes a file, and full **provenance +
effect measurement** so the learning loop closes.

## 3. The materialization map (action_type → artifact)

| action_type | Materializes into | Where it's written | Consumed by a session via |
|---|---|---|---|
| `revise_rule`, `promote_pattern` | governance **rule / principle** | `sensei.memories` at the target namespace + `enforcement` (or a `rule_pack_rules`+adoption when shareable) | `GET /api/knowledge/rules` + SessionStart/PreCompact hook; global set → `~/.sensei/rules.md` |
| `write_skill` | repo/project **skill** | `<repo>/.claude/skills/<slug>/SKILL.md` (or `sensei.library_skills` when tied to a library) | Claude Code auto-discovery |
| `create_agent` | project **agent** | `<repo>/.claude/agents/<slug>.md` (the rec's `prompt` seeds the system prompt) | Claude Code auto-discovery / auto-dispatch |
| `enrich_memory` | **memory** | `sensei.memories` (`origin='learned'→active`) | governance resolution (as today's `accept_proposal`) |
| `cross_project` | **shared rule pack** | new `rule_pack_rules` + `rule_pack_adoptions` across the sibling repos | `resolve_local_pack_raws` in the rules resolver |
| `archive_memory` | retire | `set_memory_status(archived)` | — |
| `audit_stale` | a review **nudge/task** (not a durable artifact) | task queue / pending-nudge | — |
| `library_update` | dependency bump | existing F v1/v2 maintenance automation | — |

## 4. Scope + enforcement (the user's two choices on Accept)

Reuse the existing scope ladder (`sensei.scopes`: general 0 … repository 70) and
enforcement axis (`advisory | recommended | required | mandatory`; `mandatory` = the
non-overridable constitution tier). Accept surfaces a small decision:
- **Scope** — repository (this repo only) · project · technology (e.g. `rust`,
  `svelte`) · cross-project (a shared pack). Defaulted from the rec's `based_on`
  and how widely the finding recurs (see §6).
- **Enforcement** (rules only) — default `recommended`; the user may raise to
  `required`/`mandatory` or lower to `advisory`. A rule binds by pointing
  `sensei.memories.namespace_id` at the chosen scope's namespace (creating the
  namespace if absent).

Skills/agents scope by **filesystem location** (repo `.claude/`), so their "scope"
choice is really *which repo(s)* to write into (this repo, or each repo in a
cross-project set).

## 5. The Accept flow (consent-gated, preview-first)

Because skills/agents **write executable, tool-granting files** into a repo (the
dōjō "consent-sensitive" boundary), Accept is not a blind one-click for those:

1. **Propose** — user clicks Accept on a rec. The daemon renders a **preview** of
   the exact artifact (the rule text + scope + tier, or the full `SKILL.md` /
   agent `.md` with frontmatter) from the rec's `prompt`/`why`/`based_on`.
2. **Review + adjust** — user confirms/edits scope, enforcement, name, and body.
3. **Apply** — the materializer writes it: a `memories` insert (rules/memory), a
   `rule_pack_rules`+adoption (cross-project), or a file write into
   `<repo>/.claude/skills|agents/` (skill/agent, consent-confirmed).
4. **Record** — the rec flips to `accepted`, `acted_at` set, and a provenance link
   is stored (rec ↔ materialized memory id / file path / pack rule).

Rules/memories may auto-apply at `advisory`/`recommended` (they're just injected
text, reversible); **skill/agent file writes always require the explicit Apply
step** (never auto-written), honoring the existing consent note. Everything is
idempotent (re-accept updates in place) and reversible (files are git-tracked;
memories archivable; adoptions removable).

## 6. Cross-project promotion (grounded in the 5 repos)

The same findings recur widely — `write_skill` across 11 repos, `promote_pattern`
10, `create_agent` 9, `revise_rule` 8. dbd + dbd-rs + gateway all independently
surface *"formalize module boundaries / architectural-cohesion agent."* So Accept
offers a **promotion scope** driven by recurrence:
- A finding seen in one repo → default **repository/project** scope.
- A finding recurring across sibling repos (or a stated principle like dbd's
  *"why are you using regex"*, which is language-general) → offer **technology**
  scope (e.g. a `rust` rule) or a **cross-project rule pack** adopted by each repo.
The sharing primitive exists: 14 library `rule_packs` + `rule_pack_adoptions` +
the DEFINER `dojo.set_pack_adoption`. A cross-project accept authors one pack rule
and adopts it at each sibling repo's namespace.

## 7. Provenance + the learning loop (why this compounds)

Each materialized artifact links back to its rec + `reasoning_trace` (via
`based_on`), and the rec accept already enqueues `MeasureVerdicts` (before/after
FTR on the affected repo). So the system learns **whether a materialized
rule/skill/agent actually improved first-try-rate** — feeding the ranker's leverage
weights (`revise_rule` 1.0 … `audit_stale` 0.25) so future recs are prioritized by
what has demonstrably worked. Accept → materialize → measure → re-rank.

## 8. Schema touch-points (minimal, additive)

- `inference.recommendations`: add `materialized_ref jsonb` (what this accept
  produced: `{kind, memory_id|pack_rule_id|file_path, scope, enforcement}`) — the
  provenance the measurement + un-apply read.
- Reuse `sensei.memories` (+ `namespaces`), `rule_packs`/`rule_pack_rules`/
  `rule_pack_adoptions`, and file writers — **no new rule table** (there isn't one;
  rules are memories or pack rules).
- New: a `materialize_recommendation(rec_id, scope, enforcement, overrides)` daemon
  path + per-kind writers (rule → memory insert; skill/agent → file write; cross →
  pack author+adopt), and a preview endpoint. Skill/agent writers are net-new
  (the deferred kinds).

## 9. Phasing

- **P-A — rules** (lowest risk, no file writes): `revise_rule`/`promote_pattern`/
  `enrich_memory` accept → `sensei.memories` at chosen scope+tier; extend
  `promote_pattern` to also write the memory (today it only flips the pattern
  lifecycle). Preview + scope/tier picker. Reuses the live rules injection path.
- **P-B — skills/agents** (crosses the consent boundary): the preview + Apply
  file-writers for `<repo>/.claude/skills/<slug>/SKILL.md` and
  `.claude/agents/<slug>.md`, seeded from the rec `prompt`. Explicit consent, never
  auto. Git-tracked (reversible).
- **P-C — cross-project**: recurrence detection + author-one-pack-rule +
  adopt-at-each-sibling. The `technology`/cross-project scope option.
- **P-D — loop**: surface the before/after FTR verdict per accepted artifact
  (the measurement already runs) so the value of each materialization is visible.

## 10. Invariants

- Never write a skill/agent file without an explicit Apply (consent boundary).
- Never fabricate an artifact: materialize only from a rec's real `prompt`/`based_on`
  (a rec lacking a usable seed offers "edit before apply", never an empty artifact).
- Idempotent + reversible: re-accept updates in place; files are git-tracked;
  memories archivable; adoptions removable.
- Scope + enforcement are the user's explicit choice, defaulted from recurrence;
  a more-specific scope can refine but never weaken a `mandatory` rule.
- Provenance always recorded (rec ↔ artifact) so effect can be measured + undone.
