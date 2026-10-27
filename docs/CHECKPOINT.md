# Checkpoint — 2026-10-27

**Slice:** the baseline (user, 2026-10-27) — every screen, every clickable, the console, and every sensei MCP
tool run and its answer checked against what is true. Report: `docs/baseline/2026-10-27-app.md`.

**Done (develop):** 31 of 32 read-only sensei tools right, each held to an oracle worked out without the tool
(`app/e2e/tests-inspect/tool-oracles.ts`; `bun run test:inspect -- tool-calls`). 12 defects fixed red-first:
a4f9659e (Playground runs in the workspace — "Runs in" picker, `?workspace=`; file tags on `sensei.files`;
conventions from files; rank_bm25; pattern reads refuse no/unknown workspace; playbook refusals; nudges;
communities capped with total; MCP lookup on `?folders=roots`), 14535af6 (library refs by id /
`ecosystem:name`; `dbd`, `cmake` ecosystems). Live DB converged (dbd reconcile + `apply -n`), release
daemon installed, sensei rescanned (224 route files tagged).

**Next:** profile the Layers diagram freeze in the webview (payload is small; jsdom does not freeze).
Then `get_duplicates` precomputed; a not-watched metric reason; the dōjō crawl once signed in locally.

**Open questions (user):** run the 19 write tools against a scratch workspace and clean up? Approve Keychain
("Always Allow" in Identities) so the dōjō sync resumes; #282 when the Developer ID arrives (2026-10-28).

**Known broken:** dōjō sync since 10-22 (Keychain) → production dōjō empty. A long-lived Claude session keeps
its old sensei-mcp (this one predates the workspace rename): its `update_phase` 404s — `/mcp` reconnect.
Keychain tests fail on rebuilt test binaries (#282). `dbd reconcile` misses function-body drift.
