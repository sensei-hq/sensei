# Default constitution & guardrails (the base governance seed)

> **What every sensei ships knowing** — a curated baseline of **principles**,
> **guardrails**, and **guidelines** for building software with an AI assistant,
> seeded as **data in the Dōjō** (not a local file). It lives in the **global-dōjō**,
> distributes down to every install over the built Dōjō inbox, and reaches the
> assistant through the **`get_rules` MCP tool** — shared across the whole system,
> never a hand-edited document.
>
> **Provenance.** Distilled from the **DORA** research (the Four Keys + generative
> culture), the **XP / CD** technical practices, and the **Core Protocols**, framed
> for AI-assisted work per R. Kasperowski, *"Scrum Doesn't Matter (And It Never Did)"*
> (Global Scrum Gathering, 2026) — whose thesis is sensei's reason to exist: **AI
> amplifies your current capabilities** (strong fundamentals + AI → extraordinary;
> weak fundamentals + AI → more chaos), kept strong by *measure → try a practice →
> measure again → keep what helps* — the loop sensei automates.

## Where it lives & how it's delivered (DB-owned, not a file)

sensei **writes governance to the database, never to `~/.sensei/rules.md`.** That file,
when present, is a **materialized view** the daemon generates from the DB — read-only
output, not the source. The base bundle is therefore **seed data**, not a document:

1. **Seeded into the global-dōjō** — rows in `dojo.shared_rules` under the
   `org/global-dojo` tenant (the public collective everyone joins, already seeded by
   `dojo.seed_global_dojo`), via a new **idempotent `dojo.seed_default_governance()`**
   procedure (mirrors the `ON CONFLICT DO NOTHING` pattern; safe to run every deploy).
2. **Distributed down** to every install over the **already-built** Dōjō path
   (`pull_artifacts` → `collective/inbox` → `federation::run_pull_loop`), landing as
   global-scope `sensei.memories` carrying an `enforcement` level.
3. **Resolved + served** via `resolve_global_rules` → the **`get_rules` MCP tool** — the
   assistant reads the constitution through MCP, in-context, on every session.
4. **Overridable by scope, not by editing a file** — a more specific scope
   (org → team → project → stack) refines a rule through governance authoring (which
   re-materializes), **except** the `mandatory` tier (see below), which cannot be weakened.

**Enforcement tiers** map to the `sensei.enforcement` enum
(`advisory < recommended < required < mandatory`; `ORDER BY enforcement DESC` surfaces
the strongest). The enum's own comment names `mandatory` *"the non-overridable
constitution tier"* — so the three layers below are literally that column:

| Layer | `enforcement` | Behaviour |
|---|---|---|
| **Constitution** (principles) | `mandatory` | non-overridable; a narrower scope may refine wording but never weaken it |
| **Guardrails** (rules) | `required` / `recommended` | enforced defaults; a scope may relax with cause (marked, not hidden) |
| **Guidelines** (practices) | `advisory` | suggestions; freely adopted or dropped |

## Constitution — the principles *(`mandatory`)*

1. **Measure, then keep what helps.** Try a practice, measure its effect, keep it if it
   moves the number, drop it if it doesn't. No practice is sacred — the data decides.
2. **The right thing beats more things.** Better velocity is about *direction*, not raw
   speed. Ask of any change: *is this the code that does what the user needs?*
3. **Strong fundamentals first — AI amplifies whatever you already are.** Tests, small
   changes, and clear direction make AI a multiplier instead of a chaos engine.
4. **Make it safe to question the AI.** A generative culture surfaces the assistant's
   mistakes early; nobody rubber-stamps a model's output.

## Guardrails — the rules *(`required` unless noted)*

Grouped by the governance category the `rule_type` carries (Quality · Architecture ·
Process · Tools · Patterns); each is one testable line.

- **Quality** — every change ships with a test *(tests catch AI hallucinations before
  production)* · never merge on red · a human reviews AI-written code before it lands.
- **Architecture** — prefer the simplest design that passes the tests · refactor
  continuously, not in a separate phase · keep changes small and single-purpose.
- **Process** — integrate to trunk continuously · keep change lead time short
  (commit → production in hours) · daily plan + weekly review/retro in plain language
  · review with the **Perfection Game** ("what would make this a 10?") *(`recommended`)*.
- **Tools** — keep the pipeline green and fast (a broken pipeline stops the line) ·
  automate the deploy so shipping is boring and on-demand.
- **Patterns** — match the house style over a new idiom · reuse before a 4th
  near-duplicate *(`recommended`)*.

## Guidelines — practices to try *(`advisory`)*

- **Big-picture backlog** — goals across time (last quarter → this month → next half),
  each with a *why it matters*, not a pile of tickets.
- **Plain English over jargon** — say what you mean (sensei's own insight copy follows this).
- **Sustainable pace** — the loop only compounds if it's still turning next month.

## The measurement framework — DORA, alongside FTR

sensei's north star is **FTR** (first-turn resolution) — the *leading*, session-level
signal of how well the human+AI pair works. The **DORA Four Keys** are the *lagging*,
delivery-level outcome; together they say whether AI is amplifying or eroding fundamentals.

| Metric | What it measures | sensei signal |
|---|---|---|
| **Change lead time** | commit → production | code+activity graph + git history *(needs a deploy signal)* |
| **Deployment frequency** | how often you ship | release/tag/CI events *(needs deploy detection)* |
| **Change fail rate** | % of changes that fail | revert / hotfix / correction-churn (already derived) |
| **Failed-deploy recovery time** | incident → restored | session + git timeline *(needs an incident marker)* |
| **Generative culture** *(5th)* | is it safe to question the AI? | "human corrected the assistant" (already captured) |

**The story:** correlate FTR + AI-pairing patterns against DORA movement — sensei proves
your fundamentals stayed strong *while* you leaned on AI. Build item: the [[plan]] DORA
delivery module (prerequisite: a deploy/release-signal detector).

## Build notes

- **New:** `dojo.seed_default_governance()` procedure + the seed rows (this bundle) under
  `org/global-dojo`. Mirror `seed_global_dojo`'s idempotency; content-hash keyed so a
  reworded rule is a new version, not a duplicate.
- **Reuses (already built):** the Dōjō down/inbox distribution, `resolve_global_rules`,
  the `get_rules` MCP tool, the `enforcement` tiers, `dojo.shared_rules`.
- **No local-file authoring:** never write these to `.sensei/rules.md`; the daemon
  materializes that from the DB if at all.

## Related
- [[pipeline/governance]] — how rules resolve, inherit, promote, and consolidate.
- [[vision]] — the FTR north star + why AI amplifies fundamentals.
- [[plan]] — the DORA delivery module + deploy-signal detector.
