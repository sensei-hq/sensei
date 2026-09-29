# Checkpoint

**Slice:** the call-graph screen — daemon contract first, then the UI.

## Done

- **rokkit 1.4 → 1.6 across the app** (2174b5e0) + `@rokkit/graph`. Its deps are
  EXACT pins, so the stack moves together. 1,700 tests, build green.
- **`brew install` was broken both ways** (88797b94, tap b6423fa). README named
  the formula and cask tokens transposed; the cask token resolved a dead
  `Casks/sensei.rb` left by the `senseihq` rename and 404'd. Both verified
  against the live tap after the fix. Gate: `scripts/check-brew-tokens.py`.
- **Callers AND callees now carry the placement verdict** — `resolved_via` /
  `unresolved_reason` off `sensei.call_graph`, plus `edge_kind`. `resolved: bool`
  could not tell `declared_here` from `in_the_prelude`. New `Placement` enum;
  `callee_row` is now the ONE builder for both callee paths, which fixes a
  chased row carrying no `resolved` key at all.
- **rokkit #160–#164 filed**, each with a probe that reproduces it.
- **Two type errors from the 1.6 upgrade fixed**: `Toggle` takes `label`, not
  `aria-label`, and has no rest-spread — two Projects toggles had silently lost
  their accessible name. `make app-check` existed in no gate; now in `make test`
  and in CI.

## Next command

    cargo test -p senseid        # confirm the full suite before committing

## Open

- **The screen itself is not built.** Three views, three readiness levels:
  - *Neighbourhood* — data is live NOW (this slice). Buildable next.
  - *World* — needs a new aggregating endpoint: `/api/graph/{repo}/tree` returns
    every node unaggregated and carries no counts. Plus rokkit #161/#163/#164.
  - *Schema (ER)* — no data source at all; the mockup says so itself. Blocked on
    `docs/spec/indexer/20-schema-entities.md`.
- **#131 / #132 mockup-drift audit not yet revisited** (asked for alongside the
  screen). The 1.6 upgrade changes its baseline.
- **#191** — dojo and website still on rokkit `^1.2.0`; dojo also kavach 1.1.3.
- **#187** — `app/e2e/**` still outside every static gate.
