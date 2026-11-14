# Phase 3 — MCP (sensei-mcp)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Every new lane call is one MCP tool away, so a Muse/Claude lane runs the whole loop over `relay_*` tools, and the tool docs say what the daemon now does (a read no longer starts work; a report floats; a claim is atomic).

**Architecture:** `crates/mcp/src/lib.rs` has three places per tool: the dispatch arm in the big `match tool` (builds a `DaemonRequest` with `relay_lane_body` + `relay_forward` + `relay_authed`), the `tool(...)` schema entry in `handle_list_tools`, and the two name lists the tests pin (`RELAY_TOOLS` and the ordered list near line 2028). `relay_wait_for_task` stays inline on the long-timeout client.

**Tests first:** `relay_tool_list_names_the_queue_tools`, `claim_next_dispatch_posts_with_bearer_and_no_token_in_body`, `checkpoint_dispatch_forwards_current_step_notes_steps`, `release_dispatch_forwards_reason`, `create_run_is_not_a_lane_tool` (it is a person route; not exposed over MCP), `register_dispatch_forwards_suggested_name_and_omits_role_when_absent`, `wait_request_carries_timeout_seconds_when_given`.

## Task 3.1: Dispatch arms

- [x] Done (`c710a4e56` red → green commit): the four task tools forward per op (claims nothing, release `reason`, checkpoint `current_step|notes|steps`); `relay_register` forwards `suggested_name`; `build_relay_wait_request` takes `timeout_seconds` beside `timeout_ms` (both ride when both are given; the daemon reads ms first).

```rust
"relay_claim_next" => {
    let body = relay_lane_body(args);
    Some(relay_authed(DaemonRequest::post_json("/api/relay/tasks/claim_next", Value::Object(body)), args))
}
"relay_claim_task" | "relay_claim_review" | "relay_release_task" | "relay_checkpoint_task" => {
    let task_id = args["task_id"].as_str().unwrap_or("");
    let op = match tool { "relay_claim_task" => "claim", "relay_claim_review" => "claim_review",
                          "relay_release_task" => "release", _ => "checkpoint" };
    let mut body = relay_lane_body(args);
    relay_forward(&mut body, args, &["reason", "current_step", "notes", "steps"]);
    Some(relay_authed(DaemonRequest::post_json(format!("/api/relay/tasks/{}/{op}", path_segment(task_id)), Value::Object(body)), args))
}
"relay_limit_resume" => {
    let body = relay_lane_body(args);
    Some(relay_authed(DaemonRequest::post_json("/api/relay/limit_resume", Value::Object(body)), args))
}
"relay_get_queue" => {
    let lane_id = args["lane_id"].as_str().unwrap_or("");
    Some(relay_authed(DaemonRequest::get("/api/relay/queue").with_query("lane_id", lane_id), args))
}
```
`relay_register` forwards `suggested_name` too (add to its key list); `role` stays optional (absent → the daemon defaults to contributor). `build_relay_wait_request` gains `timeout_seconds: Option<u64>` and forwards whichever the caller gave.

## Task 3.2: Schemas

- [x] Done: seven entries after `relay_wait_for_task`; `relay_register` requires `family` only (`role` optional, default contributor; `crew_id` optional, the one active crew; `suggested_name`); `relay_get_tasks`, `relay_wait_for_task`, `relay_report_task`, `relay_submit_review` say what the daemon now does; `relay_assign_task` had already dropped `reviewer_lane` in 2.1.

Seven `tool(...)` entries after `relay_wait_for_task`:
- `relay_claim_next` — "Claim the next piece of work for this lane, atomically: a floating review first (never your own report), then a ready task whose dependencies are done, then a ready task you may start early (its report is gated until its dependencies are done). Answers claim_type review|task|none." Required: `lane_id`, `lane_token`.
- `relay_claim_task` — a named ready task. Required: `lane_id`, `lane_token`, `task_id`.
- `relay_claim_review` — a named floating review. Required: + `task_id`.
- `relay_checkpoint_task` — "Save in-turn progress on the task you hold without moving it: current_step, notes, and checklist steps." Required: `lane_id`, `lane_token`, `task_id`; optional `current_step`, `notes`, `steps` (array of `{key, title, state, reason?}`).
- `relay_release_task` — "Give a claim back: a held task returns to the pool (checkpoint kept); a claimed review floats again." Required: + `task_id`; optional `reason` (≤600).
- `relay_limit_resume` — "End a quota pause early. Heartbeats never end it; the sweeper ends it at resumes_at." Required: `lane_id`, `lane_token`.
- `relay_get_queue` — "Read the crew's claim queue (read-only): floating reviews, ready tasks, claim-early tasks with their unmet dependencies. Reading never claims." Required: `lane_id`, `lane_token`.

Doc edits on existing tools: `relay_register` (role optional; `crew_id` optional — joins the one active crew; `suggested_name`); `relay_get_tasks` ("Read the lane envelope … Reading never starts a task: claim with relay_claim_next"); `relay_report_task` ("… a full report floats the task on the review queue and frees the lane; 409 relay_upstream_deps_unmet while a dependency is not done"); `relay_submit_review` ("Review a report you claimed … approval hands the task to the broker's merge gate"); `relay_assign_task` drops `reviewer_lane`; `relay_wait_for_task` names `timeout_seconds`.

## Task 3.3: Name lists, docs, install

- [x] Both name lists carry the seven; `cargo test -p sensei-mcp` 100 unit + 5 integration green; clippy clean. The only doc surface that enumerated the register flow was the marketplace `session-start` hook (its contributor line now joins without a role and names the claim loop; `test-hooks.sh` pins it, 25/25) — no `SKILL.md` lists relay tools. `make install-service` + a tools/list check follow.

- Add the seven names to `RELAY_TOOLS` and the ordered list; run `cargo test -p sensei-mcp` → green.
- `make install-service`; verify with `claude mcp` (or `sensei-mcp --list-tools` if present) that the seven appear.
- Marketplace skill docs that enumerate relay tools (`marketplace/` and `docs/skills/`, both `SKILL.md` copies per the release checklist) gain the seven and the loop.
- Commit: `feat(mcp): relay queue tools — claim_next, claim_task, claim_review, checkpoint_task, release_task, limit_resume, get_queue`.
