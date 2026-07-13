# Website redesign review — screenshots → flows + Dōjō content

> **Type:** read-only review to inform a later build (the website update is queued at the *end*
> of the work queue, after the Hive→Dōjō consolidation). This is not the build. Nothing here
> edits the site.
> **Date:** 2026-07-13 · **Author:** review agent (for Jerry to cherry-pick + extend)
> **Scope (per Jerry):** the site changes are — *replace the app screenshots with FLOWS +
> screen-agnostic content, and add DŌJŌ-related content.*

## TL;DR

The redesign is almost entirely about the **product page** (`/sensei`), not the studio hub (`/`).
The hub already tracks its mockup (`hq/site.jsx`) closely. The product page still leans on
hand-built **app-replica screenshots** (the `Mock*` components) that go stale the moment the UI
moves. The mockup direction replaces them with two things:

1. **`site/surfaces.jsx` (`<Surfaces/>`)** — the five surfaces described as *goal + flow + "why
   this shape,"* not pictures. Explicitly authored as "editorial, not screenshots… screens and
   mocks go stale the moment the UI moves."
2. **`site/variant-a.jsx` → `DojoForTeams()`** — a new **"For teams · 結 Dōjō"** section: the
   contribute→distribute loop, the six artifact types, membership/routing, and a
   Collective-vs-Dōjō comparison table.

Plus a smaller shift: the **hero centerpiece** stops being a `MockToday` screenshot and becomes
`HeroBrief` — a chrome-less "example morning" showing the *artifact Sensei produces* (one focal
teaching + three dimmed secondary signals), which is durable across UI churn.

The direction is strong and on-brand. My highest-value additions: (a) reconcile the absolute
local-first promise ("0 external requests / nothing leaves your machine") with the networked,
opt-in Dōjō so the two messages don't contradict; (b) tighten Dōjō copy to what is actually
built (console + auto-discovery + cadence scheduler are **not shipped** — see §5); (c) close the
already-tracked SEO/OG gap; (d) split the CTA path for the two audiences (solo dev vs team lead).

---

## 1. Source map

### Mockups (targets)
| File | Role |
|---|---|
| `docs/mockups/Sensei/Sensei.html` | Product page shell. Renders **`<VariantB/>`**; loads `variant-a.jsx` only for the shared `DojoForTeams` section. This is the live marketing target. |
| `docs/mockups/Sensei/site/variant-b.jsx` | Product page ("confident continuity"). Render order: `NavB · HeroB · StatsB · WhatItIsB · HowItWorksB · Surfaces · DojoForTeams · PhilosophyB · PrivacyB · PricingB · FaqB · SupportB · FooterB`. Note it imports `<Surfaces/>` — the screenshot `GalleryB` is defined but **not** rendered. |
| `docs/mockups/Sensei/site/surfaces.jsx` | **The screenshots→flows replacement.** `SurfaceToday/Sessions/Insights/Memories/Instruments` + `HeroBrief`. |
| `docs/mockups/Sensei/site/variant-a.jsx` (lines 41–199) | **`DojoForTeams()`** — the new Dōjō section. |
| `docs/mockups/Sensei/hq/site.jsx` | Studio hub (portfolio of 4 tools). Already ported. |
| `docs/mockups/Sensei/hq/_dojo-teams.png`, `_dojo-teams2.png` | Renders of the DojoForTeams section. |
| `docs/mockups/Sensei/screenshots/dojo-knowledge.png` | In-app Dōjō SaaS entry ("Your team kept learning while you were away") — visual reference for a Dōjō landing. |

