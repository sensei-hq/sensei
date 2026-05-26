# App -- Desktop Observatory

## Overview

Tauri 2.x desktop app with a SvelteKit webview. The app is not a code editor, not a project manager, and not a vanity-metrics dashboard. It is a **development session observatory** -- a system that watches how AI-assisted development is going and surfaces actionable insights at three levels: global (across all projects), project (across repos in a product), and repo (per-repository detail).

Every screen connects to one of three quality signals: FTR (first-time-right rate), efficiency (turns and tokens), or correctness (pattern compliance). If a piece of data does not serve one of these, it does not belong.

The app contains zero business logic. All data comes from the daemon HTTP API (`senseid` on port 7744 release / 7745 dev). The app renders, navigates, and dispatches user intent -- nothing more.

See [ideas/01-bootstrap](../ideas/01-bootstrap.md), [ideas/02-setup](../ideas/02-setup.md), [ideas/03-observatory](../ideas/03-observatory.md), [ideas/04-project](../ideas/04-project.md), [ideas/05-gateway](../ideas/05-gateway.md), [ideas/06-logging](../ideas/06-logging.md) for the "what."

---

## Architecture

```
Tauri 2.x binary
├── src-tauri/          Rust backend
│   ├── main.rs         Tauri entry, window management
│   ├── commands/       Tauri invoke commands (bootstrap only)
│   └── Cargo.toml      depends on sensei-bootstrap crate
│
└── src/                SvelteKit webview
    ├── routes/
    │   ├── (config)/setup/    Bootstrap + wizard
    │   └── (app)/             Observatory + project
    ├── lib/
    │   ├── events.ts          Generic EventManager (SSE)
    │   ├── api/               HTTP fetch to daemon
    │   └── stores/            $state stores per screen
    └── app.html
```

Two communication paths exist, used at different lifecycle stages:

| Path | When | Mechanism |
|------|------|-----------|
| **Tauri invoke** | Bootstrap (daemon not yet running) | In-process Rust via `sensei-bootstrap` crate |
| **HTTP fetch** | Setup wizard + observatory (daemon running) | `fetch()` to `http://127.0.0.1:7744/api/*` |

The health screen is the transition point. Once `allReady`, all subsequent data flows through the daemon HTTP API.

---

## Sidecar lifecycle

The Tauri binary manages the daemon process as a sidecar. The `sensei-bootstrap` crate (a library, not a binary) is compiled into the Tauri backend and exposed as Tauri commands.

### Start

On app launch, the sidecar checks whether the daemon is reachable:

1. `GET http://127.0.0.1:{port}/health` -- fast path if daemon is already running.
2. If unreachable, invoke `run_bootstrap()` which runs the six-gate check sequence.
3. If all prerequisites pass, `start_services()` spawns the daemon: `senseid start --port 7744` in dev mode:`senseid-dev start --port 7745` ).
4. The daemon daemonizes (spawns itself, parent exits).

### Health poll

After starting the daemon, the sidecar polls the port for up to 30 seconds (`ServiceStartFixer`). Poll interval is 500ms. If the daemon binds successfully, the health check returns `200` and the app transitions.

### Restart on crash

If the daemon becomes unreachable during app use (network error on any API call), the app falls back to the health screen and re-runs the bootstrap sequence. The sidecar detects the daemon is down and restarts it.

### Graceful shutdown

On app quit (`window.close`), the Tauri backend sends a shutdown signal. The daemon flushes pending writes and exits.

### Platform considerations

| Concern | macOS (current) | Windows/Linux (future) |
|---------|-----------------|------------------------|
| Package manager | Homebrew | Scoop / apt |
| Service management | `brew services` | OS-native service manager |
| Daemon start | `senseid start` | Same binary, platform flag |
| Database | `brew install postgresql@17` | Platform installer |

The `PlatformProvider` trait in the bootstrap crate abstracts these differences. Currently only `MacOSProvider` (Homebrew-based) is implemented.

---

## Bootstrap flow

Bootstrap is a fully automated six-gate sequence that runs on every app launch. On a warm start (everything healthy), it completes in under 2 seconds and auto-advances. On a cold start with missing prerequisites, it guides the user through resolution.

### Six gates (checked in parallel)

