# Tauri E2E Tests

Tests the compiled Sensei desktop app with real sidecar commands.

## Prerequisites

```sh
cargo install tauri-driver
safaridriver --enable  # macOS: may need sudo, run once
```

## How it works

1. `bun run build` — builds the SvelteKit frontend to `build/`
2. `cargo build` — builds the Tauri app in debug mode
3. `tauri-driver` — starts a WebDriver server that proxies to the Tauri WebView
4. WebdriverIO connects to the WebDriver server and drives the app

## Running

```sh
bun run test:tauri
```

## Architecture

```
WebdriverIO test  →  tauri-driver (:4444)  →  Tauri app (WebView)
                                                   ↓
                                              sidecar commands
                                              (real brew, psql, etc.)
```

The tests exercise the actual sidecar commands — binary detection, port probing,
service management — not browser-mode mocks.
