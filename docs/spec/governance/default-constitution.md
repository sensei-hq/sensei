# Default constitution & guardrails (the starter governance bundle)

> **What a fresh personal Dōjō ships with** instead of an empty rules file — so day
> one feels like month three. A curated, adopt-or-override starter set of
> **principles**, **guardrails**, and **guidelines** for building software with an AI
> assistant. Seeds `~/.sensei/rules.md` (and the marketplace defaults); every line is
> overridable per scope ([[pipeline/governance]]).
>
> **Provenance.** Distilled from the **DORA** research (the Four Keys + generative
> culture), the **XP / CD** technical practices, and the **Core Protocols**, framed
> for AI-assisted work per R. Kasperowski, *"Scrum Doesn't Matter (And It Never Did)"*
> (Global Scrum Gathering, 2026) — whose thesis is sensei's reason to exist: **AI
> amplifies your current capabilities** (strong fundamentals + AI → extraordinary;
> weak fundamentals + AI → more chaos), and the way you keep fundamentals strong is
> *measure → try a practice → measure again → keep what helps* — the loop sensei
> automates.

## Constitution — the durable principles (how we build here)

1. **Measure, then keep what helps.** Try a practice, measure its effect, keep it if
   it moves the number, drop it if it doesn't. No practice is sacred — the data decides.
2. **The right thing beats more things.** Better velocity is about *direction*, not
   raw speed. Ask of any change: *is this the code that does what the user needs?*
3. **Strong fundamentals first — AI amplifies whatever you already are.** Tests, small
   changes, and clear direction are what make AI a multiplier instead of a chaos engine.
4. **Make it safe to question the AI.** A generative culture surfaces the assistant's
   mistakes early; nobody rubber-stamps a model's output.

## Guardrails — default rules (`.sensei/rules.md` shape · one testable line each)

### Quality
- Every change ships with a test — tests are how AI hallucinations get caught before production.
- Never merge on red; a failing check blocks the merge.
- A human reviews AI-written code before it lands — no unreviewed model output on the main branch.

### Architecture
- Prefer the simplest design that passes the tests; delete before you add.
- Refactor relentlessly and continuously, not in a separate "cleanup" phase.
- Keep changes small and single-purpose so each is easy to review and revert.

### Process
- Integrate to trunk continuously; short-lived branches, frequent merges.
- Keep the change lead time short — commit → production in hours, not weeks.
- Hold a lightweight daily plan and a weekly review + retrospective (plain language, no ceremony jargon).
- Review with the **Perfection Game**: rate the work, then ask *"what would make this a 10?"*

### Tools
- Keep the pipeline green and fast; a broken or slow pipeline is a stop-the-line event.
- Automate the deploy so shipping is boring and on-demand.

### Patterns
- Match the house style already in the codebase over introducing a new idiom.
- Reuse a shared implementation before writing a fourth near-duplicate.

## Guidelines — practices worth trying (softer than rules)

- **Big-picture backlog** — track goals across time (last quarter → this month → next
  half), not a pile of tickets. State *why each goal matters*.
- **Plain English over jargon** — say what you mean; if a stakeholder wouldn't
  understand the word, don't use it. (sensei's own insight copy follows this.)
- **Sustainable pace** — the loop only compounds if it's still turning next month.

## The measurement framework — DORA, alongside FTR

sensei's north star is **FTR** (first-turn resolution) — the *leading*, session-level
signal of how well the human+AI pair works. The **DORA Four Keys** are the *lagging*,
delivery-level outcome; measured together they tell whether AI is amplifying or eroding
your fundamentals.

| Metric | What it measures | sensei signal |
|---|---|---|
| **Change lead time** | commit → production | code+activity graph + git history *(needs a deploy signal)* |
| **Deployment frequency** | how often you ship | release/tag/CI events *(needs deploy detection)* |
| **Change fail rate** | % of changes that cause a failure | revert / hotfix / correction-churn signals (sensei already derives these) |
| **Failed-deploy recovery time** | incident → restored | session + git timeline *(needs an incident marker)* |
| **Generative culture** *(5th marker)* | is it safe to question the AI? | "human corrected the assistant" = the pair caught a mistake (already captured) |

**The story:** correlate FTR + AI-pairing patterns against DORA movement — sensei is the
retrospective that proves your fundamentals stayed strong *while* you leaned on AI. Build
item: the [[plan]] DORA delivery module (its prerequisite is a deploy/release detector).

## Adoption

- Ships as the **default** personal-Dōjō bundle; a new project inherits it on bind.
- **Every line is overridable** — a more specific scope (org → team → project → stack)
  can strengthen, relax, or replace any rule; overrides are marked, not hidden.
- Rules are constraints (the AI must follow); the principles are the *why*; the
  guidelines are invitations. Keep each rule one line, actionable, testable.

## Related
- [[pipeline/governance]] — how rules resolve, inherit, and get promoted.
- [[requirements/vision]] — the FTR north star + why AI amplifies fundamentals.
- [[plan]] — the DORA delivery module + deploy-signal detector build items.
