---
name: Implementation Backlog
description: Prioritized index of work — open items are tracked as GitHub issues (sensei-hq/sensei)
date: 2026-04-28
---

# Implementation Backlog

Work is tracked as **GitHub issues** in [`sensei-hq/sensei`](https://github.com/sensei-hq/sensei/issues), tagged `bug` / `feature`. This file is the prioritized index. When an item ships, **close its issue and remove it from this list**. Detailed engineering history from before this migration (2026-06-04) lives in git (`git log -- docs/backlog.md`).

---

## 1. Top priority — Governance plane

**Concept:** [concepts/governance.md](./concepts/governance.md) · **Epic:** [#28](https://github.com/sensei-hq/sensei/issues/28) · **Builds on:** [knowledge plane spec](./superpowers/specs/2026-05-27-knowledge-plane-design.md)

Layered scope×enforcement rule model, `scopes`+`namespaces` (level-based set membership), global `~/.sensei/rules.md`, README-frontmatter identity, two-tier resolution via the `consolidation` inference role, promotion, and the slim hive-mind federation service. Ships top-to-bottom:

- **P1 — Tool routing** (no schema change, first): [#11](https://github.com/sensei-hq/sensei/issues/11) agents→MCP tools · [#12](https://github.com/sensei-hq/sensei/issues/12) `get_duplicates`/`get_project_conventions` · [#13](https://github.com/sensei-hq/sensei/issues/13) `~/.sensei/rules.md` + CLAUDE.md pointer · [#14](https://github.com/sensei-hq/sensei/issues/14) agents.md doc
- **P2 — Scopes/namespaces/resolution**: [#15](https://github.com/sensei-hq/sensei/issues/15) [#16](https://github.com/sensei-hq/sensei/issues/16) [#17](https://github.com/sensei-hq/sensei/issues/17) [#18](https://github.com/sensei-hq/sensei/issues/18) [#19](https://github.com/sensei-hq/sensei/issues/19) [#20](https://github.com/sensei-hq/sensei/issues/20)
- **P3 — README identity + promotion**: [#21](https://github.com/sensei-hq/sensei/issues/21) [#22](https://github.com/sensei-hq/sensei/issues/22) [#23](https://github.com/sensei-hq/sensei/issues/23) [#24](https://github.com/sensei-hq/sensei/issues/24)
- **P4 — Hive-mind federation**: [#25](https://github.com/sensei-hq/sensei/issues/25) ✅ `sensei-hive` service · [#26](https://github.com/sensei-hq/sensei/issues/26) ✅ daemon federation module (`knowledge_sources` + `federated_memories` ledger, push-on-approve + poll-pull; federated rules are ordinary `memories(origin='federated')` so resolution is unchanged) · [#27](https://github.com/sensei-hq/sensei/issues/27) Configure UI (remaining)

---

## 2. Daemon — ingest & scan correctness (bugs)

| Issue | Summary |
|-------|---------|
| [#29](https://github.com/sensei-hq/sensei/issues/29) | Subfolders auto-promoted to projects (discovery resumer treats every walked dir as a project root) |
| [#30](https://github.com/sensei-hq/sensei/issues/30) | Rokkit not detected as a library (scoped `@rokkit/*` packages missed by resolver) |
| [#31](https://github.com/sensei-hq/sensei/issues/31) | `activity.sessions` empty despite Claude hooks firing |
| [#32](https://github.com/sensei-hq/sensei/issues/32) | Queue activity event hardcodes `elapsed=0.0` (`process.rs:80`) |
| [#33](https://github.com/sensei-hq/sensei/issues/33) | Scan emits no progress events after initial queue + discover-message substring mismatch |
| [#34](https://github.com/sensei-hq/sensei/issues/34) | Test fixture projects pollute the dev DB (`ensure_test_project`) |
| [#35](https://github.com/sensei-hq/sensei/issues/35) | Scan stats: queued cumulative vs processed current-state |
| ✅ SHIPPED + DEPLOYED (2026-06-17) | **`node_kind` enum silently dropped `doc`/`struct`/`component`/`hook`/`extension` nodes** (`upsert_node` enum cast failed; `.ok()` swallowed). Fixed on develop (`9b01da2f`→`95af9848`, DDL `171ad9b4`): 5 kinds added to enum, dead variants removed, guard test, embedding allowlist widened, `.ok()` swallow → `tracing::warn!`. Prod reset+rebuilt + validated live (doc 263, hook 152, component 103, struct 5, extension 49; covers 150, references 3421 — all were 0). Plus export/import infra for capture preservation. Full `/Users/Jerry/Developer` rescan still running. Spec/plan: `docs/superpowers/{specs,plans}/2026-06-16-node-kind-drops*`. |
| _(follow-up)_ | **Codebase-wide silent-error audit.** Find & fix every place that discards an error without logging (`.ok()`, `let _ =`, empty catch, masking `unwrap_or_default`) — errors must be logged so they can be inspected/rectified. Directed after the node_kind fix; the latter is one instance. |

## 3. Setup wizard

| Issue | Summary |
|-------|---------|
| [#36](https://github.com/sensei-hq/sensei/issues/36) | Stage regressions (2026-05-27 smoke) — **may be partly fixed by the wizard rehab; triage and close verified items** |
| [#37](https://github.com/sensei-hq/sensei/issues/37) | Build out placeholder + deferred stages (Projects/Libraries/Instruments; Inference/Assignments need gateway design) |

## 4. Observatory

| Issue | Summary |
|-------|---------|
| [#38](https://github.com/sensei-hq/sensei/issues/38) | Learnings vs Insights overlap — decide `/insights` purpose |

## 5. Daemon pipeline & API + tooling (features)

| Issue | Summary |
|-------|---------|
| [#40](https://github.com/sensei-hq/sensei/issues/40) | Scan pipeline: progress + edge/connection events (pairs with #33) |
| [#41](https://github.com/sensei-hq/sensei/issues/41) | API alignment: missing endpoints (scan roots CRUD, libs configure, mcp registry/configure, projects merge) |
| _(file issue)_ | Surface stale / orphaned / unused projects & folders for cleanup. Scan reconcile already tags dead-but-ambiguous folders `stale` and empty projects `orphaned` (never auto-deletes). Needs: list endpoints + a gated purge action + a housekeeping UI (Observatory Configure or Projects setup). Last `~/Developer` rescan: 6 stale folders, 44 orphaned projects. |
| [#39](https://github.com/sensei-hq/sensei/issues/39) | Bootstrap diagnostic logging + debug mode (trace, log viewer, app menu, GitHub submit) |
| [#42](https://github.com/sensei-hq/sensei/issues/42) | E2E: configure Tauri-mode local E2E |

## 6. Design / mockup gaps (features)

| Issue | Summary |
|-------|---------|
| [#43](https://github.com/sensei-hq/sensei/issues/43) | Observatory Configure section (design + build) |
| [#44](https://github.com/sensei-hq/sensei/issues/44) | J7 Extend & Customize screens (design + build) |
| [#45](https://github.com/sensei-hq/sensei/issues/45) | J5 Pattern knowledge catalog (low) |
| [#46](https://github.com/sensei-hq/sensei/issues/46) | J9 Context pack tool (low) |

## 7. UI migration (quality)

| Issue | Summary |
|-------|---------|
| [#47](https://github.com/sensei-hq/sensei/issues/47) | Mockup component migration to Rokkit (steps 4–14) |
| [#48](https://github.com/sensei-hq/sensei/issues/48) | CSS migration to semantic tokens (largely superseded by #47) |

## 8. Future scope (deferred)

| Issue | Summary |
|-------|---------|
| [#49](https://github.com/sensei-hq/sensei/issues/49) | `gateway-embedded` crate — in-process inference adapters |
| [#50](https://github.com/sensei-hq/sensei/issues/50) | Extract `bootstrap` into a reusable library |

## 9. Known issues / limitations

| Issue | Summary |
|-------|---------|
| [#51](https://github.com/sensei-hq/sensei/issues/51) | ACP config: JSONC comment loss on rewrite |
| [#52](https://github.com/sensei-hq/sensei/issues/52) | Database DDL upgrades: no rollback for partial/failed upgrades (offload to dbd) |

## 10. Website (`website/`)

| Item | Summary |
|------|---------|
| ✅ FIXED | **Screens gallery used kanji `先` instead of the logo SVG.** `MockSidebar.svelte:17` (shared by all `Mock*` gallery components) hardcoded `<span class="kanji">先</span>` as the brand mark. Replaced with the sensei logo icon (`i-brand:sensei text-sensei`) + center-aligned `.logo`, matching the real page header at `routes/sensei/+page.svelte:120`. Verified via dev-server screenshot. |
| ⚠️ 1 known error (upstream) | **`svelte-check` baseline cleanup.** `bun run check` had 20 pre-existing errors; **19 fixed** (`app.d.ts`: `__APP_VERSION__` global + `ButtonProps.rel` augmentation + `@types/rokkit__states` shim via `typeRoots`; added `@types/node`; `HTMLElement` casts in `dark-mode.spec.ts`). **1 remaining is upstream rokkit** ([jerrythomas/rokkit#139](https://github.com/jerrythomas/rokkit/issues/139)): `@rokkit/ui`'s `CommandPalette.svelte` ships as plain JS → implicit-any TS7016 with no clean consumer-side fix. **Accepted exception** (user-signed-off) until #139 ships; then remove the `@types/rokkit__states` shim + `typeRoots` too (both only needed for the same upstream gap). |
| _(file issue)_ | **On-page SEO gaps** (from SEO checklist audit): (a) canonical tag missing — add `<link rel="canonical" href="…">`; (b) OpenGraph tags missing — `og:title`, `og:description`, `og:image`; (c) Twitter Card tags missing — `twitter:card`, `twitter:title`, `twitter:description`, `twitter:image`; (d) not yet indexed by Google — submit sitemap to Google Search Console and request indexing for key pages. Likely all belong in the root `+layout.svelte`/`app.html` `<svelte:head>` so every route inherits them, plus a generated `sitemap.xml`. |
