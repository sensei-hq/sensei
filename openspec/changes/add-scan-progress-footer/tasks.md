## 1. State into @sensei/state — closes #496 (test-first)

- [x] 1.1 Move `ScanWorkspaceState`, `ScanActivityState`, selectors into `packages/state/src/scan/`
- [x] 1.2 Move `ScanState` with source injected at construction; stream as `StateStream<E>`
- [x] 1.3 Re-hydrate on every stream open (broadcast has no replay)
- [x] 1.4 State specs drive `apply` with canned events, no DOM
- [x] 1.5 App holds one module-level instance over the daemon SSE source; delete the old module

## 2. Passive observation (test-first)

- [x] 2.1 `ScanState.attach()`: SSE + hydrate + poller, no scan triggers; idempotent
- [x] 2.2 Passive mode oscillates `idle ↔ scanning` on queue busyness; never writes `done`
- [x] 2.3 Poller tick stores the queue snapshot (`pending`, `running`, `completed`)
- [x] 2.4 Pure `miniSummary` selector over snapshot + workspace file totals, with unit specs
- [x] 2.5 `start()` keeps owning semantics (attach, then trigger unscanned roots)

## 3. Shared component

- [x] 3.1 `ScanProgress.svelte` with static `variant: 'mini' | 'band' | 'full'`, thin switch
- [x] 3.2 `ScanMini.svelte`: compact counts + progress + expand button; nothing while idle
- [x] 3.3 `ScanBand.svelte`: full-width stats + latest activity + collapse + full-view link
- [x] 3.4 `ScanFull.svelte`: setup scan markup extracted verbatim (same testids, same actions)
- [x] 3.5 Setup scan route thins to the full variant + owns `start()` on mount
- [x] 3.6 Shared `expanded` UI state in the scan state module (default false, session-scoped)

## 4. Global mounts

- [x] 4.1 Section sidebar `footer()` snippets mount `<ScanProgress variant="mini" />`
- [x] 4.2 Root `+layout.svelte` mounts the band after `children`
- [x] 4.3 Footer attaches on mount (idempotent) and re-attaches after wizard `reset()`
- [x] 4.4 Visibility derived from state: mini while `scanning`; band when `expanded && active`
- [x] 4.5 Band-to-full link honours the setup layout guard

## 5. Verification

- [x] 5.1 Package + app unit suites green (state, selectors, attach, transitions)
- [x] 5.2 `svelte-check` clean
- [x] 5.3 Setup scan flow visually unchanged (Begin → progress → summary → re-scan)
