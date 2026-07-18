# Sensei mockups — what to fix and build

One page for the designer. What each mockup surface needs, plus the roadmap the site
should reflect. Kept simple and current (2026-07-16).

Before opening any file, check **`docs/spec/MOCKUP-INDEX.md`** — it maps each screen to
its current source file. Many screens have older/superseded variants; never target
anything under `lib/discarded/`.

---

## The roadmap

The site and mockups must show what's **shipped** honestly and mark everything else as
**roadmap** — never present an unbuilt feature as available. Each roadmap item gets a
status badge and a waitlist ("notify me").

The roadmap data already exists: `site/features-data.js` (mirrors `website/src/lib/features.ts`).
It's on `window.SENSEI_ROADMAP`. Read from it; don't hardcode. Update `status` there as work
lands and the mockups follow.

Where things stand:

- **Available now (shipped)** — the app (Today, Sessions, Insights, Memories, Instruments,
  Projects) and the engine (local capture, live code graph, the learning loop, semantic
  search + context-pack, MCP server, model routing, rules & governance).
- **Next — deeper insight (planned)** — Configure & preferences, Extend & customize,
  Pattern catalog, Export & import, DORA metrics, starter governance bundle.
- **Dōjō — for teams (in progress)** — team consoles, contribute → approve → distribute,
  sign-in & membership.
- **Relay (beta)** — the live line, the run engine, the mobile companion.
- **On the horizon (planned)** — the Collective, Enterprise (SSO / SCIM / billing).

---

## Website — `site/variant-*.jsx`

Build on **variant-b** (it has the right positioning). Keep variant-a's restraint as the
tuning target. Leave variant-c out for now.

**Fix the copy (these claims are wrong — and they're also live on the real site):**

- **Privacy over-claim.** Remove "0 external requests", "never makes outbound network
  requests", and "logs nothing remotely". Sensei does make outbound calls (your AI model,
  Dōjō, Relay, library docs, update check). Say instead: *local-first; no telemetry; nothing
  leaves without an explicit action you take.*
- **Export / import.** Remove the "Settings → Export / Import JSON" claim. It isn't built.
  It's a roadmap item.
- **Assistants.** Don't say "any assistant that speaks MCP", and don't name Cursor / Windsurf
  / Copilot / Codex / Aider. Say: *works with Claude Code and Zed today; more as adapters land.*
- **Instruments.** Drop "watch toolset health over time" (not built). Say what it does now:
  *try tools in isolation, replay what the assistant did.*

**Don't reintroduce these (the live site is already correct):**

- Don't name **SQLite** — say *a local database in `~/.sensei`*.
- Don't hardcode the version — wire the footer to the app version.

**Add:**

- **Roadmap beat.** A new section after pricing that reads `window.SENSEI_ROADMAP` and shows
  Dōjō / Relay / Collective with status badges. Include a **waitlist** field ("notify me" /
  "request early access") on the roadmap items.
- **FTR beat.** First-turn-resolution is the product's headline metric but appears nowhere.
  Add it — a stat in the stats strip or a caption on Sessions.

**Pricing.** Keep the honest "free during preview / sponsorship" model. Drop variant-a's "no
upgrade prompt. Ever." Mention the paid team / Dōjō tier only in the roadmap, not as a live tier.

**Design & layout:**

- **Design system** — use the named tokens and the type scale, not inline styles, off-scale
  font sizes, or deprecated numbered tokens.
- **Responsive** — nothing adapts today. Add breakpoints (hero, stats 4→2→1, steps, gallery,
  pricing) and a mobile nav. Non-negotiable for a public site.
- **Hierarchy** — variant-b uses one big size for the hero *and* every section heading. Step
  the section headings down so the hero stands alone.
- **Nav** — add Pricing / Teams / Relay anchors and a persistent Download button.
- **Essentials** — add a quickstart / install snippet (brew), a docs link, and a trust strip.
- **Dead code** — `GalleryB` is defined but never rendered. Delete it or wire it.

**Waitlist build note (for whoever implements it):** the site is static — there's no server.
The form must POST from the browser to the Dōjō Worker (`dojo.sensei-hq.com/v1/…`) or do a
Supabase insert. It can't use a SvelteKit form action.

---

## Sensei — the app — `lib/observatory/`, `lib/project/`, `lib/setup/`, `lib/relay/`

The app screens are largely done (25 live). Read `MOCKUP-INDEX.md` first — prefer the
`-v2` / `-simple` files, never `discarded/`.

**Still to design (these are the "deeper insight" roadmap screens):**

- **Configure & preferences** (#43) — the settings surface. Also split the setup wizard into a
  short first-run flow plus a persistent, editable Preferences screen.
- **Extend & customize** (#44) — finish the agent / persona / skill editors.
- **Pattern catalog** (#45).
- **Context-pack tool** (#46).

**Fix:**

- **Instruments → Health** is parked (the data doesn't join yet). The Health mockup shouldn't
  imply it works — mark it roadmap or soften it.
- Any screen showing **Dōjō / Collective / Relay** is a roadmap feature. Fine as a design — just
  make sure the website doesn't present these as shipped.

**Consistency:**

- Keep app screens on the design system (`assistant-card.jsx` is the reference).
- The Rokkit component migration (#47, steps 4–14) is ongoing — new or edited screens should use
  Rokkit components.

---

## Dōjō — team consoles — `lib/dojo/*.jsx`

Twelve console screens (admin, maintainer, lead, developer, plus a shared frame, and
billing / governance / identity / extensions / relay / in-app). This is the **Dōjō for teams**
roadmap (in progress) — the design is ahead of the build.

**Biggest gap — design system (do this first):**

- Almost no adoption. Every screen is hand-rolled inline styles over deprecated numbered
  tokens. Migrate to the named tokens and the `zs-*` components, like the app screens. **Until
  this lands, dark mode is unreadable.**
- Off-scale font sizes everywhere — snap to the type scale.
- A few raw color literals remain (DojoKindTag, relay, billing, extensions, governance) —
  replace them with `-soft` / `-edge` tokens.

**Responsive (weak):**

- Most screens take no mobile prop and either reflow loosely or scroll sideways. Thread a mobile
  prop into each screen and stack.
- Worst case: the lead **audit ledger** scrolls horizontally — make it one card per row on mobile.

**Small fixes:**

- Maintainer **Decline** button looks like a neutral button — make it the danger style (like
  Retract / Supersede).
- Admin **"Retract downstream"** button does nothing — wire a confirm + preview.
- Admin **precedence-ladder verdict** is hardcoded — make it read the actual ladder order.
- Replace **"source dropped"** with **"anonymize"** everywhere.
- **InappShare** checkboxes don't toggle — make the rows real toggles so the count updates.
- Extensions **"Scope ▾"** shows a generic label — show the card's actual scope.
- `dojo-inapp.jsx` has a garbled, duplicated sentence (~L291) — fix it.
- Lead and relay define their own local copies of shared primitives (Panel, MOBILE_TABS) — use
  `dojo-shared.jsx`.

Note: `dojo-saas.jsx` (Orgs, SignIn) is the Dōjō **website** surface, not a desktop screen.

---

## Priority

1. **Website accuracy** — privacy, export, assistants, instruments. It's live and wrong.
2. **Website roadmap beat + waitlist + FTR beat.**
3. **Dōjō design-system migration** — unblocks dark mode; highest-leverage console work.
4. **Website design system + responsive.**
5. **Dōjō responsive + the small fixes.**
6. **Sensei app** — design the roadmap screens (#43–#46), split the wizard, soften Instruments Health.

*Updated 2026-07-16.*
