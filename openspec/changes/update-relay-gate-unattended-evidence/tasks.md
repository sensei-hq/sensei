## 1. Reply shapes (daemon, red first)
- [ ] 1.1 Red: `gate.rs` tests for `read_reply` — `{verdict:"approve"}`, `{verdict:"APPROVE"}`, `{approve:true}` → Approved; `{verdict:"deny"}`, `{approve:false}`, `{verdict:"approve", approve:false}` → Declined; `{}`, `{text:"ok"}`, `{choice:"a"}`, `{note:"x"}`, `"whatever"`, `null`, `{verdict:"maybe"}` → Unclear
- [ ] 1.2 Red: a decision test — Unclear on an unattended hard-block denies with "not a clear approve or deny" in the reason; Unclear on an interactive hard-block or an allow-list gate allows
- [ ] 1.3 Green: `read_reply` replaces `decision_from_reply`; `hook_gate` maps Approved → allow, Declined → deny, Unclear → `unanswered_decision`; update the module doc (`gate.rs:1-15`), the handler doc (`sessions.rs:371-386`) and the dōjō `askReply` doc comment that names `decision_from_reply` (`dojo/src/lib/relay/relay-inbox-map.ts:129-134`) to say so

## 2. The run record (daemon, red first)
- [ ] 2.1 Red: a pure `gate_event(outcome, tool, hard_block, why, reason) -> (RunEventKind, detail)` test per outcome (raised, approved, declined, denied, allowed); detail never holds the tool input
- [ ] 2.2 Red: `hook_gate_tests` (DB-backed, serial) — create a run, post an unattended `make deploy` with `run_id`: the answer is deny and `list_run_events` holds one `flagged` event with `outcome: "denied"`, `category: "deploy"`, the reason, and no command text
- [ ] 2.3 Red: interactive `make deploy` with `run_id` → allow and a `flagged` event with `outcome: "allowed"`; `cargo test` with `run_id` → no event; an unknown or non-uuid `run_id` → same decision as without it, no 5xx
- [ ] 2.4 Green: `hook_gate` reads `run_id`, appends via `append_run_event` on raise and on every gated outcome; a failed append is `tracing::warn!` and never changes the answer
- [ ] 2.5 `cargo fmt --all --check`, `cargo clippy -p senseid -- -D warnings`, `cargo test -p senseid --  --test-threads=1` (gate + sessions + runs), fabrication-guard and ratchet tests green

## 3. Plugin (red first)
- [ ] 3.1 Red: `gate.test.ts` — `run_id` equals `SENSEI_RUN_ID` when set; equals the drive's run id in a driven session; absent in an interactive session
- [ ] 3.2 Green: `drivenRunId($)` in `drive.ts`; `gate.ts` sends `run_id`; update the header comment

## 4. Tauri e2e proof
- [ ] 4.1 Write `app/e2e/tests/relay-gate.spec.ts` (relay spec patterns, `DAEMON_URL`): create a run with `POST /api/runs`; unattended `make deploy` with `run_id` and a fresh uuid session → `permissionDecision: "deny"` and a reason naming "unattended"; `GET /api/runs/{id}` lists a `flagged` event with `outcome: "denied"`; interactive → allow and `outcome: "allowed"`; ordinary `cargo test` → allow, no new gate event. Count only events with `detail.via == "hook_gate"`: the scheduler appends a `housekeeping` event to every active run each 15 s tick
- [ ] 4.2 Check `pgrep -f "instance e2e"` (wait if live), run `make test-app-e2e` or the single spec after the bundle builds; confirm `brew services start sensei` restored the real daemon on :7744
- [ ] 4.3 `openspec validate update-relay-gate-unattended-evidence --strict`; PR to develop; comment on #312 with the evidence (do not close yet)

## 5. Runbook for the user (production evidence, not automatable here)
- [ ] 5.1 Cut the release that carries this change (`make bump` on develop → PR to main → `make release`)
- [ ] 5.2 Start an unattended run (`start_run` / `register_plan`) whose plan includes a hard-block step, e.g. a task that would run `make deploy` in a scratch repo, with no one answering the phone gate
- [ ] 5.3 Confirm the step's tool call was refused with "… no one can answer in an unattended run: denied", and `run_status` (or `GET /api/runs/{id}`) shows the `flagged` event with `outcome: "denied"`
- [ ] 5.4 Optional: answer one hard-block from the phone with typed text instead of approve; confirm it is denied as unclear
- [ ] 5.5 Close #312 with that evidence
