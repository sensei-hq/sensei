# Checkpoint — 2026-11-18 (relay open items — #312 slice 1 built)

**Slice:** `update-relay-gate-unattended-evidence` (#312), branch `lane/relay-312-s1`
(worktree `lanes/relay-open`), PR to develop.

**Done (tasks 1.1–4.3):**
- `gate.rs`: `read_reply` (approved / declined / unclear) replaces `decision_from_reply`;
  `answered_decision`; `gate_event` maps outcomes to `gate_raised` / `gate_passed` / `flagged`.
- `hook_gate`: an unclear reply goes through `unanswered_decision` (unattended hard-block denies);
  with `run_id`, every gated outcome is appended to `activity.run_events` (`detail.via = hook_gate`,
  no command text). A bad run id is a logged failed record; the answer never changes.
- Plugin: `gate.ts` sends `run_id` (`SENSEI_RUN_ID`, else `drivenRunId`).
- Tests: gate.rs pure, `hook_gate_tests` router (DB), plugin `gate.test.ts`,
  Tauri e2e `app/e2e/tests/relay-gate.spec.ts` (red then green).
- `openspec validate update-relay-gate-unattended-evidence --strict` clean.

**Remaining:** tasks §5 is the user's production runbook (release, a live unattended run hitting a
hard-block, then close #312). Other open relay items: #398, #414, #389, #455, #393, #444, #427, #314.

**Next command:** review and merge the #312 PR into develop.

**Open questions:** none blocking.

**Known broken:** none from this branch. `make install-debug` cleans `app/src-tauri/target`, so run
`make app-e2e-build` (not install-debug alone) before a single e2e spec.
