# Sensei mockups — outstanding fixes (for the designer)

> Covers the 12 `Sensei/lib/dojo/*.jsx` console screens **and** the `Sensei/site/`
> marketing website (`variant-a` / `variant-b`), judged against the Dōjō + end-to-end
> journey maps and `Sensei/CLAUDE.md`. **Lists only what still needs fixing.** Each
> item = issue + concrete fix; priority at the bottom.
>
> **Round 3 landed the behavior/logic pass** — §0 tokens sync, danger-routing (Retract,
> Supersede, most `declined`), fabricated provenance (now from `SCOPE_OWNERS`), the
> wired stubs (Save-revision, Preview-recipients/onboarding, stance-dial, Relay Stall
> ack), governance override-marking + Stack inheritance, `dojo-identity` nav route, and
> the §7 primitive consolidation (`DojoPanel`/`DojoBtn`/`DojoLive`/`DojoKindTag`). Those
> are removed below. **The design-system migration was not attempted and is now the
> dominant gap.**
>
> *(Kept at `docs/mockups/` — outside `Sensei/` — so it survives replacing the whole
> `Sensei/` mockup folder.)*

---

# A · Dōjō console screens

## A1 · Design-system migration (S1 · S4) — highest leverage; dark mode still broken
- **S1 · still ~0% adoption across all 12 files.** Every screen — incl. the new shared
  kit — is hand-rolled inline `style` over **deprecated numbered tokens** (`--paper-2/-3`,
  `--ink-2/-3/-4`, `--edge`, `--hairline`). Migrate to semantic utilities (`bg-paper-soft`,
  `text-ink-mute`, `border-paper-edge`) + `zs-*` components (`zs-btn`, `zs-card`, `zs-input`,
  `zs-badge`), per `lib/assistant-card.jsx`. `DojoBtn`'s `text-sm`/`text-base` is the lone
  semantic-type usage — propagate that pattern. **Until this lands, `[data-theme="dark"]`
  produces unreadable screens.**
- **S4 · off-scale type** everywhere (`9 / 10.5 / 11.5 / 12.5 / 13.5 / 14.5 / 23 / 26 / 27 /
  34 / 38 / 42 / 44`) — baked into the shared primitives too. Snap to the scale
  (11/13/15/17/22/28/40) or named type classes.
- **Residual raw literals** (the amber `oklch` in admin/maintainer is *gone* — good): only
  cosmetic `color-mix(in oklch…)` left in **shared `DojoKindTag`**, **relay** (needs-you
  border, decision-radio), **billing** (enterprise dark tier ×2), and `rgba()` shadows in
  **extensions** + governance. Most need a `-soft`/`-edge` token pair to migrate cleanly —
  add the tokens, then delete the inline literals.

## A2 · Responsive — the weakest area; still mostly unthreaded
Threaded + stacks correctly: admin **Overview**, maintainer **Triage/Candidate**, saas
**SignIn**, developer **Teams**. Everything else takes **no `mobile` prop** and leans on
`auto-fit`/`flexWrap` (fluid reflow) or horizontal scroll instead of the stack contract:
- admin **Monitor** + **Scopes**, **all of governance** (zero `mobile` refs), saas
  **Orgs/OrgsEmpty/Create**, **dojo-identity** (git-role-map row `1fr auto 1fr`), inapp
  **Redact/Share** (Share is a fixed 5-col grid), **billing** (tiers/roster), **extensions**
  (toolbar + 360px min cards), developer **contributions/downstream**.
- lead **6-col audit ledger** is the worst — `minWidth:760 + overflowX:auto` (**horizontal
  scroll**, the exact anti-pattern). Card-per-row on mobile.
Thread `mobile` from each console into its child screens (not just the shell) and stack.

## A3 · Danger-vs-warning — one miss left
- maintainer **Decline** button is still neutral grey (`border: hairline`, `--ink-2`), sitting
  identical to Revise. Route it to the **`danger`** family (`DojoBtn variant="danger"` exists).
  (Retract, Supersede, inapp/developer `declined` are done; relay Stall correctly stays amber.)

