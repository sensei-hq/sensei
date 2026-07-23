# Dōjō web-app — IA rebuild plan (2026-07-22)

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

### Chunk 1 (P0) — Personal access without membership (DJ1) — *the flagged bug + unblocks everything*
- **AC:** A signed-in user with **no org membership** reaches the console without a 403 and lands on a **personal surface** (their Relay / personal home), not the org Overview. Org-scoped screens (members, identities, policies, triage, engagements, incidents, health, audit) render an honest **"join or create a Dōjō"** empty state for membership-less users instead of erroring. Users **with** membership keep current behavior. Local `/v1` 404 still degrades gracefully (dev artifact, not a bug).
- **Files:** `(console)/+layout.server.ts` (expose `hasMembership`, don't hard-require org), `(console)/console/+layout.svelte` (landing + nav conditional), post-sign-in routing (`/` , `/orgs`), API clients' 403→graceful-empty handling, new/reused personal landing.
- **Verify:** unit tests for the load logic (membership vs none) + empty-state render specs; `bun run check` 0/0; `bun run test`; autofixer on every `.svelte`; qlty; wrangler + Playwright-MCP smoke (membership-less sign-in → personal landing, no 403; mobile viewport).
- Root-cause the exact 403 site via systematic-debugging before editing.

### Chunk 2 (P1) — Constitution Library — *the "templates setup/management" pain*
- **AC:** `/console/library` renders the library: **areas rail** (Core principles 理 · Architecture 紋 · Security 盾 · Compliance 法 · Language & stack 技 · Design & UI 意); **pack cards** (name · source · rec/regulated chips · wired checkers); per-rule **include checkbox + LevelPills (Org/Team/Project/Stack) + ★ non-negotiable**; **write-your-own** card; **sticky footer** with live "N rules selected / M non-negotiable / Add N to constitution". Reachable from nav. Presentational, driven by `library-data.ts` pack catalog (mirrors mockup `LIB_PACKS`, aligned to the seeded `dojo.seed_default_governance()` content), state in `library-state.svelte.ts`. No live `/v1` required; degrades gracefully. `/v1` wiring deferred.
- **Files:** `(console)/console/library/+page.svelte`, `lib/library-data.ts`, `lib/library-view.ts`, `lib/library-state.svelte.ts`, specs, ConsoleNav entry.
- **Verify:** unit (-data/-view/-state incl. selection + count logic) + render spec + gate + browser-verify.

### Chunk 3 (P1) — Effective-constitution Preview — *the "ladder not clear" pain*
- **AC:** `/console/preview` renders the ladder resolved for a selected project: **project picker** (sample projects across lifecycles — company/client/personal/agency-monorepo); **classification banner + reclassify override** (company↔client); **left = ladder rungs** indented by depth, each rule tagged ★ non-negotiable / negotiable / overridden↑, stack rungs show wired checkers; **right = "Conflicts, resolved"** cards (topic · winner · what it beat · why) + a summary card. Pure precedence/resolution logic in `preview-view.ts` (mandatory-lock + most-specific-wins), unit-tested. Reachable from nav.
- **Files:** `(console)/console/preview/+page.svelte`, `lib/preview-data.ts`, `lib/preview-view.ts`, specs, ConsoleNav entry.
- **Verify:** unit (resolution: mandatory locks beat specificity; specificity refines otherwise) + render spec + gate + browser-verify.

### Chunk 4 (P1) — Developer / personal console (My teams · My contributions · For me)
- **AC:** routes rendering the personal seat — **My teams** (memberships + what each follows; client anonymization note), **My contributions** (upstream sends + per-destination status approved/pending/declined), **For me** (approved teachings distributed down; mute/pin). Renders for membership-less users with honest empty states. Presentational + `developer-data.ts`/`-view.ts`.
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
