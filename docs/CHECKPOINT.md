# Checkpoint — 2026-10-27 (end of day; v0.21.1 tagged)

**Agenda — 2026-10-28 (user):**
1. **Apple Developer ID → #282.** Sign senseid/sensei/sensei-mcp with it, so a reinstall stops revoking Keychain
   approval. Then ONE signed install of everything waiting (below) and verify live: get_duplicates from qlty,
   sync rows by identity, Make default, a dead credential → Sign in; rerun `bun run test:inspect -- tool-calls`.
2. **Screen-by-screen feedback walk.** One screen at a time; the user gives feedback, changes are made, verified,
   next. Start from the queue in `docs/roadmap/08-issue-burndown.md` (UI walk).

**v0.21.1 tagged on main (PR #344) and installed locally (ad-hoc signed, so Keychain approval is asked again).**
It carries everything built today: sync rows by identity, Make default, Settings · Dōjō by identity → day →
repository, get_duplicates from qlty, dead credential → Sign in, General toggles, Settings as its own window
(⌘,), Playground runs in watched workspaces, rokkit 1.9.1, the Supabase advisor fixes. The tag's release.yml
waits on the user's approval of `deploy-dojo` (production DB), then deploys the Worker; done when
dojo.sensei-hq.com/version reports v0.21.1. Live verify of the list above happens after the #282 signed install.
The review index (docs/review/README.md) and the signing steps (docs/ops/apple-signing.md) are committed.
Cloudflare's git integration for the dojo Worker: recommended disconnect (release.yml deploys it).

**2026-10-28 (user): stability and consistency first.** Done and pushed: the three sidebars are one Rail
(d4d9039c, Playwright-verified identical); Workspaces list query aggregates once (f5f9a631, 29.6→1.2 ms);
identities — one PersonaList decides status/sentence/tone/button from daemon facts + its clock, renewal folded in,
Settings and the overlay render it (0db67419, b60ea672, 09703055); the state/view audit
(docs/review/2026-10-28-state-view-audit.md) items 1–14 and the duplicated parts of 15, each red-first. Open:
item 15's single-site remainder; RelayNotifyToggle (unrendered) — wire or remove; a `make install` to see it all
(one more Keychain prompt). rokkit main merge (publishes the site) is the user's: blocked for me. NEXT: the
Settings screen walk.

**Queued (burndown, UI walk):** Roots removal progress over the SSE stream (removal is one prune call today —
needs steps that emit events); Libraries "sensei will wrap" claims docs not held; Instruments switch;
Replay's scrolling and an easier call history; Svelte components as search symbols.

**Supabase advisor (production dōjō):** fixed in DDL (0bbbc75c) — RLS on all 13 tables, search_path pinned,
guarded by database/tests/dojo/advisor_guards.sql; reaches production with the next release's dōjō deploy.
Production's Data API exposed `sensei`, unlike the repo (public, graphql_public, dojo); the user removed it
(2026-10-27) — no code reads sensei through the API. The 47 SECURITY DEFINER warnings are the dōjō's caller-gated API + RLS
helpers (by design); a test that calls each as a non-member is the proposed hardening.

**Known:** CI qlty workflows disabled (out of minutes; `gh workflow enable` restores). Long-lived Claude
sessions keep an old sensei-mcp (404 on update_phase) — `/mcp` reconnect. Keychain tests fail on rebuilt
test binaries (#282).
