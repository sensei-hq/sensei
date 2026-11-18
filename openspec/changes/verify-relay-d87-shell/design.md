# Design: verify-relay-d87-shell

## Current state (verified on `lane/relay-open-items`, which contains `231cbc660`)
| #420 item | Where it lives now |
|---|---|
| Height owned by ShellScreen | `packages/ui/ShellScreen.svelte:48` (`h-full min-h-0`); sidebar focusable `:53` |
| Pinned bands harvested | `packages/ui/ShellScreen.svelte:34,67-69,79`; Relay pins info when empty at `app/src/lib/relay/relay-shell.ts:335` |
| Shell as data | `app/src/lib/relay/relay-shell.ts` (`toRelayShell`), drawn by `app/src/lib/relay/RelayShell.svelte:40` |
| Both windows draw through RelayShell | `app/src/lib/relay/RelayScreen.svelte:196` (used by `(observatory)/relay` and `(workspace)/workspace/[id]/relay`). The task page (`RelayTaskScreen.svelte:42`) and run page (`RelayRunScreen.svelte:29`) draw `ShellScreen` directly, not `RelayShell` |
| "not reported", never 0 | `app/src/lib/relay/relay-shell.ts:199` (info band Today), `:213` (Waiting "oldest not reported"); footer `:251-253,260,262` |
| Schema filters (#384) | `app/src/lib/relay/relay-shell.ts:281-292` (`filterSchema` → `FilterSchema`); `app/package.json:33` `@rokkit/forms` |
| Loading state | unit-pinned: `app/src/lib/relay/relay-shell.spec.ts:106,206-208` |
| Error and loaded in both windows; no crew in the workspace window | `app/e2e/tests/relay-conformance.spec.ts` (3 tests) |

## Decisions
- Loading stays pinned by unit specs, not e2e. The harness cannot hold a read open; the
  conformance spec header says so and the issue's last comment accepted 3 of 3.
- No crew is proven in the workspace window only. The main window lists every crew in the e2e DB,
  so it cannot be held empty; the no-run shell itself is window-independent and unit-pinned
  (`relay-shell.spec.ts` "with no run").
- The phone findings are not reopened here. `update-phone-layout` already owns them, so specifying
  them twice would split one fix across two changes.

## Decisions defaulted (user may override)
- The two phone findings ("left for the user's call") are resolved by `update-phone-layout`'s
  mobile-first ScreenShell/StatCard (info band collapsed on phone, stats 2-up, labels wrap), not by
  a Relay-specific override. #420 closes with a pointer to that change.
- `lane/relay-screen-conformance` retirement stays the user's call; this change does not delete
  branches.
