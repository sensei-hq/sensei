## Decisions (user, 2026-11-16)
- Cost: the classifier runs **daily** over the day's candidate turns (as process quality does).
- Backfill: **yes** — relabel all history; phrase labels are replaced by intent labels with source `intent`.

## Open Questions
- Q2: Floors — the precision/recall the labelled set must hold (more information requested; see the
  measurement step: build the labelled set and record the phrase list's baseline first).
