# sensei app

Desktop observatory for Sensei. Built with Tauri + SvelteKit + Svelte 5.

## Dev

```bash
# From monorepo root (recommended)
make install-dev      # build daemon + install to ~/.local/bin
make app-dev          # Tauri dev with Vite HMR (pre-compiles Rust backend)
make app-dev-bundle   # build debug .app bundle and launch it
```

## Build

```bash
# From monorepo root
make app-release      # production Tauri bundle
```

## Type-check

```bash
# From monorepo root
make app-check

# Or from this directory
bun run check         # svelte-check + tsc
bun run check:watch   # watch mode
```

## Tests

```bash
# From monorepo root
make test-app         # unit
make test-app-unit    # Vitest unit tests only
make test-app-e2e     # Playwright e2e (requires Tauri build)

# Or from this directory
bun run test:unit
bun run test:e2e
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
