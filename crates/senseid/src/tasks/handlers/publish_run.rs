//! Relay run→relay publish bridge (P1) — federate a daemon-owned run
//! (`activity.runs`) to the Dōjō relay so Jerry can watch the build.
//!
//! The gap this closes: nothing federated `activity.runs` → `dojo.relay_sessions`.
//! The only relay writers were TodoWrite-session-keyed
//! ([`super::publish_relay_segments`]), so a `start_run` run never appeared in
//! relay. This handler is the missing bridge: per active run (each tick / on a
//! status change) it maps the run (+ its newest cadence event + — in P1's
//! plan-as-run chunk — its plan phases) onto the relay wire types via the pure
//! [`crate::dojo::relay_run_project`] core and publishes over the outbound
//! [`DojoClient`] relay seam, then persists the returned cloud session id into
//! `activity.runs.dojo_session_id`.
//!
//! **STATUS, not drive.** This is the safe half of relay: the daemon PUBLISHES
//! the run's real status + heartbeat + stall so the phone can watch. It never
//! reads a reply to advance the run — `SENSEI_RUN_DRIVE` stays OFF and is not
//! touched here.
//!
//! **Membership routing.** A run publishes to its owning membership: the one its
//! project is bound to (`sensei.projects.dojo_id`), if any; otherwise it falls
//! back to every enabled membership (the personal-beta case = one). A
//! per-membership failure is logged (never swallowed — house rule) and skipped so
//! one down dojo can't wedge the others.
//!
//! **Zero-knowledge (D10):** only logical status crosses (status, progress,
//! phase/feature labels, timestamps) — never code, diffs, or tool output. The
//! mapping is the pure, unit-tested [`crate::dojo::relay_run_project`].

use super::super::executor::TaskContext;
use super::super::Task;
use crate::db::pg_store::DojoMembership;
use crate::dojo::client::DojoClient;
use crate::dojo::relay_nudge::pickup_nudges;
use crate::dojo::relay_run_project::{
    plan_events_to_segments, run_to_session_update, segment_progress,
};
use crate::runs::Run;

/// How many of a run's newest cadence events to read for the plan→segments
/// projection. The outline is a handful of phases; this bound keeps the read
/// cheap while covering every phase of a realistic run (each phase emits only a
/// couple of phase-transition events).
const OUTLINE_EVENTS_LIMIT: i64 = 200;

