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
1d084c72 no "sensei speaks" caption; be2f37f0 no stray mark before a root's path. The app must go with the
daemon (the dōjō screen reads the identity the new daemon writes). Live DB already converged.

**In progress:** rokkit #173 (zen-sumi Switch off state) and #174 (a style's thumb travel overrides every
size) — an agent is fixing and releasing in ~/Developer/rokkit; then bump @rokkit/* here.

**Queued (burndown, UI walk):** Roots removal progress over the SSE stream (removal is one prune call today —
needs steps that emit events); Libraries "sensei will wrap" claims docs not held; Instruments switch; the
settings sidebar vs the main sidebar; the settings layout (close button vs child window — ask).

**Known:** CI qlty workflows disabled (out of minutes; `gh workflow enable` restores). Long-lived Claude
sessions keep an old sensei-mcp (404 on update_phase) — `/mcp` reconnect. Keychain tests fail on rebuilt
test binaries (#282).
