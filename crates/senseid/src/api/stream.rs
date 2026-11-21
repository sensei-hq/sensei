//! Uniform SSE frames (ops-center slice 1): every stream serializes
//! [`StreamFrame`], stamped with a per-channel cursor id + emit time by
//! [`StreamLog`], which also serves reconnect replay.

use serde::Serialize;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::response::sse::{Event, KeepAlive, Sse};
use tokio::sync::broadcast;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::BroadcastStream;

use crate::api::events::{Action, FrameError, StateEvent};
use crate::tasks::progress::TaskEvent;

/// Envelope version. Daemon and app ship together, so frames never
/// negotiate versions — a bump is a coordinated migration, loud by design.
const ENVELOPE_VERSION: u8 = 1;

/// The one SSE frame every stream speaks: envelope (`v`, cursor `id`,
/// emit `ts`) over the event (`entity`, `action`, stable `key`, payload),
/// with terminal markers only when set.
#[derive(Debug, Clone, Serialize)]
pub struct StreamFrame {
    v: u8,
    id: u64,
    ts: String,
    entity: String,
    action: Action,
    key: String,
    data: serde_json::Value,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    done: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<FrameError>,
}

fn now_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

impl StreamFrame {
    /// Frame a state event with a cursor id (stamped by [`StreamLog`]).
    pub fn live(id: u64, event: StateEvent) -> Self {
        Self {
            v: ENVELOPE_VERSION,
            id,
            ts: now_rfc3339(),
            entity: event.entity,
            action: event.action,
            key: event.key,
            data: event.data,
            done: event.done,
            error: event.error,
        }
    }

    /// Map a queue task event onto the envelope: queued→add,
    /// started→update, completed→update+done, failed→update+done+error.
    /// Folder totals ride as folder adds (the denominator protocol);
    /// stages ride as job updates keyed by stage+path (stages are not
    /// queue tasks and have no id — synthesising one could collide).
    pub fn from_task_event(id: u64, event: &TaskEvent) -> Self {
        let ts = now_rfc3339();
        let frame = |entity: &str, action: Action, key: String, data: serde_json::Value| Self {
            v: ENVELOPE_VERSION,
            id,
            ts: ts.clone(),
            entity: entity.into(),
            action,
            key,
            data,
            done: false,
            error: None,
        };
        match event {
            TaskEvent::Queued { task_id } => frame(
                "job",
                Action::Add,
                task_id.to_string(),
                serde_json::json!({ "task_id": task_id }),
            ),
            TaskEvent::Started { task_id, folder_path, kind, path } => frame(
                "job",
                Action::Update,
                task_id.to_string(),
                serde_json::json!({
                    "task_id": task_id, "kind": kind,
                    "folder_path": folder_path, "path": path,
                }),
            ),
            TaskEvent::Completed { task_id, folder_path, kind } => frame(
                "job",
                Action::Update,
                task_id.to_string(),
                serde_json::json!({
                    "task_id": task_id, "kind": kind, "folder_path": folder_path,
                }),
            )
            .done(),
            TaskEvent::Failed { task_id, folder_path, kind, error } => frame(
                "job",
                Action::Update,
                task_id.to_string(),
                serde_json::json!({
                    "task_id": task_id, "kind": kind, "folder_path": folder_path,
                }),
            )
            .failed(&error_code_for(error), error),
            TaskEvent::FolderQueued { folder_path, files_total, files_expected } => frame(
                "folder",
                Action::Add,
                folder_path.clone(),
                serde_json::json!({
                    "folder_path": folder_path,
                    "files_total": files_total,
                    "files_expected": files_expected,
                }),
            ),
            TaskEvent::StageStarted { stage, path } => frame(
                "job",
                Action::Update,
                format!("stage:{stage}:{path}"),
                serde_json::json!({ "stage": stage, "path": path }),
            ),
            TaskEvent::StageCompleted { stage, path, items, capped } => frame(
                "job",
                Action::Update,
                format!("stage:{stage}:{path}"),
                serde_json::json!({
                    "stage": stage, "path": path, "items": items, "capped": capped,
                }),
            )
            .done(),
            TaskEvent::StageFailed { stage, path, error } => frame(
                "job",
                Action::Update,
                format!("stage:{stage}:{path}"),
                serde_json::json!({ "stage": stage, "path": path }),
            )
            .failed(&error_code_for(error), error),
        }
    }