| # | Gate | Check | Remedy |
|---|------|-------|--------|
| 1 | `homebrew` | `brew --version` | Manual install (requires sudo) |
| 2 | `postgres` | Binary present + port 5432 | `brew install postgresql@17` + `brew services start` |
| 3 | `ollama` | Binary present + port 11434 | `brew install ollama` + `brew services start` |
| 4 | `sensei` | `sensei --version` matches expected | `brew upgrade/install sensei-hq/tap/sensei` |
| 5 | `database` | DB exists + pgvector extension + sensei schema | `createdb` + `CREATE EXTENSION vector` + `dbd deploy` |
| 6 | `senseid` | Port 7744 (prod) / 7745 (dev) responds | `senseid start` |

### ComponentStatus types

```rust
pub enum ComponentState {
    Detecting,
    Installing,
    Starting,
    Upgrading,
    Pulling { progress_pct: u8, size_mb: u32 },
    Ready,
    Failed { error: String },
    Skipped,
}

pub struct ComponentStatus {
    pub name: String,
    pub state: ComponentState,
    pub version: Option<String>,
    pub detail: Option<String>,
}

pub struct BootstrapResult {
    pub components: Vec<ComponentStatus>,
    pub hardware: HardwareInfo,
    pub ready: bool,
}
```

### Parallel checks and phase execution

All five gates (postgres, ollama, sensei, database, daemon) are checked in parallel by the health provider's `check_all()` method. The result is returned synchronously to the frontend. If prerequisites are missing, the per-component install resolvers (`PostgresInstallResolver`, `OllamaInstallResolver`, `SenseiInstallResolver`) attempt `brew install` for each one independently — a failure in any single prerequisite stays isolated and doesn't block the others. Services are started in parallel after install.

### API (Tauri commands)

| Command | Function | Purpose |
|---------|----------|---------|
| `run_bootstrap` | `bootstrap::run_with_traces()` | Parallel gate checks, returns all statuses |
| `install_prerequisites` | `factory::install_prerequisites()` | per-component resolvers via `PlatformProvider::resolvers()` |
| `start_services` | `factory::start_services()` | Start postgres, ollama, daemon |
| `setup_database` | `factory::setup_database()` | Create DB + extensions + `dbd deploy` |
| `get_platform` | `provider()` | Package manager info + remedies |

All phase commands spawn a background thread and emit progress events on the `"bootstrap"` Tauri channel. The frontend listens via `listenBootstrapEvents()`.

### SSE progress events

```typescript
// Gate progress
{ action: "update", entity: "gate", id: "postgresql",
  data: { status: "checking" | "ready" | "blocked", version?, detail? } }

// Phase complete
{ action: "set", entity: "phase", id: "services",
  data: { complete: true, success: true } }
```

### Cold-start timing

```
T+0s    onMount -> run_bootstrap() -> statuses set
T+1s    startServices() -> spawns thread
T+1s    $effect -> setupDatabase() -> spawns thread
T+3s    postgres/ollama start
T+5-30s createdb + extensions + dbd deploy
T+30s   database=ready
T+30s   $effect -> startServices() again
T+32s   daemon connects to DB, binds port
T+34s   senseid=ready -> allReady -> navigate
```

---

## Setup wizard

The setup wizard runs once after bootstrap completes on a fresh install. `appState.setupComplete` (persisted in Tauri store) is checked on subsequent launches to skip it.

### Linear step flow

```
Welcome -> Preferences -> Assistants -> Folders -> Scan ->
Projects -> Libraries -> Instruments -> Inference -> Assignments -> Done
```

Each step has a `commitStage(name)` that calls the relevant daemon API and advances to the next step.

### Per-step state

```typescript
class WizardState {
  stage = $state<WizardStage>("welcome");
  roots = $state<ScanRoot[]>([]);
  preferences = $state<Preferences>({});
  assistants = $state<AssistantStatus[]>([]);

  commitStage(stage: WizardStage): Promise<void>;
  reset(): void;
}
```

Key stages and their daemon API calls:

| Stage | API | Purpose |
|-------|-----|---------|
| Preferences | `PUT /api/config` | Language, telemetry, sharing, digest cadence |
| Assistants | `GET /api/assistants/detect`, `POST /api/assistants/configure` | Detect installed ACPs, register MCP |
| Folders | `POST /api/scan/roots`, `DELETE /api/scan/roots` | Add/remove watch roots |
| Scan | `POST /api/scan`, `GET /api/scan/events` (SSE) | Trigger scan, stream progress |
| Projects | `GET /api/projects`, `PUT /api/projects/:id` | Review auto-grouped projects |
| Libraries | `GET /api/libraries` | Auto-detected from manifests |
| Instruments | MCP registry browse | Third-party MCP services |
| Inference | Hardware detect + provider config | Ollama models, cloud API keys |
| Assignments | `PUT /api/gateway/assignments` | Roles: inference, consolidation, embedding, voice, fallback |

