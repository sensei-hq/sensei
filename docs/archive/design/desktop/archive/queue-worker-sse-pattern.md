# Queue Workers + SSE Event Handling with Svelte Reactive State

> Design pattern for handling long-running background processes with real-time UI updates.

## Architecture Overview

```
┌─────────────┐     POST /api/action     ┌───────────────┐
│   Browser    │ ──────────────────────→  │   HTTP API     │
│  (Svelte)    │  ← {queued: true}        │  (returns      │
│              │                           │   immediately)  │
│              │     SSE /api/progress     │                │
│  EventSource │ ←─────────────────────── │  Broadcast TX  │
│              │   event stream           │       ↑        │
└─────────────┘                           └───────┼────────┘
                                                  │
                                          ┌───────┴────────┐
                                          │  Worker Pool   │
                                          │  (N threads)   │
                                          │                │
                                          │  Queue → Job   │
                                          │  Job → Events  │
                                          └────────────────┘
```

## Core Principles

### 1. API calls are fire-and-forget

Never block an HTTP response on a long-running operation. The API should:

- Accept the request
- Enqueue the work
- Return immediately with a job ID or queue position

```rust
// Daemon: return immediately, work happens in background
async fn start_job(State(state): State<AppState>, Json(body): Json<JobRequest>) -> Json<Value> {
    let pos = state.queue.enqueue(body.id, body.params).await;
    Json(json!({"ok": true, "queued": true, "position": pos}))
}
```

```typescript
// Client: don't await the result, just fire
function startJob() {
    api.startJob(params); // no await needed
}
```

### 2. Workers pull from a shared queue

Workers run in a loop, pulling the next job when available. Multiple workers can run in parallel if jobs are independent (e.g., indexing different repos).

```rust
pub async fn spawn_workers(queue: Arc<Queue>, state: AppState, n: usize) {
    for worker_id in 0..n {
        let queue = queue.clone();
        let state = state.clone();
        tokio::spawn(async move {
            loop {
                let job = queue.next_job().await; // blocks until available
                process_job(worker_id, &queue, &state, job).await;
            }
        });
    }
}
```

Key considerations:
- Each worker should own its own DB connection (not share a Mutex)
- Use `spawn_blocking` for CPU-bound work (parsing, tree-sitter, etc.)
- Workers must handle panics gracefully (catch and mark job as failed)

### 3. SSE broadcasts lifecycle + progress events

Use a `tokio::sync::broadcast` channel. Workers send events; SSE endpoint subscribes.

**Event types** (from coarse to fine):

| Event | When | Frequency | UI Impact |
|-------|------|-----------|-----------|
| `queued` | Job added to queue | Once per job | Show in queue list |
| `started` | Worker picks up job | Once per job | Move to "active" |
| `progress` | Per unit of work | High (per file) | Update progress bar |
| `completed` | Job finished | Once per job | Mark as done |
| `failed` | Job errored | Once per job | Show error + retry |

```rust
#[derive(Clone, Serialize)]
#[serde(tag = "type")]
pub enum JobEvent {
    Queued { job_id: String, position: usize },
    Started { job_id: String },
    Progress { job_id: String, current: String, done: u32, total: u32 },
    Completed { job_id: String, duration_ms: u64 },
    Failed { job_id: String, error: String },
}
```

### 4. SSE endpoint streams from broadcast receiver

```rust
async fn progress_sse(State(state): State<AppState>) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = state.queue.subscribe(); // broadcast::Receiver
    let stream = BroadcastStream::new(rx).filter_map(|result| match result {
        Ok(event) => Some(Ok(Event::default().data(serde_json::to_string(&event).unwrap()))),
        Err(_) => None, // lagged — skip
    });
    Sse::new(stream)
}
```

## Client-Side Pattern (Svelte 5)

### 5. Reactive state module (`.svelte.ts`)

Centralize all event state in a single module with `$state`:

```typescript
// indexer.svelte.ts
let _progress = $state<Map<string, ProgressEvent>>(new Map());
let _queueStatus = $state<QueueStatus>({ current: null, queued: [], recent: [] });
```

Export accessors that return the reactive state:

```typescript
export function getProgressMap(): Map<string, ProgressEvent> { return _progress; }
export function getQueueStatus(): QueueStatus { return _queueStatus; }
```

Components bind via `$derived`:

```svelte
<script>
  let progressMap = $derived(getProgressMap());
  let queueStatus = $derived(getQueueStatus());
</script>

{#each items as item}
  {@const progress = progressMap.get(item.id)}
  <!-- Svelte re-renders when map changes -->
{/each}
```

