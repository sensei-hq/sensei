# Checkpoint — 2026-10-27 (end of day)

**Agenda — 2026-10-28 (user):**
1. **Apple Developer ID → #282.** Sign senseid/sensei/sensei-mcp with it, so a reinstall stops revoking Keychain
   approval. Then ONE signed install of everything waiting (below) and verify live: get_duplicates from qlty,
   sync rows by identity, Make default, a dead credential → Sign in; rerun `bun run test:inspect -- tool-calls`.
2. **Screen-by-screen feedback walk.** One screen at a time; the user gives feedback, changes are made, verified,
   next. Start from the queue in `docs/roadmap/08-issue-burndown.md` (UI walk).

**Built, pushed, NOT installed (waiting for the signed install):** ebcfe469 sync rows name their identity;
f5eaa279 Make default; 02444525 Settings · Dōjō by identity → day → repository; 140e28d7 get_duplicates from
qlty's scan; 4db76bd8 dead credential → Sign in, no Keychain read; 8b2dc94f General's choices as toggle groups;
1d084c72 no "sensei speaks" caption; be2f37f0 no stray mark before a root's path; 72052d8a Playground sends
what is typed; 3b8017ed "Runs in" lists watched workspaces only; 2677767a Settings as its own window (⌘,);
05cfb84c rokkit 1.9.1. The app must go with the
daemon (the dōjō screen reads the identity the new daemon writes). Live DB already converged.

**Done today, rokkit:** #173 and #174 fixed and released as 1.9.1, bumped here (05cfb84c). glyphs-inspect
stays red on its overview step until the daemon with `?watched=true` is installed.

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
