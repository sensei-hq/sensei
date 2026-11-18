# Checkpoint — 2026-11-18 (relay open items — specs landed, build next)

**Slice:** one OpenSpec change per open relay issue, then build them one by one.
Branch `lane/relay-open-items` (worktree `lanes/relay-open`).

**Done:**
- 11 changes under `openspec/changes/`, each `openspec validate --strict` clean:
  #455 assistant-catalogue, #444 host-ops, #427 dojo-realtime, #420 d87-shell (verify), #414 shared-state-sources,
  #398 crew-close-dead-lanes, #393 run-archive, #389 decision-response-form, #314 dojo-production-verification,
  #312 gate-unattended-evidence, #299 dojo-sync-files-workspaces (verify).
- Reconciled: #389 and #427 now build on #414's `RelayTaskSource`, `realtimeChannel` and dōjō alias
  (#427 adds only a `fromRelayRow` mapper plus optional `map`/`filter` on `realtimeChannel`).

**Build order:** #420, #299, #398, #312, #414, #389, #455, #393, #444, #427, #314.
- Verify-only first; #414 before #389/#427; #398 before #393; UI changes (#389, #455, #398) before #427 moves
  `app/src/lib/relay` to `packages/ui/relay`.

**Next command:** `openspec show verify-relay-d87-shell` then run its tasks (Tauri e2e, `pgrep -f "instance e2e"` first).

**Open questions:** none blocking. #314, #312 and #299 need production evidence to close (user's step).

**Known broken:** none from this branch. This worktree needed `bun install` in `app/` and `dojo/` for the hook.