## A4 · Unwired stubs — two left
- admin **"Retract downstream"** has no `onClick` / confirm / preview — dead button on a
  destructive action. Wire the confirm+preview (match the maintainer preview pattern).
- admin **precedence-ladder "which rule wins" verdict** is a *static hardcoded panel*
  ("Client anonymization wins") beside the draggable ladder — it never reads the ladder
  order. Couple the verdict to the actual rung order.

## A5 · Terminology · logic residue
- **"source dropped"** synonym still coexists with "anonymized" — developer (row note) and
  inapp (body copy L206/263). Retire to **"anonymize"** everywhere.
- **`InappShare` "Share N" count** is now *derived* but the checkboxes are **non-interactive**
  (no `useState`, no `onClick`) — clicking a row can't change the selection or the count.
  Make the rows real toggles.
- **extensions "Scope ▾" control** shows a generic label; it doesn't reflect the card's
  current scope. Surface the active scope on the control.

## A6 · Primitive residue (S6)
- relay keeps its own local `MOBILE_TABS` array (renders through shared `DojoTabBar`, but the
  list is parallel); lead redefines a local `Panel` (L33–43) instead of shared `DojoPanel`
  (which `DojoAudit` already uses). Consolidate into `dojo-shared`.

## A7 · Copy bug
- **`dojo-inapp.jsx:291`** — garbled duplicated fragment: *"…recall a share until it's
  approved.**re until it's approved.**"* Copy-paste artifact; fix the sentence.

---

# B · Static website (`site/variant-a.jsx` · `variant-b.jsx`)

**Build on `variant-b`.** It carries the complete, correct positioning — local-first hero →
stats → how-it-works → surfaces → **Dōjō for teams** (contribute→triage→approve→distribute) →
**Relay** → **3-tier pricing** (free-personal / paid-team / enterprise). `variant-a` is missing
the Dōjō/Relay story entirely and its pricing **contradicts the product** ("free… no upgrade
prompt. Ever."). Keep variant-a's *restraint* as the tuning target (variant-b's `56`-everywhere
flattens hierarchy). A `variant-c.jsx` also exists — out of scope this pass; reconcile later.

- **B1 · Design-system pass (same rules as the app).** Literal `fontSize` throughout, inline
  type styling instead of `zs-hero`/`zs-h1`/`zs-h2`/`zs-body`, deprecated numbered tokens, and
  two raw color literals — `oklch(0.55 0.13 60)` (variant-a L65, ported `DojoForTeams`) and
  `rgba(20,18,14,0.4)` (variant-b L180 shadow). Snap to scale + named classes, semantic tokens,
  `zs-*` components; replace the `oklch` with `--warning`, model the shadow as a token.
- **B2 · Responsive — neither variant adapts.** Fixed `maxWidth`, `repeat(3,1fr)` / `1fr 1.4fr`
  grids, fixed-width mocks (`width={900}`), nav never collapses. Add breakpoint stacking (hero,
  stats 4→2→1, steps, gallery rails, Dōjō grids, pricing) + a mobile nav. Non-negotiable for a
  public site.
- **B3 · Hierarchy + dead code.** variant-b uses `fontSize:56` for hero **and** every section
  heading — step section heads down (reserve the top stop for the hero). `GalleryB` is fully
  defined but never rendered (`<Surfaces/>` is used instead) — delete it or wire it.
- **B4 · Nav + conversion.** Header links omit Pricing / Teams / Relay (the major sections) and
  there's no persistent Download CTA. Add the anchors + a nav CTA.
- **B5 · Missing dev-tool essentials.** No quickstart / install snippet (brew/curl), no
  docs link, no social-proof/trust strip.

---

# C · Site ↔ product accuracy (content vs. real features)

Compared the site copy against what the daemon + app + Dōjō actually do. **The privacy
over-claims (C1–C2) are the trust-critical ones — fix before publishing.** "Fix: Site" =
copy is wrong; "Decide" = the app doesn't do this yet, so either build it or mark it roadmap.

