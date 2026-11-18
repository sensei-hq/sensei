# Checkpoint — 2026-11-18 (relay open items — #420 verified, #299 next)

**Slice:** one OpenSpec change per open relay issue, built one by one.
Current: `verify-relay-d87-shell` (#420), branch `lane/relay-420-s1` (worktree `lanes/relay-open`).

**Done (#420):**
- Re-proved on develop `f880251b8`, no code change needed:
  - unit: relay-shell, relay-shell-input, ShellScreen specs — 65/65 green.
  - Tauri e2e: relay-conformance 3/3, relay 3/3, relay-run, relay-timeline, zz-a11y — 9/9 green.
  - axe: zero violations (any impact) on `/workspace/<id>/relay`, light and dark.
- Real brew daemon restored on :7744 (`dbName: sensei`).
- Evidence commented on #420; #420 closed; phone findings pointed at `update-phone-layout`.
- `openspec validate verify-relay-d87-shell --strict` clean.

**Remaining build order:** #299, #398, #312, #414, #389, #455, #393, #444, #427, #314.

**Next command:** `openspec show verify-dojo-sync-files-workspaces`, then run its tasks.

**Open questions:** none blocking. #314, #312 and #299 need production evidence to close (user's step).

**Known broken:** none from this branch. The worktree needs `bun install` in `app/` and `dojo/` for the hook.
