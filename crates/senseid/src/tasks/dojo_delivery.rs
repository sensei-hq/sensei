//! The delivery step of the `dojo_sync` tick: each tracker board's facts, built
//! once per tick and pushed per persona (spec tracker-delivery-metrics D3; #352).
//!
//! ```text
//! build_boards (once per tick, before any persona pushes)
//!     per synced sprint board:   try_lock — skipped this tick while a sync or history run holds it
//!         token ← delivery_token            sprints, tickets, board_figures (one replay)
//!         token' ← delivery_token           moved? skipped this tick, never sent half-updated
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
use crate::db::pg_store::delivery::PartKey;
use crate::db::pg_store::sync::SyncMark;
use crate::dojo::delivery::{
    BoardFacts, FactSprint, FactTicket, board_key, eligibility, facts_hash, facts_json, is_due,
    request_body, sized_body,
};
use crate::dojo_client::user_plane::{DeliveryError, SyncPlan, UserPlane};
use crate::tracker::sync::TrackerLocks;

/// The sync-state entity a board's push is recorded under, keyed by `board_key`.
const ENTITY: &str = "tracker_board";

/// The part of a board this step sends (`effort` arrives with F3).
const FACTS: &str = "facts";

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

/// Push each built board for one persona and record each failed one; answers how
/// many were sent, or — after every board has been tried — the boards that failed.
pub async fn push_delivery(
    pg: &PgStore,
    persona: &str,
    token: &str,
    plane: &dyn UserPlane,
    plan: &SyncPlan,
    boards: &Boards,
) -> Result<usize, String> {
    let mut faults = Vec::new();
    for f in &boards.failed {
        record_error(pg, persona, &f.board_key, &f.error).await;
        faults.push(format!("{}: {}", f.board_key, f.error));
    }
    let mut sent = 0;
    if !boards.built.is_empty() {
        let persona_id = pg
            .persona_id_for_slot(persona)
            .await?
            .ok_or_else(|| format!("no persona holds the slot {persona}"))?;
        for b in &boards.built {
            match push_board(pg, persona, persona_id, token, plane, plan, b).await {
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

/// Record a board's failure in the sync state; a failure to record is logged, the
/// board's own failure is still reported to the tick.
async fn record_error(pg: &PgStore, persona: &str, board_key: &str, error: &str) {
    let mark = SyncMark { entity: ENTITY, key: board_key, direction: "push", persona };
    if let Err(e) = pg.mark_sync_error(&mark, error).await {
        tracing::warn!(persona, board = %board_key, error = %e, "dojo_sync: board failure not recorded");
    }
}

async fn push_board(
    pg: &PgStore,
    persona: &str,
    persona_id: uuid::Uuid,
    token: &str,
    plane: &dyn UserPlane,
    plan: &SyncPlan,
    b: &BuiltBoard,
) -> Result<bool, String> {
    let mark = SyncMark { entity: ENTITY, key: &b.board_key, direction: "push", persona };
    let keys = pg.workspace_repo_keys(b.workspace_id).await?;
    let to = match eligibility(&plan.allowed, &keys) {
        Ok(to) => to,
        Err(why) => {
            pg.mark_sync_skipped(&mark, why.code()).await?;
            return Ok(false);
        }
    };
    let tenant = uuid::Uuid::parse_str(&to.tenant_id)
        .map_err(|e| format!("plan tenant {}: {e}", to.tenant_id))?;
    let body = request_body(&to, &b.board_key, &b.kind, &b.board_name, &b.facts);
    let bytes = match sized_body(&body, b.tickets) {
        Ok(bytes) => bytes,
        Err(t) => {
            let why = format!("delivery_board_too_large: {} tickets, {} bytes", t.tickets, t.bytes);
            pg.mark_sync_skipped(&mark, &why).await?;
            return Ok(false);
        }
    };
    let part = PartKey { tracker_id: b.tracker_id, persona_id, tenant_id: tenant, part: FACTS };
    let prior = pg.push_mark(&part).await?;
    if !is_due(prior.as_ref(), &b.hash, chrono::Utc::now()) {
        return Ok(false);
    }
    // Before sending: a push the dōjō commits but whose answer is lost still
    // leaves a row for a withdrawal to find.
    pg.mark_push_attempt(&part).await?;
    match plane.push_delivery(token, &bytes).await {
        Ok(_) => {
            pg.mark_push_acknowledged(&part, &b.hash).await?;
            pg.mark_synced(&mark, None).await?;
            Ok(true)
        }
        Err(DeliveryError::Refused { status, body }) => {
            let why = if status == 409 {
                format!("tracker_workspace_ambiguous: {body}")
            } else {
                format!("dōjō returned {status}: {body}")
            };
            pg.mark_push_refused(&part, &b.hash, &why).await?;
            pg.mark_sync_error(&mark, &why).await?;
            Ok(false)
        }
        Err(DeliveryError::Failed(e)) => {
            pg.mark_sync_error(&mark, &e).await?;
            Ok(false)
        }
    }
}