    /// A replay that outran the ring, surfaced as a frame: the client
    /// falls back to a full re-read instead of assuming continuity.
    pub fn replay_gap(id: u64) -> Self {
        Self {
            v: ENVELOPE_VERSION,
            id,
            ts: now_rfc3339(),
            entity: "stream".into(),
            action: Action::Update,
            key: "replay-gap".into(),
            data: serde_json::json!({}),
            done: false,
            error: Some(FrameError {
                code: "replay-incomplete".into(),
                message: "cursor fell off the ring; re-read from scratch".into(),
            }),
        }
    }

    /// A lagged subscriber's gap, surfaced as a frame instead of a silent
    /// hole: the client re-reads from its last cursor.
    pub fn lagged(id: u64, missed: u64) -> Self {
        Self {
            v: ENVELOPE_VERSION,
            id,
            ts: now_rfc3339(),
            entity: "stream".into(),
            action: Action::Update,
            key: "lag".into(),
            data: serde_json::json!({ "missed": missed }),
            done: false,
            error: Some(FrameError {
                code: "stream-lag".into(),
                message: format!("{missed} frames dropped; re-read from last cursor"),
            }),
        }
    }

    fn done(mut self) -> Self {
        self.done = true;
        self
    }

    fn failed(mut self, code: &str, message: &str) -> Self {
        self.done = true;
        self.error = Some(FrameError { code: code.into(), message: message.into() });
        self
    }

    pub fn entity(&self) -> &str {
        &self.entity
    }

    pub fn key(&self) -> &str {
        &self.key
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    /// The crew this frame names, if any — the relay stream's narrowing key.
    pub fn payload_crew(&self) -> Option<&str> {
        self.data.get("crew_id").and_then(|v| v.as_str())
    }
}

/// Stable error code for a freeform task error string. The watchdog prefix
/// is the one machine-readable signal failure strings carry; everything
/// else is a plain task failure until reason codes systematize it.
fn error_code_for(error: &str) -> String {
    if error.starts_with("watchdog:") { "watchdog-timeout" } else { "task-failed" }.to_string()
}

/// Which entities each stream forwards, enforced daemon-side.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamKind {
    Scan,
    Relay,
    Tasks,
}

/// Per-stream allowlist. `stream` (lag/heartbeat meta) passes everywhere;
/// the scan socket carries scan entities only — relay traffic stays out.
pub fn allows_entity(kind: StreamKind, entity: &str) -> bool {
    if entity == "stream" {
        return true;
    }
    match kind {
        StreamKind::Scan => matches!(entity, "workspace" | "folder" | "activity" | "assistant"),
        StreamKind::Relay => StateEvent::is_relay_family(entity),
        StreamKind::Tasks => matches!(
            entity,
            "job"
                | "folder"
                | "schedule"
                | "sync"
                | "metric"
                | "transcript"
                | "index"
                | "library"
                | "watcher"
        ),
    }
}

/// The recent-frame ring behind one broadcast channel: assigns cursor ids,
/// stamps emit times, and replays what a reconnect missed. Bounded: when
/// the requested cursor has been evicted, `since` reports incompleteness
/// and the client falls back to a full re-read — never a silent hole.
pub struct StreamLog {
    cap: usize,
    next_id: u64,
    frames: VecDeque<StreamFrame>,
}

impl StreamLog {
    pub fn new(cap: usize) -> Self {
        Self { cap, next_id: 1, frames: VecDeque::new() }
    }

