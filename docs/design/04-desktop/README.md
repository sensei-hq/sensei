# 04 — Desktop

Tauri + SvelteKit desktop application. Reads from daemon HTTP API. Provides visualization, configuration, and guided coaching. Does NOT write workflow state — that's the daemon's job.

**Traces to:** [ideas/10](../../ideas/10-visualization.md), [blueprints/02](../../blueprints/02-system-architecture.md)

| Doc | Description |
|-----|-------------|
| [ux-redesign.md](./ux-redesign.md) | Solution-centric navigation redesign |
| [views.md](./views.md) | Dashboard pages — quality, phase timeline, events, patterns, coaching |
| [queue-worker-sse-pattern.md](./queue-worker-sse-pattern.md) | Design pattern for real-time UI updates via SSE |
