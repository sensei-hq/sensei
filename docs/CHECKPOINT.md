# Checkpoint — 2026-11-20 (relay open items — #389 slice R2, pick-several and long text)

**Slice:** `add-relay-decision-response-form` R1 + R2 (#389). Branch `lane/relay-389-r2` (worktree
`lanes/relay-open`), from develop after PR #501 (slices B + C1) merged.

**Done (red then green):**
- R1: sensei-hq/rokkit#197, shipped in `@rokkit/*` 1.16.0 (already `^1.16.0` in app/ and dojo/).
- Red `827e1140d`: daemon subset + values tests (`decision_schema.rs`), form mapping
  (`decision-form.spec.ts`), MultiSelect + textarea on the card (`DecisionRow.spec.svelte.ts`), live e2e
  "a pick-several and long-text decision is answered live".
- Green `e5202825e` (parked with --no-verify, then verified in full; the hook's checks run clean on it):
  - daemon: `type: "array"` with `items: {type?: "string", enum}` and optional `uniqueItems: true`;
    `minItems`/`maxItems` refused (rokkit honours none). Answer: an array of unique members; a required
    one needs at least one. `x-multiline: true` on a string without an enum, cap 4000 characters.
  - form: `textCap` (600 / 4000, in characters, as the daemon counts), empty pick-several is empty,
    `withArrayLayout` folds an array element's layout keys into the schema (rokkit drops them, rokkit#205).
  - MCP tool text, D1/D3/D9, the relay-decisions spec delta, lanes.md; R1/R2 ticked.
- Evidence: senseid relay 464 (threads=1), mcp 109, clippy -D warnings, fmt, app unit 3906, app + dojo
  check 0 errors, make test-fast, openspec strict valid; e2e setup-wizard + relay-run +
  relay-decision-form + zz-a11y 21/21 (relay-decision-form 7/7).

**Remaining:** PR to develop with "Closes #389"; CI green. rokkit#205 open (drop `withArrayLayout` when it ships).

**Next command:** `gh pr checks <pr>`. **Open questions:** none. **Known broken:** none.