### SSE scan events (StateEvent format)

The scan stage opens `GET /api/scan/events` **before** triggering the scan. Two entity types stream on the same SSE connection:

```typescript
// Entity routing
{ action: "add|update|remove|set", entity: "project|activity", data: T }
```

**Project events** -- left panel cards with per-folder progress:

```typescript
interface ScanProject {
  id: string;
  name: string;
  status: "scanning" | "indexing" | "active" | "failed";
  folders: ScanProjectFolder[];
  autoDetected: boolean;
  confidence: "high" | "medium" | "low";
}

interface ScanProjectFolder {
  id: string;
  name: string;
  path: string;
  stack: string[];
  filesTotal: number;
  filesCompleted: number;
  status: "discovered" | "queued" | "indexing" | "indexed" | "failed";
}
```

Folder updates are partial (only changed folders in the array); the client merges by folder `id`.

**Activity events** -- right panel scrolling log (append-only):

```typescript
interface ActivityEvent {
  id: string;
  level: "discover" | "queue" | "process" | "info" | "success" | "error";
  message: string;
  elapsed: number;
  timestamp: number;
}
```

---

## Observatory screens

The observatory is the daily-use surface. All screens follow the same three-layer architecture (see State Management below).

### Component structure

| Screen | Route | What it shows |
|--------|-------|---------------|
| **Dashboard (Today)** | `(app)/` | FTR hero, 14-day sparkline, hero koan, top 3 insights, adopted teachings, recent sessions |
| **Sessions** | `(app)/sessions` | Cross-project session browser, retro cards (going well / not going well / insights), filters by project/language/outcome |
| **Session detail** | `(app)/sessions/[id]` | Event timeline: start, context, edit, test, correction, end. Correction analysis. |
| **Coaching (Learnings)** | `(app)/learnings` | Memories with scope/strength/status, recommendations inbox, pattern registry |
| **Memory detail** | `(app)/learnings/[id]` | Single memory anatomy: what, why, how, where. Evidence trail, good/bad examples. |
| **Metrics** | `(app)/metrics` | FTR trend, turn efficiency, tool preference adherence, phase velocity |
| **Project overview** | `(app)/projects/[id]` | Project header, FTR trend, recommendations with pre-built prompts, repo list |
| **Project graph** | `(app)/projects/[id]/graph` | Code graph with three lenses: complexity (god-nodes), rework (repeat edits), staleness |
| **Project patterns** | `(app)/projects/[id]/patterns` | Followed patterns (rule/suggested/gap), anti-patterns with fix cross-links |
| **Project sessions** | `(app)/projects/[id]/sessions` | Scoped session list |
| **Project settings** | `(app)/projects/[id]/settings` | Links, guidelines, backlog, skills, exclusions, privacy |
| **Libraries** | `(app)/libraries` | Detected (from manifests), imported (internal SDKs), external services (MCP) |
| **Instruments** | `(app)/instruments` | Playground (try tools), Replay (session tool timeline), Insights (usage stats) |

### Sidebar

Collapsible left sidebar (wide with labels, collapsed to icons). Contains:

- Observatory nav (Today, Sessions, Learnings, Libraries, Instruments)
- Active projects (with FTR dots and warning indicators)
- Recent/dormant projects (collapsed)
- Daemon heartbeat status
- Settings link

---

## State management

Svelte 5 runes power all reactive state. Every screen follows the same three-layer split with no exceptions.

### Layers

```
Component (renders)  <--  State (transitions)  <--  API Layer (fetches)
```

**Component** (`Screen.svelte`): Pure render function. Reads from state, renders UI. No fetch calls, no business logic.

**State** (`*-state.svelte.ts`): Reactive store with explicit transition functions. Uses `$state` for mutable data, `$derived` for computed values. No implicit side effects.

```typescript
let components = $state<ComponentStatus[]>([]);
let ready = $derived(components.every(c => isReady(c.state) || isSkipped(c.state)));

export function applyBootstrapResult(result: BootstrapResult) {
  components = result.components;
  hardware = result.hardware;
}
```

**API layer** (`*.ts`): Two patterns:

- **Pattern A: Load** -- one-shot fetch on mount or navigation. Returns typed data.
- **Pattern B: Live** -- SSE stream via `EventManager`. Connects to endpoint, parses events, dispatches to subscribers. Auto-reconnects on error (3s delay).

```typescript
class EventManager<T> {
  constructor(private url: string, private parse: (data: string) => T) {}
  subscribe(handler: (event: T) => void): () => void;
  destroy(): void;
}
```

