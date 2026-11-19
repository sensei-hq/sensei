# Checkpoint — 2026-11-19 (relay open items — #389 slice R2, review r1 on PR #508)

**Slice:** `add-relay-decision-response-form` R1 + R2 + R3 (#389). Branch `lane/relay-389-r2` (worktree
`lanes/relay-open`), PR #508 to develop.

**Done (red then green):**
- R1: sensei-hq/rokkit#197, shipped in `@rokkit/*` 1.16.0.
- R2: red `827e1140d`, green `e5202825e` — pick-several (array of unique enum members) and long text
  (`x-multiline`, cap 4000) in the daemon subset, the form and the card.
- R3 (review r1, a11y): red `e5d84d39f`, green after it.
  - rokkit 1.16 MultiSelect nests a focusable `role="button"` tag remove in its trigger (axe
    `nested-interactive`); an open Select/MultiSelect listbox has no name (`aria-input-field-name`).
    Raised as sensei-hq/rokkit#206.
  - `DecisionPickSeveral.svelte`: the MultiSelect via the forms `renderers` registry, `maxDisplay={0}`,
    picked members shown beside the trigger (`data-decision-picked`). `DecisionForm` names an open listbox
    after its field label (an attachment).
  - Unit + e2e run axe on every card state (empty, messages, Select open, filled pick-several).
- Evidence: app unit 3909, `bun run check` 0 errors, make test-fast (hook), openspec strict valid;
  e2e relay-decision-form 7/7 + relay-run + zz-a11y (9/9). Red e2e showed the nested-interactive on
  the filled pick-several.

**Remaining:** PR #508 CI green, merge to develop with "Closes #389". rokkit#205 and #206 open: drop
`withArrayLayout`, `DecisionPickSeveral` and the listbox naming when they ship.

**Next command:** `gh pr checks 508`. **Open questions:** none. **Known broken:** none.
