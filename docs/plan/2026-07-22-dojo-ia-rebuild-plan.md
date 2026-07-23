# Dōjō web-app — IA rebuild plan (2026-07-22)

## STATUS — overnight run 2026-07-22→23 (all SHIPPED to develop, browser-verified)

| Chunk | Commit | Tests | Browser-verified |
|---|---|---|---|
| 1 · DJ1 personal home (membership-less) | `edc0983f` | 311 | ✅ lands "Your work" personal home, zero `/v1/t/` calls, honest empties |
| 2 · Constitution Library | `753bb300` | 354 | ✅ 6 areas, packs, sticky add-to-constitution footer |
| 3 · Effective-constitution Preview | `8fdd00b1` | 396 | ✅ 4 lifecycle projects, classification override, ladder + conflicts |
| 4 · Developer console (teams/contributions/for-me) | `ef996f45` | 423 | ✅ nav wired, honest zero-membership empties |
| 5 · Nav reframe + org-switcher popover | `aa28bacd` | 454 | ✅ Relay·you+Me on top, popover (role=menu) |

Verified in a real browser (wrangler + Playwright MCP) at **390px mobile + 1280px desktop**, **0 console errors** on every screen, with a real membership-less magic-link user. Production build clean (adapter-cloudflare). NOT merged to main / NOT released (Jerry's call).

**Deferred to discuss (see §4):** live personal-Relay `/v1/relay/session` (backend Rust), Governance authoring, two-shell management step-in (Chunk 6), create-a-Dōjō flow, `/v1` wiring for the new screens (Library/Preview/Developer are presentational off local `-data.ts`), true role-aware nav. One small open decision: the pinned "Relay · you" for a *member* navigates to `/console` but doesn't clear their `dojo_tenant` cookie (a member still resolves their org) — belongs with the Chunk 6 "everyone starts in the You zone" reframe.

---


> Bring `dojo/` in line with the **updated mockups** (`docs/mockups/Sensei/lib/dojo/*.jsx`,
> `Sensei Dōjō Console.html`, `lib/data/dojo-data.js`). Authored for an autonomous
> overnight run: commit each verified chunk to **develop** only; no release / no main-merge.

## 1. Target IA (from the updated mockups)

**"One wired app, every role"** (`dojo-flow.jsx`). Not per-role routes — one shell, role-scoped content.

- **Sign in → personal "You" zone** (`zone="you"`): the landing. Left nav prepends a **Relay group** — Projects (場) · Inbox (決) · Chat (話) — then a personal/developer area (My teams 群 · My contributions 共 · For me 贈). **Works fully solo: a Dōjō membership is optional, never a gate** (objective DJ1; `dojo-saas.jsx` `DojoOrgsEmpty`).
- **Org switcher popover** (⌘K, `DojoTopBar`): pinned "Relay · you — all Dōjōs" + each membership + "Your Dōjōs" + "＋ Create or join".
- **Step into a management shell** (`zone="dojo"`, only for `admin|maintainer|lead`; read-only/owner have no console): distinct chrome (`DojoManageBar`, "← Your work" exit), role-scoped nav, **no Relay group**.
- **The ladder, explicit**: Company → Client → Personal → Project → Stack. Two ladders share the list — Personal (free: Personal→Project→Stack) and Company (paid: Company→Client→Team→Project); Stack shared/free; Company-vs-Client derived from membership. Enforcement tiers (`advisory<recommended<required<mandatory`) are the override brake — `mandatory` (constitution) can't be relaxed by a narrower scope.
- **New screens:** Constitution Library (`dojo-library.jsx` — adopt rule packs by area, LevelPills, ★ non-negotiable, write-your-own, "Add N to constitution"), Effective-constitution Preview (`dojo-preview.jsx` — ladder resolved for one project + conflicts), Governance authoring (`dojo-governance.jsx` — stance dials + per-scope rules), Developer console (`dojo-developer.jsx`).

## 2. Current → target gap

Current `dojo/` is an **admin/maintainer-first console**: `/console` lands on a maintainer Overview; nav = Govern/Org/Clients/Trust (not role-aware); **membership-less users 403 on every console surface** (`(console)/+layout.server.ts` — the DJ1 bug + the documented "unblock-everything" item). Built + LIVE: overview, triage(+detail), relay(+detail), members, identities, policies, health, audit, engagements(+detail), incidents. **Not built:** personal "You" zone, org-switcher popover, developer console, Library, Preview, Governance authoring, create-a-Dōjō, the two-shell management step-in. `dojo/uno.config.js` theme parity is **already fixed** (has the `theme` block) — verify only, don't redo.

## 3. Sequenced chunks — each TDD + zero-errors + autofixer + qlty + browser-verify, committed to develop

Priority = value × buildability × spec-clarity. Chunks 1–3 hit the three named pains directly.

### Chunk 1 (P0) — Personal landing without membership (DJ1) — *the flagged bug + unblocks everything*
- **True bug shape (corrected via depth review + code — NOT a 403):** `resolveTenantKey()` (`dojo/src/lib/tenant.ts`) silently falls back to `DEFAULT_TENANT_KEY = orgs[0]?.url` (Acme), so a membership-less user is handed a **fabricated tenant** and shown the maintainer Overview with static demo metrics. `(console)/+layout.server.ts` `getUserOrg()` returns `undefined` for a membership-less user and nothing branches on it. Every `+page.ts` already degrades API errors to `{data,error}` banners → there is no uncaught 403. Start systematic-debugging from *this* symptom (fabricated-tenant + missing membership branch), not a thrown error.
- **Decision (resolves the data-source fork — autonomous default, per `feedback_autonomous_no_premature_stop` + the depth reviewer's recommended path):** the personal landing tonight is a **new presentational personal-home screen** (port of `dojo-saas.jsx` `DojoOrgsEmpty`), rendered when `hasMembership === false`. **No live tenant-scoped data, no new backend/Rust endpoint tonight** — the cloud Worker can't see local FS projects and `relay-data.ts` is tenant-keyed. Live personal-Relay data (a membership-free `GET /v1/relay/session` keyed by user id in `crates/dojo-mind`) is **deferred to morning** and flagged.
- **AC:**
  - `(console)/+layout.server.ts` surfaces `hasMembership` (from real memberships, not the tenant fallback); a membership-less user is **not** handed a fabricated tenant.
  - `/console` renders the **personal home** when `hasMembership===false` (needs-you placeholder · your projects honest-empty · "your own rules · optional" → link to the Library · clearly-secondary "create or join a Dōjō · optional"); renders today's maintainer Overview when `hasMembership===true`.
  - A membership-less user's landing makes **zero calls to any `/v1/t/{tenant}/…` endpoint** (mechanical smoke check).
  - Org-scoped screens (members/identities/policies/triage/engagements/incidents/health/audit) show a "join or create a Dōjō" empty state when `hasMembership===false`.
  - Users **with** membership: unchanged. Local `/v1` 404 degrades gracefully.
- **Files:** `dojo/src/lib/tenant.ts` (no fabricated tenant for membership-less), `(console)/+layout.server.ts` (`hasMembership`), `(console)/console/+layout.svelte` (landing/nav conditional), `(console)/console/+page.svelte` (personal-home vs Overview branch), new `dojo/src/lib/components/DojoPersonalHome.svelte` (+ `personal-home-view.ts` if any logic), org-scoped `+page.svelte` empty-state branch, specs.
- **Verify:** unit (layout load membership vs none; tenant resolution no longer fabricates) + render specs (personal home; empty states) + `bun run check` 0/0 + `bun run test` + autofixer + qlty + wrangler+Playwright-MCP smoke (a locally-minted magic-link user IS membership-less → lands personal home, no `/v1/t/` calls, mobile viewport).

### Chunk 2 (P1) — Constitution Library — *the "templates setup/management" pain*
- **AC:** `/console/library` renders the library: **areas rail** (Core principles 理 · Architecture 紋 · Security 盾 · Compliance 法 · Language & stack 技 · Design & UI 意); **pack cards** (name · source · rec/regulated chips · wired checkers); per-rule **include checkbox + LevelPills (Org/Team/Project/Stack) + ★ non-negotiable**; **write-your-own** card; **sticky footer** with live "N rules selected / M non-negotiable / Add N to constitution". Reachable from nav. Presentational, driven by `library-data.ts` pack catalog (mirrors mockup `LIB_PACKS`, aligned to the seeded `dojo.seed_default_governance()` content), state in `library-state.svelte.ts`. No live `/v1` required; degrades gracefully. `/v1` wiring deferred.
- **Files:** `(console)/console/library/+page.svelte`, `lib/library-data.ts`, `lib/library-view.ts`, `lib/library-state.svelte.ts`, specs, ConsoleNav entry.
- **Verify:** unit (-data/-view/-state incl. selection + count logic) + render spec + gate + browser-verify.

### Chunk 3 (P1) — Effective-constitution Preview — *the "ladder not clear" pain*
- **AC:** `/console/preview` renders the ladder resolved for a selected project: **project picker** (sample projects across lifecycles — company/client/personal/agency-monorepo); **classification banner + reclassify override** (company↔client); **left = ladder rungs** indented by depth, each rule tagged ★ non-negotiable / negotiable / overridden↑, stack rungs show wired checkers; **right = "Conflicts, resolved"** cards (topic · winner · what it beat · why) + a summary card. Pure precedence/resolution logic in `preview-view.ts` (mandatory-lock + most-specific-wins), unit-tested. Reachable from nav.
- **Files:** `(console)/console/preview/+page.svelte`, `lib/preview-data.ts`, `lib/preview-view.ts`, specs, ConsoleNav entry.
- **Verify:** unit (resolution: mandatory locks beat specificity; specificity refines otherwise) + render spec + gate + browser-verify.

### Chunk 4 (P1) — Developer / personal console (My teams · My contributions · For me)
- **AC:** routes rendering the personal seat — **My teams** (memberships + what each follows; client anonymization note), **My contributions** (upstream sends + per-destination status approved/pending/declined), **For me** (approved teachings distributed down; mute/pin). Renders for membership-less users with honest empty states (pre-written copy: "no memberships yet — join or create a Dōjō from the switcher"). Presentational + `developer-data.ts`/`-view.ts`.
- **Files:** `(console)/console/teams|contributions|downstream/+page.svelte`, `lib/developer-data.ts`, `lib/developer-view.ts`, specs, nav.
- **Verify:** unit + render + gate + browser-verify.

### Chunk 5 (P2) — Nav reframe + org-switcher popover
- **AC:** nav restructured toward the new IA — a **Relay group** (Projects/Inbox/Chat) + the personal/developer area surfaced, new screens (Library/Preview) placed, role-awareness where specced; top-bar **org switcher becomes a popover** (pinned "Relay · you" + memberships + create/join) replacing the bare `/orgs` link. Keeps the single `(console)` shell (the two-shell management step-in is Chunk 6, deferred).
- **Files:** `ConsoleNav.svelte`, `ConsoleTopBar.svelte`, a `nav-items` module, specs.
- **Verify:** render specs + gate + browser-verify.

## 4. Deferred — flag for the morning discussion (do NOT build blind tonight)
- **Chunk 6 — two-shell management step-in** (`zone="you"` ↔ `zone="dojo"`, `DojoManageBar`): the deepest reframe; risky to land verified overnight. Draft only.
- **Governance authoring** (`DojoGovernance` — stance dials + per-scope rules/skills/agents/commands + playbook-learning review): large, needs a stance model + `/v1` wiring; mock-only today.
- **Create-a-Dōjō + Starter constitution** (`DojoCreate` + `DojoStarterConstitution`).
- **`/v1` wiring** for the new screens (Library/Preview/Developer) → real `dojo.shared_rules` / memberships.
- **Gated (do NOT touch):** hook-gate activation (P2-S4), prod VAPID/RLS + Supabase Realtime (P2-S5), multi-membership gate routing (P2-S6). Live Supabase/Collective/DORA data (0 rows; deferred).

## 5. Open decisions (defaults chosen so the run proceeds; confirm in the morning)
- **D1 route names:** `/console/library`, `/console/preview`, `/console/{teams,contributions,downstream}`. *(Default; trivially renamable.)*
- **D2 shell depth tonight:** keep the single `(console)` shell + add screens/nav/DJ1; **defer** the two-shell management step-in to morning. *(Default: yes.)*
- **D3 data source for new screens:** local `-data.ts` catalogs (mirroring mockups + seeded constitution), graceful-degrade; `/v1` wiring later. *(Default: yes.)*

## 6. Hard constraints (every chunk)
Named tokens only (no hex/oklch/`var()` in components); 8-stop type scale (never inline `font-size`); 4px spacing grid; radii from tokens; **mobile-first `md:` responsive, no `@media` for layout**; per-surface uno.config parity; state in `*.svelte.ts`, components presentational; voice = lowercase "sensei", no emoji/exclamation, sentence case; Svelte MCP autofixer on every `.svelte`; sensei MCP (`get_rules`/`get_patterns`) before implementing.
