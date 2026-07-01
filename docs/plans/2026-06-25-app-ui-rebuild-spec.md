---
title: App UI rebuild — handoff spec (sidebar + new screens)
description: Self-contained spec to continue the section-by-section app UI rebuild against the updated mockups, with established guidelines + the e2e visual-verification loop.
type: plan
status: in-progress
created: 2026-06-25
references:
  - app/CLAUDE.md
  - docs/design/frontend-svelte-guidelines.md
  - docs/mockups/Sensei/lib/observatory.jsx
---

# App UI rebuild — handoff spec

Continue rebuilding the desktop app UI (`app/`, SvelteKit + Tauri) **section by
section** against the updated mockups in `docs/mockups/Sensei/` (visual source of
truth = `lib/*.jsx`). Work on branch **`develop`**; merge to `main` at milestones.

The daemon is release-installed (`make install` → `/Applications/Sensei.app`,
v0.2.23, all analyzer work live). This phase is **frontend only** unless a screen
needs a daemon endpoint (then: build UI with mock data first, wire API later).

---

## 0. Established guidelines (NON-NEGOTIABLE — from `app/CLAUDE.md` + frontend-svelte-guidelines.md)

1. **Tokens** — the 24 canonical rokkit named tokens ONLY: `paper`,
   `paper-soft`, `paper-mute`, `paper-edge`, `ink`, `ink-mute`, `ink-soft`,
   `ink-faint`, `primary`, `on-primary`, `accent`, `accent-soft`,
   `success`/`-soft`, `warning`/`-soft`, `danger`/`-soft`, `error`/`-soft`,
   `info`/`-soft`, `focus-ring`, `shadow-tint`. Utilities: `bg-paper-soft`,
   `text-ink-mute`, `border-paper-edge`, etc. **No z-scale, no OKLCH/hex in
   components, no `<style>` color blocks.** Tune values via `rokkit.config.js`
   `overrides:` only. Mockup numbered tokens map: `--paper-2`→`paper-soft`,
   `--paper-3`→`paper-mute`, `--edge`→`paper-edge`, `--ink-2`→`ink-soft`,
   `--ink-3`→`ink-mute`, `--ink-4`→`ink-faint`.
2. **State separation** — `*.svelte.ts` files own derivations / status mapping /
   copy / actions. Components are pure templates (props in, markup out).
   `+page.ts` `load()` for pre-fetched data.
3. **Svelte MCP is mandatory** — run the `svelte-autofixer` MCP tool on every
   `.svelte`/`.svelte.ts` before finalizing. (This is the official Svelte MCP
   server; use it for docs/examples too.)
4. **Testing** — co-located `Foo.harness.svelte` + `Foo.spec.svelte.ts`
   (mount via `$lib/test-mount.js` `mountComponent`), OR assert in the parent
   spec. Vitest snapshots checked in. State specs `*-state.spec.svelte.ts`.
   No checked-in PNGs.
5. **Mockup fidelity, but wire API > mockup** — type primitive props against the
   wire enum from `*-types.ts`, not the mockup's prototype values.
6. **Voice** — sentence case, lowercase **"sensei"** (even mid-sentence / brand),
   no emoji, no exclamations, no marketing speak.
7. **DRY** — 2+ uses → component/snippet. Shared primitives in
   `src/lib/components/` (Eyebrow, Kanji, KanjiHeader, StatusDisc, GateRow,
   Spinner, Switch, TabBar, Wordmark, PageHeader, EmptyState, MemoryList,
   **ProgressCard**…). Screen-local primitives co-located until a 2nd consumer.
8. **Pacing** — batch UX changes into one commit before the next slow rebuild;
   don't pivot per observation.

**Commands** (run from `app/`): `bun run check` (svelte-check, must be 0 errors),
`bun run test:unit` (vitest), `bun run tauri icon <svg>` (regen icons).
Pre-commit hook runs fast tests; commits must be zero-error. Co-author trailer:
`Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>`.

---

## 1. E2E visual-verification loop (use it to verify each phase vs the mockup)

Playwright + Tauri (`@srsholmes/tauri-playwright`, socket-driven) renders the real
app so screens can be screenshotted and compared to the mockup artboards.

**Build (slow, ~minutes — REQUIRED after any code change or screenshots are stale):**
```
make app-e2e-build      # install-debug + tauri build --debug --features e2e-testing
```
Builds the e2e bundle against a throwaway `sensei_e2e` DB in `~/.sensei-e2e/`,
daemon on port 7744. **Side effect: globalSetup STOPS the real brew `sensei`
service** to run the e2e daemon — run `brew services start sensei` after e2e work
to restore the real daemon (it auto-restarts via keep_alive, usually already up).

