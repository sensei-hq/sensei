# Change: Finish the fail-closed relay gate: unclear replies and a record in the run (#312)

## Why
#312 (G-SEC-4) asked for three things: fail closed for the high-risk classes, say which way each gate went in
the run's activity, and test every reply shape. The user chose option C (D28): hard-block categories fail
closed in unattended, relay-driven runs only. The first part shipped in `4fcea9deb` (v0.17.0). Two parts did
not:

- **An unclear reply still opens the gate.** `decision_from_reply` (`crates/senseid/src/dojo/gate.rs:40-54`)
  allows anything that is not an explicit deny. A garbled reply, an empty object, or the free-text reply the
  dōjō sends when the person types instead of tapping approve (`dojo/src/lib/relay/relay-inbox-map.ts:135-142`
  builds `{ text }`) all allow a hard-block. In an unattended run that is the same hole as a timeout.
- **The run's activity says nothing.** `hook_gate` (`crates/senseid/src/api/handlers/sessions.rs:387-623`)
  only writes `tracing` lines. It never appends to `activity.run_events`, and the plugin does not send the run
  id (`marketplace/plugins/sensei/hooks/gate.ts:25-33`), so the daemon could not attach a record if it tried.
  The issue's own closing condition ("the next unattended run that hits a hard-block should show the deny
  reason in its activity") cannot be met today.

## What Changes
- The plugin's gate hook sends `run_id` with `unattended`: `SENSEI_RUN_ID` for a daemon-spawned step, or the
  run drive's run id for a driven session.
- A reply is read as one of **approved**, **declined** or **unclear**. Only `verdict: "approve"` (any case) or
  `approve: true` is an approval; only `verdict: "deny"` or `approve: false` is a decline; anything else is
  unclear.
- In an unattended run, an unclear reply to a hard-block is denied, like no reply. Interactive sessions and
  allow-list gates keep failing open (D28 unchanged).
- When the body carries a run id, every gate outcome is appended to that run's `activity.run_events`, using the
  existing kinds: `gate_raised` when the gate goes to the phone, `gate_passed` on approval, `flagged` on a
  decline or an unanswered outcome. `detail` names the tool, category, outcome, why, and the reason; never the
  command text (D10).
- A Tauri e2e spec proves both against the e2e daemon.

## Impact
- Specs: `relay-tool-gate` (new capability for the relay tool gate).
- Code: `crates/senseid/src/dojo/gate.rs`, `crates/senseid/src/api/handlers/sessions.rs` (`hook_gate`),
  `marketplace/plugins/sensei/hooks/gate.ts`, `marketplace/plugins/sensei/hooks/drive.ts`, the doc comment on
  `askReply` in `dojo/src/lib/relay/relay-inbox-map.ts` (it names `decision_from_reply`), new
  `app/e2e/tests/relay-gate.spec.ts`.
- No DDL change: `sensei.run_event_kind` already has `gate_raised`, `gate_passed` and `flagged`
  (`database/ddl/enum/sensei/run_event_kind.ddl`).
- Closing #312 needs the change released and one live unattended run (runbook in tasks.md §5).
