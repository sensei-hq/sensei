# Checkpoint — 2026-11-24 (pause; #531 #532 #430 done)

**Slice:** 4–5h prioritized batch.
**Branch:** `develop` (in sync, pushed).

## Done
- #531: repo lists sort by activity (new
  `repository_activity` fn + gated tests); recency
  shown from the same key; dojo 2533, DB 84 green.
- #532: per-repo screen (header, metric cards,
  diagrams link, branch preserved); shared loader
  context + `RepoMetricCards`; dojo 2549 green.
- #430: churn excludes the creation Write; new
  files never flag as rework. Suites green.
- #246/#312/#433/#299 closed (confirmed).

## Remaining (post-travel)
- #513/#512/#444/#393/#268/#531/#532/#430 close
  on the next main merge (release cut).
- #292 → verify stale vs #536, link or close.
- Next candidates: #522 (relay-dōjō chain start),
  #533 (workspace screen), #301/#283 (sync).

## Next command
None — paused for travel. Resume: pick from Remaining.

## Open questions
- 6 process.rs fmt spots: local toolchains flag what CI stable
  accepts. Left untouched.

## Known-broken
- None.