### 6. Throttle high-frequency events, pass lifecycle events immediately

Progress events (per-file) can fire hundreds of times per second. Flushing each one to `$state` causes Svelte to re-render every time, thrashing the DOM.

**Pattern: buffer + flush**

```typescript
let _pending = new Map<string, Event>();  // non-reactive buffer
let _flushTimer: ReturnType<typeof setTimeout> | null = null;

eventSource.onmessage = (event) => {
    const data = JSON.parse(event.data);

    if (data.type === 'progress') {
        // Buffer — don't update reactive state yet
        _pending.set(data.job_id, data);
        if (!_flushTimer) {
            _flushTimer = setTimeout(() => {
                _flushTimer = null;
                // Flush all at once
                const next = new Map(_progress);
                for (const [k, v] of _pending) {
                    // CRITICAL: don't overwrite completed with stale progress
                    if (next.get(k)?.type === 'completed') continue;
                    next.set(k, v);
                }
                _pending.clear();
                _progress = next; // single reactive update
            }, 300); // ~3 UI updates per second
        }
    } else {
        // Lifecycle events: update immediately
        _pending.delete(data.job_id); // clear stale buffer
        _progress = new Map(_progress).set(data.job_id, data);
    }
};
```

**Why 300ms?** Humans perceive updates at ~3-5fps for progress indicators. Below 200ms you waste CPU on invisible renders. Above 500ms it feels laggy.

### 7. Never let throttled events overwrite lifecycle events

This is the most subtle bug. The race condition:

```
t=0ms    progress(99/100, file.py)  → buffered
t=50ms   completed                  → set immediately
t=300ms  flush timer fires          → overwrites completed with 99/100 ← BUG
```

**Fix:** The flush must check if the current state is terminal before overwriting:

```typescript
if (next.get(k)?.type === 'completed' || next.get(k)?.type === 'failed') continue;
```

And lifecycle events must clear the pending buffer:

```typescript
_pending.delete(data.job_id); // prevent stale flush
```

### 8. Derive status from SSE state, not stale API data

Queue status from API polling can be stale (the worker moves to the next job before the status endpoint updates). Always prefer SSE-derived state:

```typescript
export function isActive(jobId: string): boolean {
    const p = _progress.get(jobId);
    // SSE says done? Trust it over stale queue status
    if (p?.type === 'completed' || p?.type === 'failed') return false;
    // Otherwise check queue
    if (_queueStatus.current?.id === jobId) return true;
    return p?.type === 'progress' || p?.type === 'started';
}
```

### 9. Use onChange callbacks for cross-component coordination

When one component needs to react to events handled by the state module (e.g., reload a project list when indexing completes):

```typescript
// State module
let _onChange: (() => void) | null = null;
export function onIndexChange(cb: () => void) { _onChange = cb; }

// In SSE handler (lifecycle events only)
if (data.type === 'completed' || data.type === 'started') {
    _onChange?.();
}
```

```svelte
<!-- Component -->
<script>
  onMount(() => { onIndexChange(() => loadData()); });
  onDestroy(() => { offIndexChange(); });
</script>
```

**Don't call onChange on every progress event** — it will flood the callback.

### 10. SSE reconnection

EventSource auto-reconnects on network errors, but handle edge cases:

```typescript
eventSource.onerror = () => {
    if (reconnectTimer) clearTimeout(reconnectTimer);
    reconnectTimer = setTimeout(() => {
        if (!eventSource || eventSource.readyState !== EventSource.OPEN) {
            connectSSE(port); // full reconnect
        }
    }, 2000);
};
```

## Summary: Event Flow

```
User clicks "Start" → POST /api/action → {queued: true}
                                              ↓
                                         Worker picks up
                                              ↓
         SSE "started" ←──────────── Worker emits event
              ↓
    _progress.set(id, started)   [immediate]
    _onChange?.()                 [notify components]
              ↓
         SSE "progress" ←──────── Worker emits per-item
              ↓
    _pending.set(id, progress)   [buffered]
              ↓  (300ms later)
    flush → _progress = merged   [single reactive update]
              ↓
         SSE "completed" ←─────── Worker emits final
              ↓
    _pending.delete(id)          [clear stale buffer]
    _progress.set(id, completed) [immediate]
    _onChange?.()                 [notify components]
              ↓
    Template: isActive() → false
    Template: progress.type === 'completed' → show "done"
```