/// Handler for `TaskKind::PublishRun`: federate one run's status to every owning
/// Dōjō membership. Returns the number of memberships successfully published to.
///
/// Short-circuits to `Ok(0)` (empty work, not an error) when there is nothing to
/// publish: no run id, a non-UUID run, a run that no longer exists, or no
/// membership to publish to.
pub async fn publish_run(ctx: &TaskContext, task: &Task) -> Result<u32, String> {
    let run_id_str = task.path.as_str();
    if run_id_str.is_empty() {
        return Ok(0);
    }
    let Ok(run_id) = uuid::Uuid::parse_str(run_id_str) else {
        return Ok(0);
    };

    // A run completed/deleted between enqueue and dispatch is empty work.
    let Some(run) = ctx
        .pg()
        .get_run(&run_id)
        .await
        .map_err(|e| format!("get_run failed: {e}"))?
    else {
        return Ok(0);
    };

    let memberships = resolve_run_memberships(ctx.pg(), &run).await?;
    if memberships.is_empty() {
        return Ok(0);
    }

    // The run's newest cadence events → the plan outline (phases → segments) +
    // the "last progress N min ago" timestamp. `list_run_events` is newest-first,
    // so the first row (if any) is the latest event.
    let events = ctx
        .pg()
        .list_run_events(&run_id, OUTLINE_EVENTS_LIMIT)
        .await
        .map_err(|e| format!("list_run_events failed: {e}"))?;
    let last_event_at = events.first().map(|e| e.created_at.clone());

    // Authored-vs-derived outline (AR-2). A run SEEDED from a registered plan
    // carries its authored graph in `activity.runs.plan_graph` — project THAT
    // (phase→task structure + per-task agent/model/spec_ref + state). An ad-hoc /
    // start_run run has no graph, so derive the outline from its cadence phases
    // (`phase_started`/`phase_done` events). A malformed/failed graph read
    // FAILS OPEN to the derived outline (a wedged read must never blank the feed).
    let derived = || {
        let segs = plan_events_to_segments(&events);
        let (d, t) = segment_progress(&segs);
        (segs, d, t)
    };
    let (segments, progress_done, progress_total) = match ctx.pg().run_plan_graph(&run_id).await {
        Ok(Some(raw)) => match serde_json::from_value::<crate::plan_graph::PlanGraph>(raw) {
            Ok(graph) => {
                let (d, t) = crate::plan_graph::task_progress(&graph);
                (crate::plan_graph::plan_to_segments(&graph), d, t)
            }
            Err(e) => {
                tracing::warn!(run_id = %run_id, error = %e, "publish_run: plan_graph parse failed; using cadence-derived outline");
                derived()
            }
        },
        Ok(None) => derived(),
        Err(e) => {
            tracing::warn!(run_id = %run_id, error = %e, "publish_run: plan_graph read failed; using cadence-derived outline");
            derived()
        }
    };

    let mut update =
        run_to_session_update(&run, last_event_at.as_deref(), progress_done, progress_total);
    // Seat attribution (P4): tell the Worker which project this run is on so it can
    // open/refresh the caller's billing seat. Best-effort — a missing project or
    // namespace just means no seat is touched; never fail federation over it.
    update.project_slug = ctx.pg().run_project_slug(&run_id).await.ok().flatten();

    // The run's project → the dōjō `dojo.projects` display row (so the user sees
    // their own projects). Best-effort: a missing project just skips the block,
    // never fails federation (mirrors project_slug). classification is per-recipient
    // — the project's BOUND membership gets its kind's classification (an org row,
    // tenant-scoped); any other recipient (an unbound run fanned out to all the
    // user's dōjōs) gets `personal` (the dōjō then stores it tenant-less), so the
    // unique(user_id, slug) row can't race across tenants.
    let project_info = ctx.pg().run_project_info(&run_id).await.ok().flatten();
    let bound_membership_id = match run.project_id {
        Some(pid) => ctx.pg().project_bound_membership(&pid).await.ok().flatten(),
        None => None,
    };

    // The project's resolved constitution → federated so the dōjō can DISPLAY the
    // "before you start" preview without re-resolving (F4). Composed ONCE from the
    // run's folder (identical for every recipient). Best-effort: a missing folder
    // or a resolve error just leaves it absent (the dōjō falls back to its
    // "resolves in your editor" state), never fails federation.
    let constitution = match ctx.pg().run_folder_id(&run_id).await.ok().flatten() {
        Some(folder_id) => ctx
            .pg()
            .resolve_repo_raw_local(&folder_id)
            .await
            .ok()
            .map(crate::dojo::relay_constitution::compose_constitution),
        None => None,
    };

    let mut published = 0u32;
    // Persist the cloud session id once — from the first membership that acks it.
    let mut persisted = run.dojo_session_id.is_some();
    for m in &memberships {
        update.project = project_info.as_ref().map(|(slug, name)| {
            let classification = if Some(m.id) == bound_membership_id {
                crate::dojo::relay_run_project::kind_to_classification(&m.kind)
            } else {
                "personal"
            };
            dojo_protocol::relay::RelayProjectInfo {
                slug: slug.clone(),
                name: name.clone(),
                classification: classification.to_string(),
                phase: "watch".to_string(),
                constitution: constitution.clone(),
            }
        });
        let client = DojoClient::for_membership(m);
        let session_id = match client.publish_session_update_returning_id(&update).await {
            Ok(session_id) => session_id,
            Err(e) => {
                tracing::warn!(run_id = %run_id, membership = %m.id, error = %e, "publish_run: session publish failed");
                continue;
            }
        };
        if !persisted {
            match uuid::Uuid::parse_str(&session_id) {
                Ok(sid) => {
                    if let Err(e) = ctx.pg().set_run_dojo_session_id(&run_id, &sid).await {
                        // Non-fatal: the publish succeeded; only the local join-id
                        // write failed. Log and keep going — the next tick retries.
                        tracing::warn!(run_id = %run_id, error = %e, "publish_run: persisting dojo_session_id failed");
                    } else {
                        persisted = true;
                    }
                }
                Err(e) => {
                    tracing::warn!(run_id = %run_id, session_id, error = %e, "publish_run: Worker returned a non-uuid session id");
                }
            }
        }
        // Upsert the plan outline (idempotent, keyed by session+seq). A segment
        // failure is logged + skipped: the status snapshot already published, so
        // the run still surfaces even if the outline lags a tick.
        if !segments.is_empty()
            && let Err(e) = client.upsert_segments(&run_id.to_string(), &segments).await
        {
            tracing::warn!(run_id = %run_id, membership = %m.id, error = %e, "publish_run: plan segment upsert failed");
        }
        published += 1;
    }

    // (e) Nudge pickup — STEER, not drive. Poll the run's owning membership inbox
    // and SURFACE the human→agent nudges/chats for this run (log). We deliberately
    // do NOT consume them to advance the run: SENSEI_RUN_DRIVE stays OFF; this is
    // a manual-steer signal + observability seam only. Best-effort: a poll failure
    // is logged and skipped (it must never wedge the status publish above).
    surface_run_nudges(&run_id, &memberships).await;

    Ok(published)
}

