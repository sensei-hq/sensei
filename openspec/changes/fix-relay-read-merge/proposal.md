# Change: A Now read merges with newer stream rows instead of replacing them (#450)

## Why
A read that was out while a lane registered lands with its older snapshot and replaces the
lanes, so the new member disappears until the follow-up read (added in #426) brings it back —
the crew roster blinks while a crew joins.

## What Changes
- When a read lands, rows the stream delivered after that read started are kept (newer by
  arrival), and rows the read lacks but the stream added since are not dropped; a stream
  `remove` after the read started stays removed.
- The follow-up read stays as a backstop for ids-only events.

## Impact
- Specs: `relay-state`. Code: `packages/state/relay/relay-state.svelte.ts` (+ entity states).