### Current site (to change)
| File | Today |
|---|---|
| `website/src/routes/sensei/+page.svelte` | Product page. **Uses `Mock*` screenshots** in hero + gallery. No Dōjō section. |
| `website/src/lib/components/mock/Mock{Today,Sessions,Insights,Memory,Instruments,Sidebar}.svelte`, `AppFrame.svelte` | 811 LOC of hard-coded app replicas ("Tuesday, March 12", "Good morning, Aiko", `useEffect` copy). The stale-screenshot liability. |
| `website/src/routes/+page.svelte` + `lib/components/hub/*` + `lib/hub-data.ts` | Studio hub — already matches `hq/site.jsx` ("verbatim from …/hq/site.jsx"). |
| `website/src/routes/sensei/{faq,docs}/+page.svelte`, `routes/{privacy,terms}` | Sub-pages. |
| `website/src/app.html`, `routes/+layout.svelte`, `routes/sensei/+layout.svelte` | Head/meta. Only `<title>` + `<meta description>`. **No canonical / OG / Twitter Card.** |

---

## 2. What the mockups change vs today

### Shift A — Screenshots → flows (the core change)
- **Today:** the hero shows `MockToday` (a fake app window), and the "Screens" gallery renders
  `MockToday/Sessions/Insights/Memory/Instruments` — pixel replicas of the app.
