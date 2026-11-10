//! The delivery step of the `dojo_sync` tick: each tracker board's facts, built
//! once per tick and pushed per persona (spec tracker-delivery-metrics D3; #352).
//!
//! ```text
//! build_boards (once per tick, before any persona pushes)
//!     per synced sprint board:   try_lock — skipped this tick while a sync or history run holds it
//!         token ← delivery_token            sprints, tickets, board_figures (one replay)
//!         token' ← delivery_token           moved? skipped this tick, never sent half-updated
//!         sync age (A5)                     null or moved sync? skipped; sent only without history
//!         facts, canonical hash
//! push_delivery (per persona)
//!     per built board:  eligibility (plan tenant) — not shared / ambiguous → sync state skipped
//!                       size check → too large → skipped
//!                       is_due(mark, hash)? — no → nothing
//!                       mark the attempt, THEN send
//!                       accepted|stale → acknowledged · 4xx → refused (backoff) · else → error
//! ```
//! One board's failure, its database writes included, never stops the others; each
//! is recorded in the sync state, and any reaches the tick's failures after every
//! board has been tried (D3). A failed read is an error, never "no boards" (D83).

use crate::db::pg_store::PgStore;
use crate::db::pg_store::delivery::{PartKey, WithdrawalKey};
use crate::db::pg_store::sync::SyncMark;
use crate::dojo::delivery::{
    BoardFacts, Effort, FactSprint, FactTicket, board_key, effort_from, effort_hash, effort_json,
    eligibility, facts_hash, facts_json, is_due, project_effort, request_body, sized_body,
    sprint_effort, sync_age, ticket_effort,
};
use crate::dojo_client::user_plane::{DeliveryError, SyncPlan, UserPlane};
use crate::tracker::sync::TrackerLocks;

/// The sync-state entity a board's push is recorded under, keyed by `board_key`.
const ENTITY: &str = "tracker_board";

/// The parts of a board this step sends, each with its own mark (A2).
const FACTS: &str = "facts";
const EFFORT: &str = "effort";

/// A board's facts, built once per tick for every persona to push.
#[derive(Debug, Clone)]
pub struct BuiltBoard {
    pub tracker_id: uuid::Uuid,
    pub workspace_id: uuid::Uuid,
    pub kind: String,
    pub board_name: String,
    pub board_key: String,
    pub facts: serde_json::Value,
    pub hash: String,
    pub tickets: usize,
}

/// A board whose facts could not be built this tick, and why.
#[derive(Debug, Clone)]
pub struct FailedBoard {
    pub board_key: String,
    pub error: String,
}

/// This tick's boards: those built, and those that failed to build.
#[derive(Debug, Clone, Default)]
pub struct Boards {
    pub built: Vec<BuiltBoard>,
    pub failed: Vec<FailedBoard>,
}

/// Build every synced sprint board's facts, skipping a board whose tracker is
/// locked (a sync or history run is writing it) or whose token moved while read;
/// a board that fails to build is carried as failed, and a failed board read is
/// an error.
pub async fn build_boards(
    pg: &PgStore,
    locks: &TrackerLocks,
    now: chrono::DateTime<chrono::Utc>,
) -> Result<Boards, String> {
    let trackers = pg.delivery_trackers().await?;
    let mut out = Boards::default();
    for t in trackers {
        let Some(_guard) = locks.try_lock(t.id) else {
            tracing::info!(tracker = %t.id, "dojo_sync: board busy (sync or history running); delivered next tick");
            continue;
        };
        match build_one(pg, &t, now).await {
            Ok(Some(b)) => out.built.push(b),
            Ok(None) => {
                tracing::info!(tracker = %t.id, "dojo_sync: board changed while read; delivered next tick")
            }
            Err(error) => out.failed.push(FailedBoard {
                board_key: board_key(&t.kind, &t.base_url, &t.board_ref),
                error,
            }),
        }
    }
    Ok(out)
}

