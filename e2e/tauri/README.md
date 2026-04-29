# Tauri E2E Tests

## Current state

**`tauri-driver` does not support macOS 26 (Tahoe)** as of v2.0.5.
Full WebView-level E2E testing is blocked until upstream fixes this.

## What works now

### Rust integration tests (Tier 1)

Tests the actual sidecar logic — real binary detection, port probes, platform provider, event contract shapes — without the Tauri runtime or WebView.

```sh
cd src-tauri && cargo test --test bootstrap_integration
```

18 tests covering:
- `run_bootstrap` returns real component statuses
- Binary detection (`which postgres`, `which ollama`, etc.)
- Port probing
- Database checks
- Platform provider + remedies
- Event payload contract (JSON shapes match frontend expectations)

### Browser E2E with Playwright (Tier 2)

Tests the UI rendering with mock data (no real sidecar).

```sh
bun run test:e2e
```

## When tauri-driver gets macOS 26 support

Re-enable `wdio.conf.ts` and `bootstrap.spec.ts`:

```sh
bun run test:tauri
```

This will launch the compiled app via tauri-driver and drive the WebView with WebdriverIO, exercising real Tauri invoke commands from the UI.

Track: https://github.com/tauri-apps/tauri/issues — search for "tauri-driver macOS"
