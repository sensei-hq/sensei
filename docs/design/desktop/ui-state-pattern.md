---
name: UI State Pattern
description: Component → State → API architecture for all desktop screens — SSE event manager + load functions
date: 2026-04-27
type: design
---

# UI State Pattern

Every desktop screen follows the same three-layer architecture. No exceptions.

## Layers

```
┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│   Component  │◀────│    State     │◀────│   API Layer  │
│  (renders)   │     │ (transitions)│     │ (fetches)    │
└──────────────┘     └──────────────┘     └──────────────┘
```

### Component (`Screen.svelte`)

Pure render function. Reads from state, renders UI. No fetch calls, no business logic.

```svelte
<script lang="ts">
  import { bootstrapState } from './bootstrap-state.svelte.js';
  const state = bootstrapState;
</script>

{#each state.components as comp}
  <ComponentCard {comp} />
{/each}
```

**Test:** Feed state, assert DOM output.

### State (`*-state.svelte.ts`)

Reactive store with explicit transitions. Accepts events from the API layer, updates state.

```ts
// Explicit state transitions — no implicit side effects
let components = $state<ComponentStatus[]>([]);
let ready = $derived(components.every(c => isReady(c.state) || isSkipped(c.state)));

export function applyBootstrapResult(result: BootstrapResult) {
  components = result.components;
  hardware = result.hardware;
}

export function updateComponent(name: string, status: ComponentStatus) {
  const idx = components.findIndex(c => c.name === name);
  if (idx >= 0) components[idx] = status;
}
```

**Test:** Call transition functions, assert state values. No DOM, no network.

### API Layer (`*.ts`)

Fetch/invoke functions that return typed data. Two patterns:

## Pattern A: Load (one-shot fetch)

For page data that's fetched once on mount or navigation.

```ts
// bootstrap.ts
export async function loadBootstrap(): Promise<BootstrapResult> { ... }
export async function installComponent(name: string): Promise<ComponentStatus> { ... }
```

Used in page `+page.svelte` or `+page.ts` load function:

```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import { loadBootstrap } from '$lib/bootstrap.js';
  import { applyBootstrapResult } from './bootstrap-state.svelte.js';

  onMount(async () => {
    const result = await loadBootstrap();
    applyBootstrapResult(result);
  });
</script>
```

**Test:** Mock fetch/invoke, assert return shape.

## Pattern B: Live (SSE stream)

For real-time updates (indexing progress, model pulls, session events).

### EventManager

A generic SSE subscriber that connects to an endpoint and dispatches typed events to a handler.

```ts
// $lib/events.ts

export type EventHandler<T> = (event: T) => void;

export class EventManager<T> {
  private source: EventSource | null = null;
  private handlers: EventHandler<T>[] = [];

  constructor(private url: string, private parse: (data: string) => T) {}

  subscribe(handler: EventHandler<T>): () => void {
    this.handlers.push(handler);
    if (!this.source) this.connect();
    return () => this.unsubscribe(handler);
  }

  private unsubscribe(handler: EventHandler<T>) {
    this.handlers = this.handlers.filter(h => h !== handler);
    if (this.handlers.length === 0) this.disconnect();
  }

  private connect() {
    this.source = new EventSource(this.url);
    this.source.onmessage = (e) => {
      const parsed = this.parse(e.data);
      this.handlers.forEach(h => h(parsed));
    };
    this.source.onerror = () => {
      this.disconnect();
      // Reconnect after delay
      setTimeout(() => { if (this.handlers.length > 0) this.connect(); }, 3000);
    };
  }

  private disconnect() {
    this.source?.close();
    this.source = null;
  }

  destroy() {
    this.handlers = [];
    this.disconnect();
  }
}
```

### Usage with state

```ts
// index-state.svelte.ts
import { EventManager } from '$lib/events.js';

let progress = $state<IndexProgress>({ ... });

const events = new EventManager<IndexEvent>(
  'http://127.0.0.1:7744/api/index/progress',
  (data) => JSON.parse(data) as IndexEvent,
);

export function startListening() {
  return events.subscribe((event) => {
    // State transition based on event type
    if (event.type === 'file_indexed') {
      progress.completed += 1;
    } else if (event.type === 'repo_done') {
      progress.repos_done += 1;
    }
  });
}
```

**Test EventManager:** Feed mock events, assert handler calls.
**Test state:** Call transition functions with event payloads, assert state.

## File naming convention

```
src/lib/
├── events.ts                    ← generic EventManager
├── bootstrap.ts                 ← API layer (load pattern)
├── bootstrap-state.svelte.ts    ← state store
├── bootstrap.test.ts            ← state + API tests
└── components/
    └── BootstrapScreen.svelte   ← pure render component

src/routes/(health)/health/
└── +page.svelte                 ← wires state to component
```

## Testing matrix

| Layer | Tool | What to assert |
|-------|------|----------------|
| Component | @testing-library/svelte | Given state X, renders elements Y |
| State | vitest (plain) | Given event E in state S, transitions to S' |
| API (load) | vitest + mock fetch | Returns correct shape from mock response |
| API (live) | vitest + mock EventSource | EventManager dispatches parsed events to handlers |
| Integration | Playwright (future) | Full flow with mocked daemon |

## Rules

1. Components never call fetch/invoke directly
2. State stores never create EventSource or call fetch
3. API functions are pure — return data, no side effects
4. SSE connections always go through EventManager
5. Every screen follows the same three-layer split
6. State transitions are explicit functions, not implicit effects