async fn build_one(
    pg: &PgStore,
    t: &crate::db::pg_store::delivery::DeliveryTracker,
    now: chrono::DateTime<chrono::Utc>,
) -> Result<Option<BuiltBoard>, String> {
    let before = pg.delivery_token(t.id).await?;
    let sprints = pg.delivery_sprints(t.id).await?;
    let tickets = pg.delivery_tickets(t.id).await?;
    let figures = crate::sdlc::sprint_replay::board_figures(pg, t.id, now).await?;
    if pg.delivery_token(t.id).await? != before {
        return Ok(None);
    }
    let history = crate::tracker::capabilities_of_kind(&t.kind)?.history;
    let Some(synced_age_s) =
        sync_age(history, before.last_success_at, pg.delivery_sync_age(t.id).await?)
    else {
        return Ok(None);
    };
    let mut fact_sprints = Vec::with_capacity(sprints.len());
    for s in sprints {
        // A removed sprint has no replay and travels with no figures; a live one
        // without figures is a fault, never a made-up null.
        let sdlc = if s.removed {
            None
        } else {
            let f = figures
                .by_sprint
                .get(&s.id)
                .ok_or_else(|| format!("sprint {} has no figures", s.external_id))?;
            Some(
                serde_json::to_value(f)
                    .map_err(|e| format!("sprint {} figures: {e}", s.external_id))?,
            )
        };
        fact_sprints.push(FactSprint {
            external_id: s.external_id,
            seq: s.seq,
            name: s.name,
            state: s.state,
            start_at: s.start_at,
            end_at: s.end_at,
            complete_at: s.complete_at,
            removed: s.removed,
            sdlc,
        });
    }
    let ticket_count = tickets.len();
    let facts = BoardFacts {
        history_at: t.history_at,
        synced_age_s,
        pending_tickets: before.pending,
        predictability: figures.predictability.ok(),
        predictability_reason: figures.predictability.err().map(str::to_string),
        sprints: fact_sprints,
        tickets: tickets
            .into_iter()
            .map(|x| FactTicket {
                key: x.key,
                title: x.title,
                status_category: x.status_category,
                points: x.points,
                sprints: x.sprints,
                issue_created_at: x.issue_created_at,
            })
            .collect(),
    };
    let facts = facts_json(&facts);
    // The label the app shows for a board without a name.
    let board_name = t.board_name.clone().unwrap_or_else(|| format!("board #{}", t.board_ref));
    Ok(Some(BuiltBoard {
        tracker_id: t.id,
        workspace_id: t.workspace_id,
        kind: t.kind.clone(),
        board_key: board_key(&t.kind, &t.base_url, &t.board_ref),
        hash: facts_hash(&t.kind, &board_name, &facts),
        board_name,
        facts,
        tickets: ticket_count,
    }))
}

/// This persona's effort on one board (A2), and the candidate sessions held back
/// as not provably theirs (#240).
#[derive(Debug, Clone)]
pub struct BuiltEffort {
    pub json: serde_json::Value,
    pub hash: String,
    pub held_back_sessions: usize,
}

/// Build one persona's effort on a board over its provable sessions only: a
/// candidate session whose repository's `repo_key` is in `exclusive` (#240). Every
/// other candidate session linked to a live ticket is held back and counted.
pub async fn build_effort(
    pg: &PgStore,
    b: &BuiltBoard,
    exclusive: &std::collections::HashSet<String>,
    now: chrono::DateTime<chrono::Utc>,
) -> Result<BuiltEffort, String> {
    use crate::db::pg_store::tracker_usage::{TaskScope, TicketSession};
    use std::collections::{HashMap, HashSet};
    let keys = pg.candidate_session_keys(b.tracker_id).await?;
    let key_of = |s: &uuid::Uuid| keys.get(s).cloned().flatten();
    let provable = |s: &uuid::Uuid| key_of(s).is_some_and(|k| exclusive.contains(&k));
    let keys_of = |sessions: &mut dyn Iterator<Item = &uuid::Uuid>| {
        let mut out: Vec<String> = sessions.filter_map(&key_of).collect();
        out.sort();
        out.dedup();
        out
    };
    let live: HashMap<uuid::Uuid, String> = pg
        .tracker_task_rows(b.tracker_id, TaskScope::All, false)
        .await?
        .into_iter()
        .map(|t| (t.id, t.key))
        .collect();
    let (pairs, held): (Vec<TicketSession>, Vec<TicketSession>) = pg
        .ticket_sessions(b.tracker_id, None)
        .await?
        .into_iter()
        .filter(|p| live.contains_key(&p.task_id))
        .partition(|p| provable(&p.session_id));
    let held_back_sessions = held.iter().map(|p| p.session_id).collect::<HashSet<_>>().len();
    let mut effort = Effort::default();

    let mut tasks: Vec<uuid::Uuid> = pairs.iter().map(|p| p.task_id).collect();
    tasks.sort();
    tasks.dedup();
    let usage = crate::tracker::usage::ticket_usages_of(pg, b.tracker_id, &pairs).await?;
    let figures = crate::sdlc::ticket::ticket_figures_of(
        pg,
        b.tracker_id,
        b.workspace_id,
        &tasks,
        &pairs,
        &usage,
    )
    .await?;
    for task in &tasks {
        let (Some(f), Some(u)) = (figures.get(task), usage.get(task)) else {
            return Err(format!("ticket {} has no figures", live[task]));
        };
        let mut row = ticket_effort(f, u)?;
        row.repo_keys =
            keys_of(&mut pairs.iter().filter(|p| p.task_id == *task).map(|p| &p.session_id));
        effort.tickets.push((live[task].clone(), row));
    }

    let mut sessions: Vec<uuid::Uuid> = pairs.iter().map(|p| p.session_id).collect();
    sessions.sort();
    sessions.dedup();
    if !sessions.is_empty() {
        let t = crate::sdlc::sprint_replay::session_totals(pg, &sessions).await?;
        let mut row = project_effort(sessions.len() as i64, t.tokens, t.without_tokens, t.prompts);
        row.repo_keys = keys_of(&mut sessions.iter());
        effort.project = Some(row);
    }

    let external: HashMap<uuid::Uuid, String> = pg
        .delivery_sprints(b.tracker_id)
        .await?
        .into_iter()
        .map(|s| (s.id, s.external_id))
        .collect();
    let only: HashSet<uuid::Uuid> = keys.keys().filter(|s| provable(s)).copied().collect();
    for su in crate::tracker::usage::sprint_usages_among(pg, b.tracker_id, now, Some(&only)).await?
    {
        // A sprint without a window, or without a provable session in it, sends no row.
        let Ok(parts) = su.parts else { continue };
        if parts.linked_sprint_sessions.is_empty() {
            continue;
        }
        let id = external
            .get(&su.sprint.id)
            .ok_or_else(|| format!("sprint {} has no external id", su.sprint.id))?;
        let mut row = sprint_effort(&parts.linked_sprint)?;
        row.repo_keys = keys_of(&mut parts.linked_sprint_sessions.iter());
        effort.sprints.push((id.clone(), row));
    }

    let json = effort_json(&effort);
    Ok(BuiltEffort { hash: effort_hash(&json), json, held_back_sessions })
}