/// Poll the run's owning membership inbox and log the human→agent steer messages
/// for this run. STEER, not drive: it surfaces the human's nudge/chat, never
/// consumes it to advance the run. Uses the FIRST resolved membership (the run's
/// owner; personal beta = one). Best-effort — any failure is logged and swallowed
/// so the nudge poll can never wedge the status publish that already ran.
///
/// There is no run-scoped inbox cursor (no new DDL), so this polls from `0` and
/// logs the surfaced nudges each tick; the run loop's consumer of these is a
/// later, cursor-backed step. STATUS/steer only.
async fn surface_run_nudges(run_id: &uuid::Uuid, memberships: &[DojoMembership]) {
    // Steer must come from THIS run's dōjō. A bound run resolves to exactly one
    // membership; an unbound run with several enabled dōjōs is ambiguous — DON'T
    // poll an arbitrary tenant's inbox (that could surface another org's message
    // as this run's steer). Skip on ambiguity; bind the project to scope steer.
    let m = match crate::resolution::Resolution::from_unique(memberships.iter()) {
        crate::resolution::Resolution::Resolved(m) => m,
        crate::resolution::Resolution::Ambiguous { count } => {
            tracing::debug!(run_id = %run_id, count, "publish_run: {count} enabled memberships for an unbound run — skipping steer poll (ambiguous dōjō)");
            return;
        }
        crate::resolution::Resolution::Unresolved => return,
    };
    let client = DojoClient::for_membership(m);
    match client.poll_inbox(0).await {
        Ok(pull) => {
            for n in pickup_nudges(&pull.items, &run_id.to_string()) {
                // Logical only (kind + the human's short message) — never code.
                tracing::info!(
                    run_id = %run_id,
                    inbox_id = n.id.as_deref().unwrap_or(""),
                    kind = %n.kind,
                    "relay nudge for run (steer): {}",
                    n.text
                );
            }
        }
        Err(e) => {
            tracing::warn!(run_id = %run_id, membership = %m.id, error = %e, "publish_run: nudge poll failed (steer, non-fatal)");
        }
    }
}

