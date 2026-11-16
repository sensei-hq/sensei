# Change: Name lanes by codename on the task view, and say who handed a task off (#425, #395)

## Why
Lanes have codenames (minted at register since #419) and the lane rows show them, but the task
view still names a lane by its code: the facts line reads "Diagrams · L1" and history entries
say "L1". The mockup names a lane by codename everywhere ("Diagrams · L1 · cobalt-lyra"). The
Handed off card says when and what was handed off, but not by whom or to whom.

## What Changes
- The task view's facts line names the holding lane as "objective · code · codename"; a lane
  with no codename reads its code alone (never a placeholder).
- History entries (by-lines, review verdicts, messages, nudges) name their lane by codename,
  with the code as the hover title.
- The Handed off card names who handed the task off and, when it has one, the lane that took it
  over ("Handed off by cobalt-lyra · 10:42 → coral-spica").
- Daemon: the task view's lanes carry `suggested_name`, and the handoff carries its from-lane
  (and the to-lane when re-claimed); if not already served.

## Impact
- Specs: `relay-task-view`.
- Code: `crates/senseid` task view read (handoff from/to), `packages/state/relay/task-selectors.ts`
  (`toTaskView`, history by-lines), `app/src/lib/relay` (TaskHeader, HistoryEntry, HandoffCard),
  fixtures and `/preview`.
- Closes #425 and the remaining half of #395.
