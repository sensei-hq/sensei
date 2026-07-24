---
title: Design journey — approaches tried, what worked, what didn't
date: 2026-07-24
status: living record (append as we learn)
purpose: keep us on target — never re-litigate a settled pivot; remember why we rejected what we rejected
---

# Design journey — what we tried, what stuck, what didn't

The crisp locked answers are in [`../decisions.md`](../decisions.md). **This is the *why*** —
the approaches we circled through and the evidence that settled each. Read this before
re-opening a "should we instead…" question.

## 1. Dōjō backend — Rust service → Worker `/v1`
- **Tried:** a standalone Rust `dojo-mind` (`sensei-dojo`) service as the dōjō backend, in
  parallel with a SvelteKit Worker `/v1`.
- **What didn't work:** two backends for one thing; the Rust service kept getting features after
  the Worker decision, and senseid only touched it via a **dev-dependency test** (not a runtime
  link) — a confusing legacy that blocked clarity.
- **What worked / kept:** **Worker `/v1` is the sole dōjō backend.** Ported rules + artifacts
  (incl. the k-anonymity promote) to the Worker, repointed senseid federation to the tenant path,
  **deleted `dojo-mind`**. `dojo-protocol` (shared wire types) kept.
- **On target:** all dōjō backend work goes to the Worker `/v1` over the existing `dojo.*` Supabase
  tables — never revive a Rust dōjō service.

## 2. Dōjō frontend IA — dojo1 → dojo2
- **Tried:** dojo1 = personal-first IA + the DJ1 fix (solo works with no membership; root cause was
  a fabricated-tenant fallback in `tenant.ts`, not a 403). Then dojo2 = "one wired app, every role,"
  kit-first.
- **What didn't work:** dojo1's screen-by-screen build was superseded within days; the first dojo2
  pass **dropped 8 org consoles** (triage/approvals/knowledge/engagements/incidents/client-audit/
  identity/health) by oversight.
- **What worked / kept:** **kit-first componentization** (build the ~30 K2 components once, compose
  every screen from them) + **work-first IA** (`/you` personal landing, `/org/[slug]` role-scoped) +
  the DJ1 root-cause + the `org-guard.ts` DRY guard. The dropped consoles were re-added.
- **On target:** compose from the dojo2 kit; role-scope org surfaces; never gate personal on
  membership (DJ1 is the contract).

## 3. Instruction / constitution delivery — the stickiness problem
- **Tried, in order:** (a) skills/agents as the carrier of standards; (b) `get_rules` MCP pull +
  the session-reminder; (c) a materialized `~/.sensei/rules.md`.
- **What didn't work:** **skills-as-primary** — skills are on-demand capabilities, so an agent can
  violate a rule *before* invoking the skill; they don't stick. **Pull-only `get_rules`** — depends
  on the agent remembering to call it; the reminder is a nudge, not a guarantee. SessionStart today
  injects only `user`+`general` and PreCompact drops the mandatory tier — so rules **reach nobody
  reliably**, and never reach a daemon-driven agent at all.
- **What worked / decided:** **primary = an enforcement-tiered RULESET, PUSHED** at SessionStart +
  re-injected at PreCompact + injected into the relay driver (D-INJECT). **Skills = secondary**,
  procedural layer (a stack/compliance pack may *also* ship a skill). Enforcement tier is the
  delivery discriminator: mandatory/required always injected; advisory on-demand.
- **On target:** governance is pushed, not pulled; skills carry "how-to," rules carry "always."

## 4. Rule + rule-pack shape
- **Tried:** flattened dojo2 `KitRulePack = {name, by, count, rules:string[]}` with placeholder
  sources ("by ACME", "Rust Guild").
- **What didn't work:** the flat shape can't express **aspect** or **scope** or **enforcement**;
  packs read as fake placeholders; no way to see a rule's detail.
- **What worked / decided:** **restore the designer's original rich model** (LIB_PACKS): a pack
  carries **area** (7-set: principles/architecture/security/compliance/tech-stack/design/process),
  **scope** on the ladder where **"organization"** replaces company/client (that split is the
  *viewer's* relationship, resolved per-membership), **enforcement** (advisory→mandatory — drives
  precedence *and* injection), a **real source** (Robert C. Martin, OWASP, PCI SSC, **Rokkit** for
  Zen-Sumi…), and `rules[]` each `{text, detail?, hard?, checker?, skill?}`. Backed by a new
  **`dojo.rule_packs` table + join** (D-RULEPACK). Pack row = at-a-glance summary that expands.
