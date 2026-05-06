# sensei app

Desktop observatory for sensei. Built with Tauri + SvelteKit + Svelte 5.

## Dev

```bash
bun install
bun run dev          # SvelteKit dev server
bun run tauri:dev    # Tauri + SvelteKit
```

## Build

```bash
bun run tauri:build
```

## Pages

| Route | Purpose |
|-------|---------|
| `/observatory` | Daily landing — FTR, hero koan, insights |
| `/sessions` | Cross-project session browser with retro |
| `/learnings` | Memories, patterns, corrections, lifecycle |
| `/libraries` | Detected + imported + service libraries |
| `/instruments` | MCP playground, replay, insights |
| `/settings` | General, assistants, inference, extensions |
| `/projects/[id]` | Project overview, graph, patterns, sessions, settings |