**Run a screenshot spec (targeted):**
```
bun run test:e2e -- e2e/tests/<spec>.spec.ts
```
In a spec: `import { test } from '../fixtures'`, `import { navigateTo } from '../helpers'`.
`await navigateTo(tauriPage, '/route')` (client-side SvelteKit nav; reload breaks
WKWebView). `await tauriPage.screenshot({ path: '/tmp/sensei-shots/x.png' })`
(native window PNG). Settle ~2s after nav. Then **Read the PNG** and compare to
`docs/mockups/Sensei/lib/<screen>.jsx` / the artboard. Report mockup-vs-impl diffs.

Reusable verification specs already exist (UNTRACKED, screenshot to `/tmp/sensei-shots/`):
`e2e/tests/zz-screenshots.spec.ts` (health + observatory), `e2e/tests/zz-degraded.spec.ts`.
Use `execFileSync('brew', [...])` not `execSync` (a PreToolUse hook blocks `exec`).

**Verification GOTCHAS (learned this session):**
- The e2e daemon is **healthy**, so `/health` shows the green "ok" state +
  auto-redirects to `/`. Use `/health?auto=false` to stay on it.
- **Stopping a service does NOT cleanly produce the degraded/remedy/resolving
  screens**: the **ollama gate checks `brew list` (installed)**, not the running
  service → stays green; **postgres uses `pg_isready` (running)** → degrades but
  also kills the daemon (needs pg to serve). The resolving (ProgressCard) +
  needs-action (Remedy) states are best verified by **injecting a HealthState
  payload** via a small test hook, not by killing services. (Unit tests already
  cover the wiring.)
- The e2e bundle reflects code **at build time** — rebuild before screenshotting
  new changes (a descriptions screenshot showed stale text because the bundle
  predated the commit).

---

## 2. Done this session (committed on `develop`)

- `abcf8d7f` — health `FoundationNote` ("what this is" card in the checking state).
- `d71638c9` — **rokkit `@rokkit/* 1.1.1 → 1.3.0`**; dropped the obsolete
  `@rokkit/ui` TreeFields patch (fixed upstream).
- `4893ff71` — **logo fix**: synced canonical `sensei.svg` → `app/static/sensei.svg`
  (was stale); `Wordmark` = logo mark + lowercase "sensei" (was `先生` + "Sensei");
  sidebar header too. **Visually verified** (mark + "sensei" on /health + /).
- `59fe8440` — **reusable `ProgressCard.svelte`** (label/trailing/percent/left/
  right/activity/note; Eyebrow+Spinner; clamps 0–100) wired into HealthView
  **resolving** state; gate descriptions simplified (zen → functional one-liners,
  UI-owned `DESCRIPTIONS` map in `health-state.svelte.ts`).
- `e7b58466` — **app icons regenerated** from `sensei.svg` (`tauri icon`, full set
  + the 3 unreferenced 256/512/1024 PNGs). New icon needs `make install-app` to go live.

All on `develop`; **not yet merged to main**. 568 app tests pass; svelte-check clean.

**Health leftovers (low priority):**
- The new descriptions + ProgressCard are **not yet visually confirmed** (e2e
  bundle was built before that commit) — confirm in the next e2e rebuild (inject a
  `resolving` HealthState payload to see the ProgressCard).
- Voice nit: observatory body still reads "**Sensei** is watching…" → lowercase.
- Gate **titles** ("Database & schema", "Background daemon") — **user said fine
  for now, skip** (would be a daemon `graph.rs` change).

---

## 3. NEXT: Sidebar → rokkit grouped `List` (phase 3) — SPEC LOCKED