- **Mockup:** the hero shows `HeroBrief` (the *one teaching* artifact, labelled "an example
  morning," chrome-less); the gallery becomes `<Surfaces/>` — each surface is an
  **identity + headline + lead + "Why this shape" + a mechanic** (a flow strip, an anatomy grid,
  or a lane triptych). No app chrome to keep in sync.
- **Rationale (verbatim in `surfaces.jsx`):** "a surface's GOAL … and its FLOW … stay true release
  to release. So we describe what each surface is for, why it's shaped that way, and how the flow
  moves — no captures to keep in sync."
- **Net effect:** the page stops making a promise the UI has to keep pixel-for-pixel, and starts
  explaining *why each screen exists*. This is a better fit for a product whose UI is actively
  being rebuilt (per the app-rebuild work).

### Shift B — Screen-agnostic content
The `Mock*` components hard-code sample data (names, dates, project names). The `Surfaces` copy is
about **behavior and contract** ("out of everything sensei watched overnight, Today elevates a
single focal observation") — no dataset to age. `HeroBrief` still shows a concrete example, but
flags it "an example morning" so it reads as illustration, not a live screenshot.

### Shift C — Add Dōjō
Net-new section between Surfaces and Philosophy: **"For teams · 結 Dōjō."** Introduces the
company-hosted collective, the five-stage loop, the six artifact types, one-dev-many-orgs routing,
and a Collective-vs-Dōjō table. Today the site has **zero** team/collective content — Sensei is
presented as purely solo/local-first.

### IA / nav / messaging shifts
- **Nav** is unchanged in the mockup (How · Screens · Philosophy · Privacy · FAQ). Dōjō gets no nav
  entry despite being a major section — see §6 (add "Teams").
- **Messaging** stays "patient observer / quiet by default," but Dōjō introduces a second, louder
  promise ("your team's hard-won lessons — shared, governed, routed"). The page now speaks to two
  audiences; today it speaks to one.

---

## 3. Section-by-section map (product page)

| # | Section (mockup) | Communicates | vs current site |
|---|---|---|---|
| Nav | `NavB` | logo + How/Screens/Philosophy/Privacy/FAQ | Same. No "Teams" link (gap). |
| Hero | `HeroB` + **`HeroBrief`** | "A quiet companion for AI-assisted work"; centerpiece = the *one teaching* artifact | Current hero uses **`MockToday` screenshot**. Copy nearly identical. |
| Stats | `StatsB` | `0 external requests · <60MB · MCP · Free` | Current already softened to `Preview / free during preview`. Mockup's "0 external requests" needs a Dōjō caveat (§4). |
| What it is | `WhatItIsB` | "One desktop app. One quiet promise." | Same copy. |
| How it works | `HowItWorksB` | 観・察・覚 Watch/Notice/Adopt, 3 cards | Same. |
| **Screens** | **`Surfaces`** | 5 surfaces as goal + flow + "why" | **Current = `Gallery` of 5 `Mock*` screenshots.** The headline change. |
| **For teams** | **`DojoForTeams`** | Dōjō: loop, artifacts, routing, Collective-vs-Dōjō | **Net-new. Absent today.** |
| Philosophy | `PhilosophyB` | 静 stillness; "silence is the feature" | Same. |
| Privacy | `PrivacyB` | 蔵 local storage, no telemetry, easy delete | Current is **more accurate** (says PostgreSQL, Ollama-local, "beyond the AI assistant you already use"). Mockup still says "SQLite" — keep the current wording. |
| Pricing | `PricingB` | "Free. Pay what feels right." | Current reframed to **"Free during preview"** + early-adopter discount. Keep current. |
| FAQ | `FaqB` | 5 Q&A | Current lists specific assistants (Claude Code/Cursor/…), 20 commands/8 agents — richer + truer. Keep current, add a Dōjō Q. |
| Support | `SupportB` | 志 GitHub Sponsors | Same. |
| Footer | `FooterB` | product/source/connect cols + version | Current uses dynamic `v{__APP_VERSION__}` (0.2.43); mockup hard-codes `v0.4.2`. Keep dynamic. Add a Teams/Dōjō link. |

### The five surfaces (from `surfaces.jsx`) — what each mechanic is
- **01 Today (観):** priority-ordered anatomy grid — P0 focal teaching → P1 also-worth-noticing →
  P2 system-has-learned → P2 First-Try-Right/14d. "One ranked item forces a decision and respects a
  morning."
- **02 Sessions (録):** three lanes (Going well / Not going well / Insights) + "how a session is
  read" (Captured→Scored→Checkpointed).
- **03 Insights (今):** a 4-step **flow** — Noticed (sensei) → Scored (sensei) → Reviewed (you) →
  Promoted (you). "Sensei proposes; you dispose."
- **04 Memories (覚):** anatomy (When to apply / Examples watched / Provenance) + Adopt/Refine/Dismiss.
- **05 Instruments (具):** three modes — Playground / Replay / Health.

### The Dōjō section (from `variant-a.jsx`) — what it contains
- **The loop:** Contribute (individual) → Accumulate (the hive) → Triage (maintainer) → Approve
  (maintainer) → Distribute (everyone), with a "distributed back to every matching scope" return arc.
- **What flows through it:** Guiding principles (理) · Patterns (紋) · Prompts (問) · Guards (守) ·
  Skills (技) · Agents (使).
- **One developer, many orgs:** Employer / Clients / Communities / Personal chips; "every project is
  bound to exactly one — findings route only where they belong."
- **Ranked by priority, not hierarchy:** Security/Architecture/Testing ladders with P0/P1/P2 rungs;
  "When principles compete, the higher rung wins." **(⚠ see §5 — not grounded as written.)**
- **Collective vs Dōjō table:** global public commons (anonymized, reputation-based, opt-in) vs
  private governed company-hosted (attributed, triage + named approval, roles/policies).

---

## 4. The one thing to get right: reconcile local-first vs networked Dōjō

The current site's strongest, most repeated promise is **absolute locality**:
- Stat: "**0** external requests"
- Privacy: "Sensei **never makes outbound network requests** beyond the AI assistant you already use"
- Hero note: "Local-first · No account required"

Dōjō is, by design, **networked and account-based** (company-hosted SaaS at `dojo.sensei-hq.org`
or self-hosted; SSO/GitHub/device-code auth; contributions leave the machine). Dropping a Dōjō
section next to "0 external requests" reads as a contradiction unless the framing is explicit.

**Recommended framing (Must):** make the locality promise *conditional and opt-in*, consistently:
- Solo Sensei = "0 external requests, nothing leaves your machine — **until you choose to join a
  Dōjō.**"
- Dōjō = "opt-in, off by default; you pick what to share and at what scope; source is stripped
  before anything leaves."
- Keep the Dōjō section visually and narratively downstream of the personal story, so the default
  reading is still "quiet, local, yours," and Dōjō is the *"and when you're on a team…"* extension.
- Consider a one-line bridge sentence at the top of the Dōjō section (the mockup already has a good
  one: "Sensei is local-first for one developer. Dōjō is its company-hosted counterpart… with
  nothing leaking") — but qualify "nothing leaking" per §5.

---

## 5. Grounding check — Dōjō claims vs what the product does

Grounded against the specs (`observatory-collective.md`, `pipeline/collective-intelligence.md`,
`pipeline/dojo-lifecycle.md`, the four console specs, `park/_dojo-build-plan.md`). **Build status
is the load-bearing caveat: the Docker-free spine is built + tested, but the SaaS console app,
admin/client-lead backend endpoints, share-review desktop screen, auto-discovery, and the cadence
scheduler are NOT shipped.** The website should market the *shape* of Dōjō, not imply the whole
console/SaaS exists today.

| Mockup claim | Reality | Verdict |
|---|---|---|
| Loop: Contribute→Accumulate→Triage→Approve→Distribute, distributed back to matching scopes | Matches the pipeline (cluster-by-signature, score, human triage, named approval, downstream inbox pull every 300s) | ✅ Accurate — safe to ship |
| Six artifacts (principles/patterns/prompts/guards/skills/agents) round-trip | Stored in `dojo.artifacts`, upstream/downstream shapes identical | ✅ Accurate |
| One dev → many orgs; one project → exactly one Dōjō; findings route only where they belong | Implemented: `projects.dojo_id`, `client_precedence_route()` (pure fn, 58 tests, called live) | ✅ Accurate |
| Client work is source-dereferenced automatically; the lesson travels, the source doesn't | Fail-closed dereference (`attribution.rs`, 20+ adversarial tests); client-lead cannot override | ✅ Accurate — and a great trust point to lead with |
| Global Collective is anonymized (source stripped, stack descriptor, stable anon id, k-anonymity ≥3) | Grounded; **stricter** than employer/client scope | ✅ Accurate — but note k-anon is **global-only** |
| **"…with nothing leaking"** (section lead) | Overstated. Source refs are stripped for *client* work and generalized for *global*; but **employer-scoped** work is named-to-you and org-internal, and *does* reference internal specifics inside the org. "Nothing leaks" ≠ "nothing is shared." | ⚠ **Revise** — say "source stays home; the lesson travels" and "client source is stripped automatically," not a blanket "nothing leaks." |
| **"Ranked by priority, not hierarchy … the higher rung wins"** (P0/P1/P2 ladders) | Not found in the Dōjō conflict model. Governance uses **mandatory vs advisory** enforcement + scope-matching + consumer mute/pin, and contribution **thresholds** (e.g., memory `strength ≥ 0.7`), not a P0/P1/P2 "rung" tiering. | ⚠ **Reframe** to the real model (mandatory rules win; more-specific scope refines; you keep mute/pin), or drop the ladder metaphor. Don't invent a tier system. |
| Collective **vs** Dōjō as two distinct systems | Spec unifies them: "there is no separate 'Collective' concept — it's the `global-dojo` Dōjō at scope `global`." | ◑ The user-facing distinction (public commons vs private governed) is still useful, but "vs" slightly over-separates. Consider "One model, two scopes: the public commons, and your private Dōjō." |
| (Implied by a console-heavy pitch) admin/maintainer/client-lead consoles exist | Only the **maintainer** surface has backend endpoints; **admin + client-lead consoles are not built** (no `console/` app, no Supabase scaffold) | ⚠ Do **not** show working console screenshots or "sign in to your admin console" CTAs yet. |
| (If added) "auto-discovers/joins your org's Dōjō" | Auto-discovery `.well-known/dojo` probe is **parked**; auth is manual (device-code/GitHub) | ⚠ Avoid "automatic join" language. |
| (If added) "shares on a daily/weekly schedule automatically" | Cadence toggles exist in UI, but the **scheduler is not wired** — publish is manual today | ⚠ Say "you choose when to share" / "scheduled batches (coming)"; don't imply auto-fire now. |

**Safe, high-trust claims to feature:** "org-hosted or self-hosted — your infrastructure";
"client work is source-stripped automatically, verifiably"; "contribute → maintainer approves →
lands in everyone's Upgrades"; "personal Sensei works fully standalone; Dōjō is optional and
opt-in."

---

## 6. Suggestions + enhancements (prioritized)

### MUST (do before/with the build)
1. **Kill the `Mock*` screenshot components; port `<Surfaces/>` + `HeroBrief` faithfully.** They
   are the whole point of the redesign and remove ~811 LOC of stale-data liability. (`Mock*` under
   `website/src/lib/components/mock/`.)
2. **Reconcile local-first vs Dōjō messaging** (§4). This is the single biggest coherence risk.
3. **Trim Dōjō copy to shipped reality** (§5): revise "nothing leaking," reframe the P0/P1/P2
   "higher rung wins" ladder, avoid auto-discovery/auto-cadence/console-ready implications.
4. **Add "Teams" (Dōjō) to the nav** and a footer link. A major section with no nav entry gets
   missed; team leads are the ones scanning the nav for it.
5. **Preserve the current site's factual corrections** when porting from the mockup: PostgreSQL (not
   SQLite), Ollama-local inference, "free during preview" pricing, dynamic `v{__APP_VERSION__}`,
   the real assistant list + toolkit counts in the FAQ. The mockup JSX predates these fixes.
6. **SEO/OG/canonical/sitemap** (already tracked in `docs/backlog.md` §10 "On-page SEO gaps"):
   add `og:title/description/image`, `twitter:card`, `<link rel="canonical">` in root
   `+layout.svelte`/`app.html` so every route inherits, plus a generated `sitemap.xml`. The new
   Dōjō section is a fresh indexable page-fragment — give it an `og:image` (the `_dojo-teams.png`
   render is a ready candidate). Today there is **only** title+description.

### NICE (meaningfully better)
7. **Split CTA by audience.** Solo dev → "Download for {OS}" (exists). Team lead → a distinct
   secondary path in the Dōjō section: "See how Dōjō works" / "Talk to us about a team Dōjō" /
   "Read the Dōjō docs." Right now every CTA funnels to a solo download; the Dōjō section has no
   next step of its own.
8. **Anchor the Dōjō trust story on the dereference guarantee.** "Client source is stripped
   automatically — verified, fail-closed" is the strongest, truest line and exactly what a
   consulting/agency lead fears. Consider a small "what leaves / what stays" two-column visual
   (the `dojo-lifecycle.md` attribution table maps cleanly: personal-OSS / personal-closed /
   employer / client → what's stripped). This turns an abstract promise into a concrete contract.
9. **Give the Dōjō loop a "so what" payoff line.** The loop cards explain mechanics; add one
   outcome sentence ("a lesson one dev learns on Tuesday is protecting the whole team by Friday")
   and, later, a real metric once peer/community comparison ships (G8 in the build plan).
10. **`HeroBrief` "example morning" honesty.** Keep the "an example morning" label prominent so it
    never reads as a live screenshot; consider a subtle "illustrative" watermark. This is the
    credibility payoff of dropping screenshots — don't undercut it with a too-real-looking mock.
11. **Reduced-motion / accessibility pass on the new content.** The loop uses an SVG return-arc and
    `→` glyphs; ensure the flow is legible without color (P0/P1/P2 tones), the `→` connectors have
    text equivalents or `aria-hidden`, the decorative kanji watermarks are `aria-hidden`, and the
    `<details>` FAQ keyboard behavior is preserved. The mockup's color-only priority tones
    (P0=accent/P1=amber/P2=grey) need a non-color signal (the label text already carries it — keep it).
12. **Semantic-token compliance on port.** The current `/sensei` page is bespoke CSS with `--shu`,
    `--sumi`, `--paper` vars (not the rokkit named-token vocabulary the app uses). The new Surfaces
    + Dōjō blocks are large; porting them is the moment to decide: stay with the site's local CSS
    system (fine, it's isolated) but at least keep it internally consistent, and make the accent map
    to one token so a future theme change is one edit. (Not a blocker — flagging so it isn't
    hand-rolled ad hoc across 6 new blocks.)
13. **Cross-link the hub.** The hub's Sensei card (`hub-data.ts`) sells solo Sensei; add a "for
    teams" beat there too, or at least ensure `/sensei#teams` is linkable from the hub.

### SPECULATIVE (worth a conversation, not obviously in-scope)
14. **A dedicated `/sensei/dojo` (or `/teams`) page** rather than one section. If Dōjō grows a
    real pitch (roles, self-host vs SaaS, pricing, compliance/audit story for agencies), a section
    won't hold it. A section now, its own page when the console/SaaS actually ships (post-build-plan
    R6/R9–R11). The `dojo-knowledge.png` "Your team kept learning while you were away" landing is a
    strong page hero when that day comes.
15. **Interactive flow diagram** for the loop/Insights flow (hover a step → its detail) instead of
    static cards. Higher engagement, but more to maintain — probably not worth it pre-launch.
16. **"Is my work safe?" self-select widget** — pick personal-OSS / employer / client and see
    exactly what's shared vs stripped (drives the trust story from §8 interactively). Speculative;
    only if the trust framing tests as the conversion lever for team leads.
17. **Social proof / logos** once there are real Dōjō users — deliberately omitted now (nothing to
    show), but leave a slot in the Dōjō section IA so it isn't a retrofit later.

---

## 7. Open questions for Jerry

1. **Section vs page for Dōjō?** Keep it as one section on `/sensei` (mockup's choice), or spin a
   `/sensei/dojo` page now with the section as a teaser? (Affects nav + SEO + how much Dōjō detail
   the homepage carries.)
2. **How hard do we sell Dōjō before the console/SaaS ships?** The spine is built but the
   admin/client-lead consoles and SaaS auth plane are not (§5). Market it as "available now,"
   "early access / waitlist," or "coming"? This gates the CTA wording and whether we collect emails.
3. **The Collective-vs-Dōjō framing:** spec says they're one system (global = the public Dōjō).
   Keep the "vs" table (clear for users) or reframe as "two scopes of one model"? Your call on
   which mental model you want to teach.
4. **P0/P1/P2 "higher rung wins" ladder** — do you want to (a) reframe to the real
   mandatory/advisory + scope model, or (b) keep the ladder as an intentional simplification? It's
   currently not grounded as written.
5. **"0 external requests" stat** — keep it (with a Dōjō asterisk), soften it ("local-first by
   default"), or drop it now that a networked feature is on the same page?
6. **Audience priority:** is the primary conversion still the solo-dev download, with Dōjō as a
   "…and for teams" beat? Or is Dōjō becoming a co-equal pitch? Determines section order, CTA
   hierarchy, and nav.
7. **Trust visual:** want the "what leaves / what stays" attribution table rendered on-site
   (from `dojo-lifecycle.md`)? It's the most persuasive asset for the team-lead audience.
8. **Hub scope:** the hub (`/`) already matches `hq/site.jsx`. Does it also need a "for teams"
   mention for Sensei, or is that entirely a `/sensei` concern?

---

## Appendix — factual deltas to carry from current site into the port (don't regress)
- Storage: **local PostgreSQL** (mockup says "SQLite file").
- Inference: **Ollama, on-device** (mockup omits).
- Network claim: "never makes outbound network requests **beyond the AI assistant you already
  use**" (+ Dōjō opt-in caveat) — not a bare "0 external requests."
- Pricing: **"Free during preview"** + early-adopter permanent discount (mockup: "Free forever /
  pay what feels right").
- Version: dynamic **`v{__APP_VERSION__}`** (currently 0.2.43); mockup hard-codes `v0.4.2`.
- FAQ specifics: real assistant list (Claude Code, Cursor, Windsurf, Copilot, Codex, Aider) + "20
  commands, 8 agents" toolkit (mockup: generic "any MCP assistant").
</content>
</invoke>