/// Push each built board for one persona and record each failed one; answers how
/// many were sent, or — after every board has been tried — the boards that failed.
/// `exclusive` is the persona's provable repositories (#240): effort comes only
/// from their sessions.
pub async fn push_delivery(
    pg: &PgStore,
    persona: &str,
    token: &str,
    plane: &dyn UserPlane,
    plan: &SyncPlan,
    exclusive: &[String],
    boards: &Boards,
) -> Result<usize, String> {
    let mut faults = Vec::new();
    for f in &boards.failed {
        record_error(pg, persona, &f.board_key, &f.error).await;
        faults.push(format!("{}: {}", f.board_key, f.error));
    }
    let mut sent = 0;
    let persona_id = pg.persona_id_for_slot(persona).await?;
    // Withdrawals first: a board that left is gone before any other is sent (B13).
    if let Some(persona_id) = persona_id {
        faults.extend(send_withdrawals(pg, persona, persona_id, token, plane).await?);
    }
    if !boards.built.is_empty() {
        let persona_id =
            persona_id.ok_or_else(|| format!("no persona holds the slot {persona}"))?;
        let ctx = PushCtx {
            pg,
            persona,
            persona_id,
            token,
            plane,
            plan,
            exclusive: exclusive.iter().cloned().collect(),
        };
        for b in &boards.built {
            match push_board(&ctx, b).await {
                Ok(true) => sent += 1,
                Ok(false) => {}
                Err(e) => {
                    record_error(pg, persona, &b.board_key, &e).await;
                    faults.push(format!("{}: {e}", b.board_key));
                }
            }
        }
    }
    if faults.is_empty() {
        Ok(sent)
    } else {
        Err(format!("{} board(s) not delivered: {}", faults.len(), faults.join("; ")))
    }
}

/// One persona's pass: who pushes, with what, and which repositories are theirs.
struct PushCtx<'a> {
    pg: &'a PgStore,
    persona: &'a str,
    persona_id: uuid::Uuid,
    token: &'a str,
    plane: &'a dyn UserPlane,
    plan: &'a SyncPlan,
    exclusive: std::collections::HashSet<String>,
}

/// Send this persona's due withdrawals; each that lands leaves the queue and marks
/// its board synced; each that does not stays with its error — a refusal waiting an
/// hour, out of the tick's faults meanwhile — and is answered as a fault after every
/// one has been tried. A failed queue read is an error, never "nothing to withdraw".
async fn send_withdrawals(
    pg: &PgStore,
    persona: &str,
    persona_id: uuid::Uuid,
    token: &str,
    plane: &dyn UserPlane,
) -> Result<Vec<String>, String> {
    let mut faults = Vec::new();
    for w in pg.delivery_withdrawals(persona_id).await? {
        let mark = SyncMark { entity: ENTITY, key: &w.board_key, direction: "push", persona };
        let sent = plane.withdraw_delivery(token, &w.board_key, w.tenant_id, persona_id).await;
        let outcome = match sent {
            Ok(()) => match pg.withdrawal_done(&w).await {
                Ok(()) => pg.mark_synced(&mark, None).await,
                Err(e) => Err(e),
            },
            Err(e) => {
                let refused = matches!(e, DeliveryError::Refused { .. });
                let why = format!("withdrawal not sent: {e}");
                record_error(pg, persona, &w.board_key, &why).await;
                pg.withdrawal_failed(&w, &why, refused).await.and(Err(why))
            }
        };
        if let Err(e) = outcome {
            faults.push(format!("{}: {e}", w.board_key));
        }
    }
    Ok(faults)
}