### Store pattern: API -> store -> UI

1. Page mounts, calls API load function (or opens SSE stream).
2. API returns data, page calls store transition function.
3. Store updates `$state`, `$derived` values recompute.
4. Component re-renders from store reads.

### Rules

1. Components never call fetch/invoke directly.
2. State stores never create EventSource or call fetch.
3. API functions are pure -- return data, no side effects.
4. SSE connections always go through EventManager.
5. State transitions are explicit functions, not implicit effects.

---

## Styling

The design system is called **Rokkit**. It defines a token palette, typography stack, and CSS custom property layer.

### Token palette

| Token | Color space | Usage |
|-------|-------------|-------|
| **Paper** | `oklch(0.97 0.008 85)` | Washi-paper background |
| **Sumi** | Near-black | Foreground text (ink) |
| **Shu** (vermillion) | `oklch(0.58 0.15 35)` | Primary accent, signal |
| **Jade** | `oklch(0.65 0.08 150)` | Positive / calm states |
| **Amber** | Warm yellow | Warning / attention |
| **Matcha** | Greenish | Positive states (secondary) |

### Typography

| Role | Font | Usage |
|------|------|-------|
| Display / numerals | Fraunces | FTR hero numbers, headings, kanji |
| UI text | Inter | Labels, body text, navigation |
| Code | JetBrains Mono | Code blocks, monospace data |

### CSS custom properties

All design tokens are exposed as `-z-*` CSS custom properties (e.g., `-z-paper`, `-z-sumi-1`, `-z-shu`). Components consume tokens through these variables, never raw color values. This enables theming and dark mode through a single property layer swap.

### Dark mode

The token palette inverts through skin definitions. Paper becomes near-black, Sumi becomes light. Accent colors (Shu, Jade, Amber, Matcha) adjust luminance to maintain contrast. The swap happens at the `-z-*` variable level -- no component changes needed.

### Voice

Terse, koan-like. Coaching text uses short imperative phrases: "The AI does not know your auth." "Three corrections. One teacher." Kanji characters mark wizard steps and section headings throughout the UI.

---

## Routing

The app uses SvelteKit route groups to separate the two major phases:

### `(health)/health` -- Bootstrap gate

| Route | Screen |
|-------|--------|
| `/health` | Bootstrap gate checks (lives in the `(health)` group; always reachable, exempt from the setup gate) |

### `(config)/setup/` -- Wizard

| Route | Screen |
|-------|--------|
| `/setup/welcome` | Wizard step 1 |
| `/setup/preferences` | Wizard step 2 |
| `/setup/assistants` | Wizard step 3 |
| `/setup/folders` | Wizard step 4 |
| `/setup/scan` | Wizard step 5 (SSE) |
| `/setup/projects` | Wizard step 6 |
| `/setup/libraries` | Wizard step 7 |
| `/setup/instruments` | Wizard step 8 |
| `/setup/inference` | Wizard step 9 |
| `/setup/assignments` | Wizard step 10 |
| `/setup/done` | Wizard step 11 |

### `(app)/` -- Observatory and project

| Route | Screen |
|-------|--------|
| `/` | Dashboard (Today) |
| `/sessions` | Session browser |
| `/sessions/[id]` | Session detail |
| `/learnings` | Coaching / memories |
| `/learnings/[id]` | Memory detail |
| `/libraries` | Library browser |
| `/instruments` | MCP playground / replay / insights |
| `/projects/[id]` | Project overview |
| `/projects/[id]/graph` | Code graph |
| `/projects/[id]/patterns` | Patterns + anti-patterns |
| `/projects/[id]/sessions` | Project sessions |
| `/projects/[id]/settings` | Project configuration |
| `/settings` | App settings, ACP config, extensions |

Routing gates live in `src/hooks.client.ts` (synchronous, runs before any page mounts):

| Tier | Source | Redirects to |
|------|--------|--------------|
| Upgrade | `localStorage['sensei:app-version']` ≠ running app version | `/upgrade` |
| Health | `sessionStorage['sensei:health']` ≠ `'ready'` | `/health` |
| Setup | `localStorage['sensei:setup-complete']` ≠ `'1'` | `/setup/welcome` |

`HealthState` owns the health-cache key (writes `'ready'` on `apply(ok)`, clears otherwise). `(observatory)/+layout.ts`, `(project)/+layout.ts`, and `(config)/+layout.ts` call `appState.load()` which reconciles the setup-complete localStorage flag from the daemon's `config['setup_complete']` so it cannot drift past a daemon write that did not land.