Rail mockup = **`docs/mockups/Sensei/lib/observatory.jsx`** lines ~247-323 (NOT
`navigation.jsx` — that's the Projects index grid/palette/tree-browser).
Rebuild `app/src/routes/(observatory)/ObservatorySidebar.svelte` (currently a
hand-rolled `<aside>`) using the rokkit `List` component.

**Layout top → bottom:**
- `Wordmark` (logo + "sensei", already done — reuse `$lib/components` Wordmark).
- Header row: "OBSERVATORY" eyebrow + an **All | Focus segmented toggle** (right).
- The grouped `List`.
- Spacer, then footer: `daemon · running` / `last heartbeat …` (mono, ink-faint).

**Entries (kanji → route, badge / alert):**

| Group | Entry | kanji | route | badge |
|---|---|---|---|---|
| *(anchors)* | Today | 家 | `/` | |
| | Projects | 場 | `/projects` | count |
| **Needs you** | Insights | 今 | `/insights` | 6 |
| | Memories | 覚 | `/learnings` (rail labels "Memories") | 7 |
| | Impact | 果 | **new** `/impact` | 3 · **alert** |
| | Traceability | 巻 | **new** `/traceability` | 4 |
| | Upgrades | 贈 | **new** `/upgrades` | 5 |
| **Review** *(hidden in Focus)* | Sessions | 録 | `/sessions` | 41 |
| | Libraries | 庫 | `/libraries` | 14 |
| | Instruments | 具 | `/instruments` | |
| | Logs | 診 | `/logs` | |
| **— separator —** *(hidden in Focus)* | Preferences | 調 | `/settings` (rail labels "Preferences") | |

(Badges are mock/placeholder values for now — real counts wired later. **Dōjō is
omitted** — standalone-deferred.)

**All | Focus toggle:** `focus` is local `$state` (segmented 2-button control,
mockup ~L264). *All* shows everything; *Focus* shows only the anchors + "Needs
you" (hides the Review group and the Preferences separator section) — "just what
needs a decision."

**rokkit `List` integration contract (`@rokkit/ui` `List`, v1.3.0):**
- `items` array; group = item with a **`children`** array. Set
  **`collapsible={false}`** so groups render as static cluster-label sections
  (the mockup clusters don't accordion).
- `fields` maps data keys, e.g. `{ text:'text', href:'href', value:'value',
  badge:'badge', children:'children' }`. An item with **`href`** renders as
  `<a>` → SvelteKit client nav works. Set each item's `value` = its href and pass
  List **`value={page.url.pathname}`** (`import { page } from '$app/state'`) → the
  wrapper gets `data-active` on the current route.
- Snippets receive **only a `ProxyItem`** (List renders the `<a data-path href
  data-active>` wrapper itself). **Read fields via `proxy.get('fieldName')`** —
  custom fields work (`#fields[name] ?? name`): `proxy.label` (text),
  `proxy.value`, `proxy.get('badge')`, `proxy.get('glyph')`, `proxy.get('alert')`.
  - `itemContent` snippet → render kanji glyph (use `font-kanji text-accent`-ish,
    see `Kanji.svelte`) + label + badge + alert dot.
  - `groupContent` snippet → render the cluster label (uppercase eyebrow).
- Active/hover background come from rokkit theme CSS on `[data-active]` /
  `[data-list-item]`. Accent-on-active kanji may need a CSS rule targeting the
  data-active wrapper (scoped, no color literals — use a token via a class).
- Keep `data-component="observatory-sidebar"` for the existing e2e/test hooks.

Also reuse the rail pattern for **`ProjectSidebar.svelte`** if it shares structure
(check `(project)/project/[id]/ProjectSidebar.svelte`) — defer if different.

---

## 4. New screens (build data-driven with MOCK data, wire API later)

User-confirmed approach: **build each screen data-driven from props/`+page.ts`
mock data now; switch to the real daemon API later** (revisit the API/data surface
at that point). Create routes under `(observatory)`:

- **`/impact`** — mockup `lib/impact.jsx`. (Project-level `/project/[id]/impact`
  exists; this is the observatory-wide view.)
- **`/traceability`** — mockup `lib/traceability.jsx`. **= doc-vs-code drift** (a
  real capability worth having regardless).
- **`/upgrades`** — mockup `lib/upgrades.jsx`.

Each: `+page.ts` returns typed **mock data**; `+page.svelte` is a pure template
matching the mockup (24 tokens, named primitives). Add a `Foo.spec` where it has
logic. Mark the mock clearly (e.g. a `// MOCK: replace with daemon API` note) so
the API-wiring pass is findable.

Step (b) first: create **minimal** `/impact` `/traceability` `/upgrades` pages so
the rail navigates (a `PageHeader` + empty state), then flesh each from its mockup.

---

## 5. Build order + acceptance

1. **Sidebar**: rebuild `ObservatorySidebar` with rokkit `List` + All|Focus toggle
   + entries above. svelte-autofixer + `bun run check` + a spec asserting groups
   render, Focus hides Review/Preferences, active highlights the current route.
2. **Stub routes**: minimal `/impact` `/traceability` `/upgrades`.
3. **Flesh new screens** data-driven from mockups (one per pass).
4. **e2e verify**: `make app-e2e-build` → screenshot the rail (All + Focus), the 3
   new screens, AND the health descriptions + ProgressCard (inject a `resolving`
   payload) → Read PNGs → compare to mockups → report diffs. `brew services start
   sensei` after.
5. Commit per logical slice (zero-error); merge `develop`→`main` at the milestone.

**Acceptance:** rail matches `observatory.jsx` (groups, kanji, badges, alert,
All|Focus behavior, active state); new routes navigable + render their mockup
shape from mock data; svelte-check 0 errors; app tests pass; screenshots reviewed.

---

## 6. Later phases (not now)

- **#4 page-by-page**: remaining observatory/project/preferences screens vs mockups.
- **#5 major arch change**: long setup wizard → **Preferences**; a shorter
  first-run setup reusing setup pages; on first start, **auto-configure assistants
  in the background** (currently via the assistants page); the ONLY first-run user
  action = **folders to scan**; everything else auto-derived + default-configured,
  editable in Preferences. Spans daemon (first-start auto-config) + UI. Mockups:
  `lib/setup-wizard.jsx`, `lib/wiz-assignments.jsx`, `lib/wiz-inference.jsx`,
  `lib/collective-settings.jsx`, `lib/inference-settings.jsx`, `lib/instruments*.jsx`.
- Website `/sensei` static refresh = **#81** (after daemon UI done).