/// Record a board's failure in the sync state; a failure to record is logged, the
/// board's own failure is still reported to the tick.
async fn record_error(pg: &PgStore, persona: &str, board_key: &str, error: &str) {
    let mark = SyncMark { entity: ENTITY, key: board_key, direction: "push", persona };
    if let Err(e) = pg.mark_sync_error(&mark, error).await {
        tracing::warn!(persona, board = %board_key, error = %e, "dojo_sync: board failure not recorded");
    }
}

async fn push_board(ctx: &PushCtx<'_>, b: &BuiltBoard) -> Result<bool, String> {
    let pg = ctx.pg;
    let mark =
        SyncMark { entity: ENTITY, key: &b.board_key, direction: "push", persona: ctx.persona };
    let keys = pg.workspace_repo_keys(b.workspace_id).await?;
    let to = match eligibility(&ctx.plan.allowed, &keys) {
        Ok(to) => to,
        Err(why) => {
            // A board that was pushed and is no longer shared is withdrawn (B13).
            let queued = pg.queue_withdrawal(b.tracker_id, ctx.persona_id).await?;
            if queued > 0 {
                tracing::info!(persona = ctx.persona, board = %b.board_key, "dojo_sync: board no longer shared; withdrawal queued");
            }
            pg.mark_sync_skipped(&mark, why.code()).await?;
            return Ok(false);
        }
    };
    let tenant = uuid::Uuid::parse_str(&to.tenant_id)
        .map_err(|e| format!("plan tenant {}: {e}", to.tenant_id))?;
    let effort = build_effort(pg, b, &ctx.exclusive, chrono::Utc::now()).await?;
    if effort.held_back_sessions > 0 {
        tracing::info!(
            persona = ctx.persona,
            board = %b.board_key,
            held_back = effort.held_back_sessions,
            "dojo_sync: sessions held back from a board's effort — not provably this person's"
        );
    }
    let sent = effort_from(ctx.persona_id, &effort.json);
    let body = request_body(&to, &b.board_key, &b.kind, &b.board_name, &b.facts, &sent);
    let bytes = match sized_body(&body, b.tickets) {
        Ok(bytes) => bytes,
        Err(t) => {
            let why = format!("delivery_board_too_large: {} tickets, {} bytes", t.tickets, t.bytes);
            pg.mark_sync_skipped(&mark, &why).await?;
            return Ok(false);
        }
    };
    let part = |part| PartKey {
        tracker_id: b.tracker_id,
        persona_id: ctx.persona_id,
        tenant_id: tenant,
        part,
    };
    // Each part keeps its own mark; one request carries both whenever either is due (A2).
    let parts = [(part(FACTS), b.hash.as_str()), (part(EFFORT), effort.hash.as_str())];
    let now = chrono::Utc::now();
    let mut due = false;
    for (key, hash) in &parts {
        due |= is_due(pg.push_mark(key).await?.as_ref(), hash, now);
    }
    if !due {
        return Ok(false);
    }
    // Before sending: a push the dōjō commits but whose answer is lost still
    // leaves a row for a withdrawal to find.
    for (key, _) in &parts {
        pg.mark_push_attempt(key, &b.board_key).await?;
    }
    match ctx.plane.push_delivery(ctx.token, &bytes).await {
        Ok(_) => {
            for (key, hash) in &parts {
                pg.mark_push_acknowledged(key, hash).await?;
            }
            // Pushed again since a withdrawal was queued: the push stands, and the
            // withdrawal must never later remove it.
            let w = WithdrawalKey {
                board_key: b.board_key.clone(),
                tenant_id: tenant,
                persona_id: ctx.persona_id,
            };
            pg.withdrawal_done(&w).await?;
            pg.mark_synced(&mark, None).await?;
            Ok(true)
        }
        Err(DeliveryError::Refused { status, body }) => {
            let why = if status == 409 {
                format!("tracker_workspace_ambiguous: {body}")
            } else {
                format!("dōjō returned {status}: {body}")
            };
            for (key, hash) in &parts {
                pg.mark_push_refused(key, hash, &why).await?;
            }
            pg.mark_sync_error(&mark, &why).await?;
            Ok(false)
        }
        Err(DeliveryError::Failed(e)) => {
            pg.mark_sync_error(&mark, &e).await?;
            Ok(false)
        }
    }
}
