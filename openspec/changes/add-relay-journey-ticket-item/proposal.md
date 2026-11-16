# Change: The run journey adds an item from a seeded ticket (#449)

## Why
The design's journey (§3 step 1) adds one backlog item from a tracker ticket; the e2e adds only
manual items, so the ticketed path (`POST /relay/backlog` with `ticket_provider` + `ticket_key`,
the triage row's ticket pill, the move to the backlog) is not pinned end to end.

## What Changes
- The e2e harness seeds a synced tracker and one ticket in `sensei_e2e` (via the daemon's own
  sync path or a fixture import — no hand-written rows the app could not produce).
- Journey stage 1 adds that ticket to the backlog by key and picks it with the manual items.

## Impact
- Specs: `relay-e2e`. Code: `app/e2e` (seed + journey), possibly a test-only tracker fixture.
