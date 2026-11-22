## Context

`ScanState` (`app/src/lib/scan-state.svelte.ts`) is already a
navigation-surviving singleton: SSE subscription, workspace/activity stores,
and a queue-idle poller. Only the setup scan page drives it, and only via
`start()`, which *triggers* scans. Outside setup the state sits `idle`
forever, no matter how busy the daemon is.

`/api/index/status` already serves queue counts
(`pending/blocked/running/completed`); the done poller reads it every 1.5s
but keeps only the idle verdict.

Sidebars are per-section (`WorkspaceSidebar`, `ObservatorySidebar`, …),
each passing a `footer()` snippet to `Rail` — there is no single global
sidebar, so Mini mounts in each section footer.

## Goals / Non-Goals

- Goals: live indexing progress on every screen; one component, one state,
  three variants; zero behaviour change to the setup flow; close #496 as the
  state foundation.
- Non-Goals: promoting the components to `@sensei/ui` (follow-up);
  daemon/API changes; a new full-page route (the setup scan page IS the full
  variant); Tauri e2e (follow-up, per #496 checklist).

## Decisions

### Component shape

```
app/src/lib/components/indexer/
├── ScanProgress.svelte   # public: `variant: 'mini'|'band'|'full'`, thin switch
├── ScanMini.svelte       # internal: compact counts + expand button
├── ScanBand.svelte       # internal: full-width stats + activity + collapse
└── ScanFull.svelte       # internal: setup scan view, extracted verbatim
```

- One public name, split internals: the variant bulk lives in per-variant
  files, so `ScanProgress.svelte` stays a ~10-line switch and no file grows
  a second concern.
- The variant prop is safe because it is static per mount point — no mount
  ever swaps variants at runtime, so there is no variant-transition state to
  go wrong. The only runtime switch is mini↔band visibility, coordinated by
  shared `expanded` UI state, not by the prop.
- No prop drilling: every variant imports the `scanState` singleton and
  reads reactively. The component connects to state by import, exactly as
  the setup page does today.

### Mounts

| Mount | Markup | Visibility |
|---|---|---|
| Section sidebar `footer()` snippets | `<ScanProgress variant="mini" />` | Renders nothing while idle |
| Root `+layout.svelte`, after `children` | `<ScanProgress variant="band" />` | `expanded && active` |
| Setup scan route | `<ScanProgress variant="full" />` | Always (owns `start()` on mount) |

- `expanded` (default false) lives in the scan state module beside the run
  state — the composite owns its UI state, per the #496 checklist. It
  survives navigation and remounts; session-scoped, not persisted.
- `active` = `status === 'scanning'`, plus unreachable/error mid-run.
- Wide-to-full navigation: the band links to the setup scan route. If the
  setup layout guard refuses post-setup entry, the link hides outside setup
  (decided at implementation; spec covers both via the guard clause).

### State (#496)

- Move `ScanWorkspaceState`, `ScanActivityState`, `ScanState`, `pollStep`,
  `triggerScans` to `packages/state/src/scan/`, following the `relay/`
  exemplar: source injected at construction, stream as `StateStream<E>`,
  re-hydrate on every open (broadcast has no replay).
- The app holds one module-level instance built over a `scanSseSource`
  (SSE + the reads the state needs: workspaces, roots, index status, scan
  trigger). Footer, sidebars, and setup import the same instance.
- `attach()` over a second state object: the footer observes through the
  same singleton the setup page owns. A separate footer poller would
  duplicate the SSE stream and could disagree with the scan page.
- Passive mode never writes `done`: `done` gates the wizard Continue
  button (`wizardState.canAdvance('scan')`). Passive oscillates
  `idle ↔ scanning` on queue busyness; only owning `start()` (setup) can
  reach `done`/`error`.
- Queue snapshot in state: the poller tick stores the latest
  `{pending, running, completed}` alongside the idle verdict. The mini
  counts are a pure selector over snapshot + workspace file totals.

## Risks / Trade-offs

- `reset()` on wizard re-entry clears the singleton mid-run; the footer's
  mount effect re-attaches idempotently, so the band flickers rather than
  dies. Accepted: reset semantics belong to the wizard.
- One SSE stream shared by footer + scan page: `EventManager` broadcast
  already fans out to subscribers; attach-while-attached is a no-op.
- Poll cadence stays 1.5s (existing); the mini counts update at most that
  often. Fine for counts.

## Migration Plan

No migration. Additive UI change; rollback is reverting the mounts. The
#496 move deletes `app/src/lib/scan-state.svelte.ts` in favour of the
package + a thin app instance module — one commit, no shims left behind.

## Open Questions

- None. Band-to-full link visibility is decided by the setup guard at
  implementation time.