- **On target:** pack = curated bundle of `shared_rules` rows; area/scope/enforcement are
  first-class; sources are real.

## 5. Styling / design-system bugs — verification lesson
- **Tried & failed then fixed:** (a) **transparent buttons** — `@unocss/reset`'s
  `[type=button]{background:transparent}` tied `.bg-primary` on specificity and won by source
  order → every typed button rendered transparent (white-on-white). Fixed by moving the reset into
  a low `@layer`. (b) **blank icons** — dynamic `i-glyph:{name}` UnoCSS couldn't statically scan +
  Solar not installed. Fixed via rokkit `icons.overrides` → `i-solar:*` (bare-name shortcuts,
  auto-safelisted). (c) **adopted-pill dark mode** — dōjō lacks `*-edge` border tokens (still open).
- **What didn't work (process):** I twice claimed "it's visible/fixed" from **geometry/opacity or a
  clean build** without checking **computed color/mask** — Jerry caught both.
- **On target lesson:** verify UI by **computed style** (background color, `--un-icon` mask), not
  a11y geometry or a green build. Baked into `DESIGN-BRIEF.md` guardrail B1 (dark-mode token check).

## 6. Deploy pipeline — the CF dedup trap
- **Tried:** push to `develop`, then fast-forward `develop:main` (same SHA).
- **What didn't work:** Cloudflare Workers Builds **dedups by commit SHA** — it built the SHA as a
  *develop preview* and **skipped the main production build**, so prod stayed on the old bundle
  (~18 min of "why isn't it deploying"). Jerry spotted "only develop built."
- **What worked:** an **empty commit gives main a fresh SHA** → forces the production build.
- **On target:** to deploy a merge-equal-to-develop, push a distinct SHA to `main` (empty commit if
  needed); `/version` (baked from `package.json` at build) is the deploy-verification signal.
  Also: `make bump` now updates `dojo/package.json` (was stuck at 0.3.0 vs repo 0.6.0).

## 7. Supervision model — task list → relay-as-run
- **Tried:** the task list + `/workflows` for "am I stalled?" visibility.
- **What didn't work well:** too coarse for a long unattended run; Jerry shouldn't have to guess.
- **What worked / decided:** **relay carries the build run itself** — publish the plan as a relay
  run (phases→segments) via the sensei run MCP tools, daemon federates status → Dōjō, Jerry watches
  as `jerrythomas` + nudges; **no update in 5 min = stall** (Relay-first). Relay **STATUS** (safe)
  vs **DRIVE** (off).
- **On target:** build relay supervision **first** so the rest of the unattended build is watchable.

## 8. Scope framing — dojo-only → holistic 3-plane
- **Tried:** focusing the design brief + plan on Dōjō.
- **What didn't work:** dojo-only misses the **impact surface** — Dōjō *configures/defines/triages/
  promotes*, but **Sensei (app+daemon) is where governance applies** and where impact shows; and
  Relay is the execution/supervision plane. Designing one plane in isolation creates integration
  gaps.
- **What worked / decided:** design the **whole loop** — Dōjō defines → Sensei applies → impact
  shows in Sensei → contributes up → Relay supervises throughout. `DESIGN-BRIEF.md` Part D is now
  three planes (Sensei impact surface `DS`, Dōjō, Relay `DR`).
- **On target:** every design/feature change names its plane and shows its counterpart on the others.

## 9. Local-model agents + coordinator
- **Tried (concept):** a superpowers-like coordinator that mixes local (gemma/qwen) + external
  subagents.
- **Decided (v1):** a **task-typed router** at `driver_for` that sends *cheap mechanical* tasks to a
  local model via `gateway.execute` single-shot, keeps reviewers/integration on Claude, adds an
  **attribution ledger**, with autonomous **drive OFF** (D-COORD). Local ACP coding-agent + the
  Planner→Builder→Judge loop are follow-ups, not v1.
- **On target:** extend `driver_for`, don't rebuild execution; local-first as an *extension*, gated
  behind the drive switch.

## Process meta — how we work now
- **Ultracode for big sweeps** (understand→synthesize→grill), sequential commit-per-chunk for
  mutating work; **never concurrent tree-mutating subagents** (a bg agent once clobbered uncommitted
  edits — use worktree isolation or commit-first).
- **Decisions land** in `docs/decisions.md`; the *why* lands here; nothing important stays verbal.
- **Unattended when clarity is full**, stopping only at the named human gates.