    /// Log a state event, returning the stamped frame (single stamping:
    /// the ring and the re-broadcast carry identical frames).
    pub fn push(&mut self, event: StateEvent) -> StreamFrame {
        let id = self.next_id;
        self.next_id += 1;
        let frame = StreamFrame::live(id, event);
        self.frames.push_back(frame.clone());
        while self.frames.len() > self.cap {
            self.frames.pop_front();
        }
        frame
    }

    /// Log a task event, returning the stamped frame.
    pub fn push_task(&mut self, event: &TaskEvent) -> StreamFrame {
        let id = self.next_id;
        self.next_id += 1;
        let frame = StreamFrame::from_task_event(id, event);
        self.frames.push_back(frame.clone());
        while self.frames.len() > self.cap {
            self.frames.pop_front();
        }
        frame
    }

    /// Frames after `cursor` (exclusive) plus whether the replay is
    /// complete. `false` means the cursor fell off the ring: re-read.
    pub fn since(&self, cursor: u64) -> (Vec<StreamFrame>, bool) {
        if self.frames.iter().any(|f| f.id == cursor) || cursor == 0 {
            let frames = self.frames.iter().filter(|f| f.id > cursor).cloned().collect::<Vec<_>>();
            // Cursor 0 with an empty log, or a cursor AT the oldest frame,
            // is complete only if nothing older was evicted.
            let oldest = self.frames.front().map(|f| f.id).unwrap_or(self.next_id);
            let complete = cursor >= oldest.saturating_sub(1);
            (frames, complete)
        } else if self.frames.iter().any(|f| f.id > cursor) {
            // Cursor older than the ring but frames remain: partial.
            (self.frames.iter().cloned().collect(), false)
        } else {
            // Cursor at or past the head: nothing missed.
            (Vec::new(), true)
        }
    }
}

/// Recent-frame ring capacity per channel. Reconnect replay covers brief
/// drops; anything older falls back to a full re-read (signaled, never
/// silent).
const STREAM_LOG_CAP: usize = 500;

/// Heartbeat cadence on every stream: idle connections stay visibly alive.
const HEARTBEAT_SECS: u64 = 15;

/// The framed-stream hub: recent-frame rings + re-broadcast channels behind
/// the two raw broadcasts. One field on shared state so every handler and
/// the forwarders reach streams through one path.
pub struct StreamHub {
    pub state_log: Arc<Mutex<StreamLog>>,
    pub task_log: Arc<Mutex<StreamLog>>,
    pub state_tx: broadcast::Sender<StreamFrame>,
    pub task_tx: broadcast::Sender<StreamFrame>,
}

impl StreamHub {
    pub fn new() -> Self {
        let (state_tx, _) = broadcast::channel(1024);
        let (task_tx, _) = broadcast::channel(1024);
        Self {
            state_log: Arc::new(Mutex::new(StreamLog::new(STREAM_LOG_CAP))),
            task_log: Arc::new(Mutex::new(StreamLog::new(STREAM_LOG_CAP))),
            state_tx,
            task_tx,
        }
    }
}

impl Default for StreamHub {
    fn default() -> Self {
        Self::new()
    }
}

