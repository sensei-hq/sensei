# Change: Global scan-progress footer (mini / band / full)

## Why

Indexing progress is visible only on the setup scan page. A root added later
from Settings indexes invisibly — the daemon works through tens of thousands
of tasks while every screen reads as idle. Users conclude the indexer never
started.

## What Changes

- Scan state moves into `@sensei/state` (closes #496): `ScanState` and its
  source migrate per the package checklist, the stream becomes a
  `StateStream<E>`, and the app holds one module-level instance every
  component shares.
- `ScanState` gains a read-only `attach()` (observe-only: SSE + hydrate +
  queue poller, never triggers scans). `start()` keeps its setup behaviour
  (attach, then trigger unscanned roots).
- Passive observation oscillates `idle ↔ scanning` and never writes `done`,
  so the footer cannot trip the wizard's Continue gate.
- One shared `ScanProgress.svelte` with a static `variant` prop
  (`mini | band | full`), all variants reading the same state:
  - `mini` in each section sidebar's `Rail` footer: compact
    completed/queued counts + expand button. Hidden while idle.
  - `band` in the root layout: full-width, small-height bar with stats +
    latest activity; visible when expanded while active.
  - `full`: the existing setup scan view, extracted verbatim; the setup
    route renders it with no visual change.
- Mini ↔ band coordination is shared UI state (`expanded`) in the scan
  state module, not a wrapper prop.

## Impact

- Affected specs: `scan-progress` (new capability).
- Affected code: `packages/state/src/scan/` (new, from
  `app/src/lib/scan-state.svelte.ts`), `app/src/lib/scan.ts` (the shared
  instance), `app/src/lib/components/indexer/` (new: `ScanProgress.svelte` +
  per-variant internals), `app/src/routes/+layout.svelte` (band mount),
  section sidebars (mini mounts),
  `app/src/routes/(config)/setup/scan/+page.svelte` (thins to Full).
- No daemon, API, or DDL changes. No new dependencies.