| # | Site claims | Reality | Fix |
|---|---|---|---|
| **C1** | "Never makes outbound network requests" · "0 external requests" · "logs nothing remotely" | Outbound is **core**: BYOK inference (Claude/GPT APIs), Dōjō share/pull, **Relay** live line, library-docs ingest (GitHub/website), update check | **Site** — qualify to *"local-first; no telemetry; nothing leaves without an explicit share."* Over-claiming privacy is the worst trust break. |
| **C2** | "all stored in a **SQLite** file under your home directory" | Store is **Postgres** under `~/.sensei` (59 refs). `rusqlite` only *reads Zed's* `threads.db` as a capture source | **Site** — say *"a local database in `~/.sensei`"* (don't name SQLite). |
| **C3** | "Settings → Export … JSON dump of every pattern, memory … Import is also supported" | **Not built** in the app. Only `dojo-mind` has a per-engagement *compliance* export (CSV/JSON) | **Site** — drop the claim now. Local export/import is on the **roadmap** (future phase — see [plan](../plan/README.md)). |
| **C4** | "Any AI assistant that speaks the Model Context Protocol" | Capture = **Claude Code** (hooks) + **Zed** (ACP / `threads.db`). Sensei *exposes* an MCP server ≠ observes any MCP client | **Site** — *"works with Claude Code and Zed today; more as adapters land."* |
| **C5** | Dōjō · Relay · global Collective · SSO/SCIM · 3-tier billing all shown as **available** | Dōjō web consoles **in progress**; **Relay design-only**; Collective / SSO / SCIM / billing **not built** | **Build (accelerate)** — decision is to *accelerate* these, not gate them. Until each lands, keep the site honest (don't present unbuilt features as shipped). |
| **C6** | variant-a pricing: "free … no upgrade prompt. **Ever.**" | Private **team/enterprise Dōjō is the paid tier** | **Site** — use variant-b's free-personal / paid-team model (variant-a contradicts it). |
| **C7** | "Free · **No account**" (hero/trust line) | Local app: true. **Dōjō requires GitHub sign-in** | **Site** — scope "no account" to the local app; the Dōjō signs in. |
| **C8** | Instruments — "watch toolset **health** over time" | Instruments-Health is **parked** (MCP registry ↔ usage don't join) | **Site/app** — soften the claim, or ship the join. |
| **C9** | *FTR / first-turn-resolution* appears **nowhere** | FTR is the product's **north-star metric** — the measured outcome of the loop | **Site** — add an FTR beat (a stat or Sessions caption). It's the "why it works" the site omits. |
| **C10** | Footer **v0.4.2** | `VERSION` = **0.3.6** | **Site** — wire the footer to `VERSION`. |

Consistent (no action): the 5 gallery surfaces (Today / Sessions / Insights / Memories /
Instruments) match the app; embedded-Ollama-Gemma-4 + BYOK models copy is accurate; the
Dōjō contribute→approve→distribute loop + artifact taxonomy match governance.

---

## Priority order
1. **C1 + C2 privacy/storage accuracy** — trust-critical, and the site is *live*. Fix first.
2. **C6 fix variant-a pricing** + keep the site honest on **C5** (unbuilt ≠ shipped) while the
   Dōjō/Relay build is **accelerated** (C5 decision). **C3** — drop the export claim now.
3. **A1 design-system migration** (dojo, §1) — unblocks dark mode; highest-leverage app item.
4. **B1 + B2** website design-system + responsive pass (build on variant-b).
5. **A2 responsive** (dojo) + **A3 danger** (maintainer Decline) + **A4 stubs** (admin Retract, ladder verdict).
6. **A5 terminology / InappShare / scope-control** + **A6 primitive residue** + **A7 copy bug**.
7. **C3 / C4 / C7 / C8 / C9 / C10** site copy refinements; **B3–B5** hierarchy/nav/quickstart.

*Updated 2026-07-16 (round 3) — resolved items removed; adds static-website review (B) and site↔product accuracy audit (C).*
