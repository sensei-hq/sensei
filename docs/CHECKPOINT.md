# Checkpoint — 2026-11-20 (rokkit 1.16.1 follow-up — #389 R3/R4 workarounds dropped)

**Slice:** rokkit 1.16.1 follow-up (`add-relay-decision-response-form` R3/R4 done-notes).
**Branch:** `lane/relay-rokkit-1.16.1` (worktree `lanes/relay-rokkit`), from origin/develop @ c4162893d (#510); #514 merged in.

## Done
- Bump: `@rokkit/*` ^1.16.0 → ^1.16.1 in `app/` + `dojo/` (+ lockfiles); 1.16.1 verified on npm.
- Deleted `DecisionPickSeveral.svelte`: the card draws rokkit's MultiSelect directly (#206).
- Removed the listbox-naming attachment (#206), `disabled`-only-when-true (#207),
  chooser placeholder blanking (#208). #205 still OPEN in rokkit: `withArrayLayout` stays.
- Specs (red→green): tags in trigger, placeholders shown, axe on every state. e2e covers
  the open pick-several with members picked (#209); `#209` annotation removed.
- Gates: app check 0 errors, app unit 3912 pass, dojo test 2411 pass + check clean,
  e2e `relay-decision-form.spec.ts` 7/7. Prod daemon restored after the run.

## Remaining
- Push lane, PR to develop, CI green, merge. No issue to close (follow-up of closed #389).

## Next command
`git push -u origin lane/relay-rokkit-1.16.1` (from `lanes/relay-rokkit`), then `gh pr create`.

## Open questions
- None.

## Known-broken
- None.