/// Spawn the two frame loggers for the daemon's lifetime: each subscribes
/// its raw broadcast, stamps cursor ids into its ring, and re-broadcasts
/// the stamped frames handlers subscribe to.
pub fn spawn_frame_loggers(
    hub: &StreamHub,
    event_rx: broadcast::Receiver<StateEvent>,
    task_rx: broadcast::Receiver<TaskEvent>,
) {
    let log = hub.state_log.clone();
    let tx = hub.state_tx.clone();
    tokio::spawn(async move {
        let mut rx = event_rx;
        loop {
            match rx.recv().await {
                Ok(event) => {
                    if let Ok(mut log) = log.lock() {
                        let frame = log.push(event);
                        let _ = tx.send(frame);
                    }
                }
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    });
    let log = hub.task_log.clone();
    let tx = hub.task_tx.clone();
    tokio::spawn(async move {
        let mut rx = task_rx;
        loop {
            match rx.recv().await {
                Ok(event) => {
                    if let Ok(mut log) = log.lock() {
                        let frame = log.push_task(&event);
                        let _ = tx.send(frame);
                    }
                }
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    });
}

/// Parse a `Last-Event-ID` header value into a cursor. Missing or
/// malformed reads as 0 (from the ring head) — a client that cannot name
/// its cursor gets replay, not an error.
pub fn parse_cursor(headers: &axum::http::HeaderMap) -> u64 {
    headers
        .get("last-event-id")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.trim().parse::<u64>().ok())
        .unwrap_or(0)
}

fn frame_event(frame: &StreamFrame) -> Event {
    // The SSE id mirrors the frame cursor: a natively retrying EventSource
    // resumes with Last-Event-ID on its own. Reconnects that mint a fresh
    // socket replay from the ring (or fall back to a full re-read).
    Event::default()
        .id(frame.id().to_string())
        .data(serde_json::to_string(frame).unwrap_or_default())
}

/// Subscribe to a framed channel with replay: missed frames from the ring
/// (filtered), then live frames (filtered, deduped past what was sent), lag
/// mapped to gap frames. The receiver MUST be subscribed before this call so
/// nothing emitted during the ring read is lost; the id check drops the
/// overlap. Every stream answers with keep-alive heartbeats.
pub fn framed_stream(
    rx: broadcast::Receiver<StreamFrame>,
    log: Arc<Mutex<StreamLog>>,
    cursor: u64,
    filter: impl Fn(&StreamFrame) -> bool + Send + Sync + 'static,
) -> impl tokio_stream::Stream<Item = Result<Event, std::convert::Infallible>> + Send + 'static {
    let (missed, complete) = log.lock().map(|l| l.since(cursor)).unwrap_or((Vec::new(), false));
    let mut head: Vec<StreamFrame> = missed.into_iter().filter(|f| filter(f)).collect();
    let mut last_sent = head.iter().map(|f| f.id()).max().unwrap_or(cursor);
    if !complete {
        head.push(StreamFrame::replay_gap(last_sent));
    }
    let head_stream = tokio_stream::iter(
        head.into_iter().map(|f| Ok::<_, std::convert::Infallible>(frame_event(&f))),
    );
    let live = BroadcastStream::new(rx).filter_map(move |result| match result {
        Ok(frame) if frame.id() > last_sent && filter(&frame) => {
            last_sent = frame.id();
            Some(Ok(frame_event(&frame)))
        }
        Ok(_) => None,
        Err(tokio_stream::wrappers::errors::BroadcastStreamRecvError::Lagged(missed)) => {
            Some(Ok(frame_event(&StreamFrame::lagged(last_sent, missed))))
        }
    });
    head_stream.chain(live)
}

/// An SSE response with heartbeat keep-alives. Every stream uses this so an
/// idle connection stays visibly alive (and a dead one is noticed).
pub fn heartbeat<S>(
    stream: S,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, std::convert::Infallible>>>
where
    S: tokio_stream::Stream<Item = Result<Event, std::convert::Infallible>> + Send + 'static,
{
    Sse::new(stream).keep_alive(
        KeepAlive::new().interval(Duration::from_secs(HEARTBEAT_SECS)).text("stream-heartbeat"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The envelope every stream speaks: version, cursor, time, entity,
    /// action, key, payload — terminals and errors only when set.
    #[test]
    fn frame_serializes_the_uniform_envelope() {
        let event = crate::api::events::StateEvent::relay("message");
        let frame = StreamFrame::live(41, event);
        let json = serde_json::to_value(&frame).unwrap();
        assert_eq!(json["v"], 1);
        assert_eq!(json["id"], 41);
        assert!(json["ts"].as_str().unwrap().contains('T'));
        assert_eq!(json["entity"], "relay");
        assert_eq!(json["action"], "add");
        assert_eq!(json["key"], "message");
        assert_eq!(json["data"]["op"], "message");
        assert!(json.get("done").is_none() && json.get("error").is_none());
    }

    /// TaskEvent maps onto the envelope: queued→add, started→update,
    /// completed→update+done, failed→update+done+error, folder_queued→
    /// folder add with totals, stages→job updates.
    #[test]
    fn task_events_map_onto_frames() {
        use crate::tasks::progress::TaskEvent;
        let frame =
            |id, e: &TaskEvent| serde_json::to_value(StreamFrame::from_task_event(id, e)).unwrap();
        let q = frame(1, &TaskEvent::Queued { task_id: 7 });
        assert_eq!(
            (q["entity"].as_str(), q["action"].as_str(), q["key"].as_str()),
            (Some("job"), Some("add"), Some("7"))
        );
        assert!(q.get("done").is_none());
        let c = frame(
            2,
            &TaskEvent::Completed {
                task_id: 7,
                folder_path: "/c".into(),
                kind: "process_file".into(),
            },
        );
        assert_eq!((c["entity"].as_str(), c["action"].as_str()), (Some("job"), Some("update")));
        assert_eq!(c["done"], true);
        let f = frame(
            3,
            &TaskEvent::Failed {
                task_id: 7,
                folder_path: "/c".into(),
                kind: "process_file".into(),
                error: "watchdog: budget exceeded".into(),
            },
        );
        assert_eq!(f["done"], true);
        assert_eq!(f["error"]["code"], "watchdog-timeout");
        let fq = frame(
            4,
            &TaskEvent::FolderQueued {
                folder_path: "/c".into(),
                files_total: 10,
                files_expected: Some(12),
            },
        );
        assert_eq!(
            (fq["entity"].as_str(), fq["action"].as_str(), fq["key"].as_str()),
            (Some("folder"), Some("add"), Some("/c"))
        );
        assert_eq!(fq["data"]["files_total"], 10);
        let s = frame(5, &TaskEvent::StageStarted { stage: "scan_root".into(), path: "/c".into() });
        assert_eq!((s["entity"].as_str(), s["action"].as_str()), (Some("job"), Some("update")));
    }

    /// The scan socket carries scan entities only — relay traffic stays out.
    #[test]
    fn scan_allowlist_excludes_relay_traffic() {
        assert!(allows_entity(StreamKind::Scan, "workspace"));
        assert!(allows_entity(StreamKind::Scan, "folder"));
        assert!(allows_entity(StreamKind::Scan, "activity"));
        assert!(allows_entity(StreamKind::Scan, "assistant"));
        assert!(!allows_entity(StreamKind::Scan, "task"));
        assert!(!allows_entity(StreamKind::Scan, "lane"));
        assert!(allows_entity(StreamKind::Relay, "task"));
        assert!(!allows_entity(StreamKind::Relay, "folder"));
    }

    /// The ring assigns cursor ids and replays what a reconnect missed;
    /// eviction reports incompleteness instead of a silent hole.
    #[test]
    fn stream_log_replays_and_reports_eviction() {
        let mut log = StreamLog::new(2);
        let a = log.push(test_event("folder", "/a")).id();
        let b = log.push(test_event("folder", "/b")).id();
        assert_eq!((a, b), (1, 2));
        let (frames, complete) = log.since(0);
        assert_eq!(frames.len(), 2);
        assert!(complete);
        let (frames, complete) = log.since(1);
        assert_eq!(frames.len(), 1);
        assert!(complete);
        log.push(test_event("folder", "/c"));
        let (_, complete) = log.since(1);
        assert!(!complete, "id 1 evicted: the client must re-read");
    }

    fn test_event(entity: &str, key: &str) -> crate::api::events::StateEvent {
        let mut e = crate::api::events::StateEvent::relay("test");
        e.entity = entity.to_string();
        e.key = key.to_string();
        e
    }
}
