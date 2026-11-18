# Checkpoint — 2026-11-18 (relay open items — #414 slice 2 built)

**Slice:** `update-shared-state-sources` slice 2 (#414): the realtime channel adapter and the dōjō
host. Branch `lane/relay-414-s2` (worktree `lanes/relay-open`), PR to develop.

**Done (tasks 2.1–2.10):** (2.8: relay-run + relay.spec 4/4 green on the tauri harness)
- `relay/sources/postgres-change.ts`: `fromPostgresChange(payload, tables = RELAY_TABLES)` maps a
  `postgres_changes` payload to the daemon's event (task_id / lane_id / run_id added); SSE parity spec.
- `relay/sources/channel.ts`: `realtimeChannel(client, {topic, schema, tables})` over a structural
  supabase-js client; unmapped tables and unknown statuses are logged; teardown removes the channel.
- `RelayRealtimeTransport.readTask`, `relayRealtimeTaskSource`, `cannedRealtime({task})`.
- dōjō aliases `@sensei/state`; `dojo/src/lib/relay/shared-state.spec.svelte.ts` drives both states
  over the realtime sources and type-checks a real `SupabaseClient` against the structural client.
- `packages/state/README.md`: source contract, migration order, per-screen checklist.

**Remaining:** 2.11 issue comments, follow-up
issues (scan, health, wizard), #427 note; #414 closes when the PR merges.

**Next command:** `gh pr view lane/relay-414-s2` — review and merge the PR into develop (closes #414).

**Open questions:** none blocking. The dōjō task view moves by re-reading on a `tasks` change (steps
have no relay table yet); #427's publisher decides whether steps get one.

**Known broken:** none from this branch. Do not run app unit tests while the tauri e2e build runs:
it re-syncs `.svelte-kit/tsconfig.json` and the suite fails with "Tsconfig not found".
