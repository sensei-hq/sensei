# Checkpoint — 2026-11-20 (relay open items — #389, acceptance r1 on PR #508)

**Slice:** `add-relay-decision-response-form` R1–R4 (#389). Branch `lane/relay-389-r2` (worktree
`lanes/relay-open`), PR #508 to develop.

**Done (red then green):**
- R1: sensei-hq/rokkit#197, shipped in `@rokkit/*` 1.16.0.
- R2: red `827e1140d`, green `e5202825e`. Adds pick-several and long text.
- R3 (review r1, a11y): red `e5d84d39f`, green `72993dfcc`. rokkit#206: nested tag remove, unnamed listbox.
- R4 (acceptance r1): the filled pick-several failed axe contrast in dark mode (2.49:1).
  - red `387fcf6f8`, green `132e457e3`: rokkit writes `data-field-disabled="false"`, and zen-sumi fades any
    such field to 40% (rokkit#207). `DecisionForm` passes `disabled` only when true.
  - red `5e7f0ecd9`, green `1c2c7e0f2`: the open Select placeholder is drawn at 60% in ink-mute, 3.98:1
    (rokkit#208). A Select or pick-several on the card draws no placeholder.
  - `e1244b066`: the e2e closes the list and focuses the notes before running axe. A focused, picked
    option reads 4.4:1 (rokkit#209). That is rokkit's look, left as shipped and annotated on the test.
- Evidence: DecisionRow unit 29/29; `bun run check` 0 errors; hook (make test-fast) green;
  relay-decision-form e2e 7/7, and 21/21 with `--repeat-each=3`. Full `make test-app-e2e` at `e1244b066`:
  146 passed, 22 skipped, 0 failed.

**Remaining:** PR #508 CI green, then merge to develop with "Closes #389". Open upstream issues: rokkit#205,
#206, #207, #208 and #209. Drop each workaround when its fix ships.

**Next command:** `gh pr checks 508`. **Open questions:** none.
**Known broken:** none in this slice. Under load, the setup-wizard and workspace-window e2e specs time out
at times. One full run failed 7 of them; the next run passed them all.
