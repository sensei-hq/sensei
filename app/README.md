# sensei app

Desktop observatory for Sensei. Built with Tauri + SvelteKit + Svelte 5.

## Dev

```bash
# From monorepo root (builds daemon first)
make install-dev
make app-dev

# Or directly from this directory
bun install
bun run tauri:vite-dev   # Tauri + Vite HMR
```

## Build

```bash
# From monorepo root
make app-release

# Or directly
bun run tauri:build
```

## Daemon build scripts

```bash
bun run daemon:dev        # build all daemon binaries (debug)
bun run daemon:release    # build all daemon binaries (release)
bun run daemon:dev:daemon # build senseid only
bun run daemon:dev:cli    # build sensei-cli only
bun run daemon:dev:mcp    # build sensei-mcp only
```

## Routes

| Route | Purpose |
|-------|---------|
| `/` | Bootstrap gate — waits for daemon health |
| `/setup/*` | Setup wizard — 11 stages from welcome through assignments |
| `/observatory` | Daily landing — insights, hero koan |
| `/sessions` | Cross-project session browser |
| `/learnings` | Memories, patterns, corrections |
| `/libraries` | Detected and imported libraries |
| `/instruments` | MCP playground and insights |
| `/settings` | General, assistants, inference, extensions |
| `/projects/[id]` | Per-project overview, graph, patterns, sessions |

## Tests

```bash
bun run test:unit    # Vitest unit tests
bun run test:e2e     # Playwright e2e (requires Tauri build)
```