/// The memberships a run federates to: its project-bound membership if the
/// project is bound (`sensei.projects.dojo_id`), otherwise every enabled
/// membership. Only enabled memberships are ever returned. DB errors bubble up;
/// an unbound / unknown project cleanly falls back.
pub(crate) async fn resolve_run_memberships(
    pg: &crate::db::pg_store::PgStore,
    run: &Run,
) -> Result<Vec<DojoMembership>, String> {
    let all_enabled = || async {
        Ok::<_, String>(
            pg.list_dojo_memberships()
                .await
                .map_err(|e| format!("list_dojo_memberships failed: {e}"))?
                .into_iter()
                .filter(|m| m.enabled)
                .collect::<Vec<_>>(),
        )
    };

    let Some(project_id) = run.project_id else {
        return all_enabled().await;
    };
    let bound = pg
        .project_bound_membership(&project_id)
        .await
        .map_err(|e| format!("project_bound_membership failed: {e}"))?;
    let Some(membership_id) = bound else {
        return all_enabled().await;
    };
    // The project is bound — publish only to that membership, and only if it is
    // still enabled (a disabled binding falls back to nothing, not to broadcast:
    // an explicit binding is a deliberate scoping the owner shouldn't have
    // silently widened).
    match pg
        .get_dojo_membership(&membership_id)
        .await
        .map_err(|e| format!("get_dojo_membership failed: {e}"))?
    {
        Some(m) if m.enabled => Ok(vec![m]),
        _ => Ok(vec![]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::state::SharedState;
    use crate::runs::NewRun;
    use crate::tasks::queue::TaskQueue;
    use crate::tasks::TaskKind;
    use std::sync::Arc;

    async fn make_ctx() -> Option<Arc<TaskContext>> {
        let queue = Arc::new(TaskQueue::new());
        let gateway = crate::api::gateway_init::init_gateway_test().await;
        let pg = crate::db::pg_store::PgStore::connect_test().await.ok()?;
        let app_state = Arc::new(SharedState {
            task_queue: queue.clone(),
            pg,
            gateway,
            event_tx: { let (tx, _) = tokio::sync::broadcast::channel(16); tx },
            breaker: std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
            provisioning: None,
        });
        Some(Arc::new(TaskContext { queue, app_state, _graph_path: None, logger: sensei_logger::Logger::noop() }))
    }

    async fn del_run(pg: &crate::db::pg_store::PgStore, id: &uuid::Uuid) {
        sqlx_core::query::query("DELETE FROM activity.runs WHERE id = $1")
            .bind(id).execute(pg.pool()).await.unwrap();
    }

    #[tokio::test]
    async fn empty_run_id_is_empty_work() {
        let Some(ctx) = make_ctx().await else { return; };
        let task = Task::new(TaskKind::PublishRun, "", "");
        assert_eq!(publish_run(&ctx, &task).await.unwrap(), 0);
    }

    #[tokio::test]
    async fn non_uuid_run_id_is_empty_work() {
        let Some(ctx) = make_ctx().await else { return; };
        let task = Task::new(TaskKind::PublishRun, "", "not-a-uuid");
        assert_eq!(publish_run(&ctx, &task).await.unwrap(), 0);
    }

    #[tokio::test]
    async fn unknown_run_is_empty_work() {
        let Some(ctx) = make_ctx().await else { return; };
        let id = uuid::Uuid::new_v4().to_string();
        let task = Task::new(TaskKind::PublishRun, "", &id);
        assert_eq!(publish_run(&ctx, &task).await.unwrap(), 0);
    }

    #[tokio::test]
    async fn known_run_with_no_dojo_is_zero() {
        // A real run but (typically) no enrolled dojo in the test DB → empty work,
        // not an error. Exercises the get_run + membership-resolve path end to end.
        let Some(ctx) = make_ctx().await else { return; };
        let pg = ctx.pg();
        let id = pg.create_run(&NewRun::default()).await.unwrap();
        let task = Task::new(TaskKind::PublishRun, "", &id.to_string());
        // With no enabled membership, resolve returns empty → Ok(0). (If a dev DB
        // happens to have an enrolled dojo, the publish would attempt a network
        // call to it; the test DB has none, so this is a clean 0.)
        let n = publish_run(&ctx, &task).await.unwrap();
        assert_eq!(n, 0, "no enrolled dojo ⇒ nothing published");
        del_run(pg, &id).await;
    }

    // Full-path bridge test against an axum stub Worker: seed a project-bound
    // membership pointing at the stub, create a run with phase events, run the
    // handler, and assert it publishes the status + plan segments, polls the inbox
    // for nudges, and persists the cloud session id. Mirrors the client.rs relay
    // stub tests. DB + macOS-keychain guarded.
    #[tokio::test]
    #[cfg_attr(not(target_os = "macos"), ignore)]
    async fn full_bridge_publishes_status_segments_and_persists_session_id() {
        use crate::db::pg_store::NewDojoMembership;
        use crate::runs::RunEventKind;
        use axum::{extract::Query, routing::get, routing::post, Json, Router};
        use dojo_protocol::relay::{RelayInboxPull, RelaySegmentsPublish, RelaySessionUpdate};
        use std::collections::HashMap;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        let Some(ctx) = make_ctx().await else { return; };
        let pg = ctx.pg();

        // Stub Worker: session returns {id}, segments asserts the projected plan,
        // inbox returns one human→agent nudge for the run.
        let seg_hits = Arc::new(AtomicUsize::new(0));
        let seg_hits2 = seg_hits.clone();
        async fn session(Json(u): Json<RelaySessionUpdate>) -> Json<serde_json::Value> {
            // The bridge published a real status + a heartbeat + a progress rollup.
            assert_eq!(u.progress_total, 2, "two plan phases projected");
            assert_eq!(u.progress_done, 1, "one phase done");
            assert!(u.heartbeat_at.is_some(), "heartbeat crossed the wire");
            Json(serde_json::json!({ "id": "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa" }))
        }
        let app = Router::new()
            .route("/v1/t/{origin}/{org}/relay/session", post(session))
            .route(
                "/v1/t/{origin}/{org}/relay/segments",
                post(move |Json(s): Json<RelaySegmentsPublish>| {
                    let hits = seg_hits2.clone();
                    async move {
                        hits.fetch_add(1, Ordering::SeqCst);
                        assert_eq!(s.segments.len(), 2, "P1 + P2 phase segments");
                        assert_eq!(s.segments[0].title, "P1");
                        axum::http::StatusCode::OK
                    }
                }),
            )
            .route(
                "/v1/t/{origin}/{org}/relay/inbox",
                get(|Query(_q): Query<HashMap<String, String>>| async {
                    Json(RelayInboxPull { items: vec![], cursor: 0 })
                }),
            );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap(); });
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        // Seed a membership pointing at the stub, with a keychain token.
        let mid = uuid::Uuid::new_v4();
        let cref = format!("dojo-bridge-{}", uuid::Uuid::new_v4());
        crate::gateway_keys::set_key(&cref, "device-token-bridge").unwrap();
        pg.create_dojo_membership(&NewDojoMembership {
            id: mid,
            registry_url: format!("http://{addr}"),
            tenant_key: "github/acme".into(),
            dojo_url: format!("http://{addr}/github/acme"),
            kind: "personal".into(),
            org_slugs: vec![],
            role: "contributor".into(),
            authenticated_via: "device_code".into(),
            attribution_default: "anonymous".into(),
            credential_ref: cref.clone(),
            sync_status: "authenticating".into(),
        })
        .await
        .unwrap();

        // A run (no project → falls back to all enabled memberships = our stub).
        let id = pg.create_run(&NewRun::default()).await.unwrap();
        // Stamp a heartbeat (as the AdvanceRun tick would) so the bridge has a
        // liveness instant to federate — a fresh run is NULL until first ticked.
        pg.touch_run_heartbeat(&id).await.unwrap();
        // Plan phases via cadence events: P1 started+done, P2 started.
        pg.append_run_event(&id, RunEventKind::PhaseStarted, Some("P1"), None, &serde_json::json!({})).await.unwrap();
        pg.append_run_event(&id, RunEventKind::PhaseDone, Some("P1"), None, &serde_json::json!({})).await.unwrap();
        pg.append_run_event(&id, RunEventKind::PhaseStarted, Some("P2"), None, &serde_json::json!({})).await.unwrap();

        let task = Task::new(TaskKind::PublishRun, "", &id.to_string());
        let n = publish_run(&ctx, &task).await.unwrap();
        assert_eq!(n, 1, "published to the one enrolled membership");
        assert!(seg_hits.load(Ordering::SeqCst) >= 1, "plan segments were upserted");

        // The cloud session id was persisted onto the run.
        let run = pg.get_run(&id).await.unwrap().unwrap();
        assert_eq!(
            run.dojo_session_id.map(|u| u.to_string()).as_deref(),
            Some("aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa"),
            "the Worker's session id is persisted"
        );

        del_run(pg, &id).await;
        sqlx_core::query::query("DELETE FROM sensei.dojo_memberships WHERE id = $1")
            .bind(mid).execute(pg.pool()).await.unwrap();
        crate::gateway_keys::delete_key(&cref).unwrap();
    }

    // A run seeded WITH an authored plan graph federates the AUTHORED outline (the
    // authored-vs-derived branch, AR-2): publish_run projects plan_graph → segments
    // (phase + tasks) carrying per-task agent/model, NOT the cadence-derived phases.
    // Captures the published segments and asserts them in the test thread (robust —
    // an assert inside the stub handler would be swallowed by axum's panic catch).
    #[tokio::test]
    #[cfg_attr(not(target_os = "macos"), ignore)]
    async fn full_bridge_authors_plan_graph_segments() {
        use crate::db::pg_store::NewDojoMembership;
        use axum::{extract::Query, routing::get, routing::post, Json, Router};
        use dojo_protocol::relay::{RelayInboxPull, RelaySegment, RelaySegmentsPublish, RelaySessionUpdate};
        use std::collections::HashMap;
        use std::sync::{Arc, Mutex};

        let Some(ctx) = make_ctx().await else { return; };
        let pg = ctx.pg();

        let captured: Arc<Mutex<Vec<RelaySegment>>> = Arc::new(Mutex::new(Vec::new()));
        let captured2 = captured.clone();
        async fn session(Json(_u): Json<RelaySessionUpdate>) -> Json<serde_json::Value> {
            Json(serde_json::json!({ "id": "bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb" }))
        }
        let app = Router::new()
            .route("/v1/t/{origin}/{org}/relay/session", post(session))
            .route(
                "/v1/t/{origin}/{org}/relay/segments",
                post(move |Json(s): Json<RelaySegmentsPublish>| {
                    let cap = captured2.clone();
                    async move {
                        cap.lock().unwrap().extend(s.segments.clone());
                        axum::http::StatusCode::OK
                    }
                }),
            )
            .route(
                "/v1/t/{origin}/{org}/relay/inbox",
                get(|Query(_q): Query<HashMap<String, String>>| async {
                    Json(RelayInboxPull { items: vec![], cursor: 0 })
                }),
            );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap(); });
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        let mid = uuid::Uuid::new_v4();
        let cref = format!("dojo-authored-{}", uuid::Uuid::new_v4());
        crate::gateway_keys::set_key(&cref, "device-token-authored").unwrap();
        pg.create_dojo_membership(&NewDojoMembership {
            id: mid,
            registry_url: format!("http://{addr}"),
            tenant_key: "github/acme".into(),
            dojo_url: format!("http://{addr}/github/acme"),
            kind: "personal".into(),
            org_slugs: vec![],
            role: "contributor".into(),
            authenticated_via: "device_code".into(),
            attribution_default: "anonymous".into(),
            credential_ref: cref.clone(),
            sync_status: "authenticating".into(),
        })
        .await
        .unwrap();

        let graph = serde_json::json!({
            "phases": [{ "title": "Build", "tasks": [
                { "id": "t1", "title": "one", "agent": "general-purpose", "model": "sonnet" },
                { "id": "t2", "title": "two", "model": "opus", "deps": ["t1"] }
            ]}]
        });
        let id = pg
            .create_run(&NewRun { plan_graph: Some(graph), ..Default::default() })
            .await
            .unwrap();
        pg.touch_run_heartbeat(&id).await.unwrap();

        let task = Task::new(TaskKind::PublishRun, "", &id.to_string());
        let n = publish_run(&ctx, &task).await.unwrap();
        assert_eq!(n, 1, "published to the one enrolled membership");

        let segs = captured.lock().unwrap().clone();
        assert_eq!(segs.len(), 3, "authored outline = phase + 2 tasks, got {segs:?}");
        assert_eq!(segs[0].title, "Build");
        assert!(segs[0].agent.is_none(), "a phase carries no agent");
        assert_eq!(segs[1].title, "one");
        assert_eq!(segs[1].agent.as_deref(), Some("general-purpose"));
        assert_eq!(segs[1].model.as_deref(), Some("sonnet"));
        assert_eq!(segs[2].title, "two");
        assert_eq!(segs[2].model.as_deref(), Some("opus"), "per-task model rode the wire");

        del_run(pg, &id).await;
        sqlx_core::query::query("DELETE FROM sensei.dojo_memberships WHERE id = $1")
            .bind(mid).execute(pg.pool()).await.unwrap();
        crate::gateway_keys::delete_key(&cref).unwrap();
    }
}
