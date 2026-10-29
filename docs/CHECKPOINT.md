# Checkpoint — 2026-10-29 (Diagrams slices 1–4 built; awaiting the user's review)

**Slice:** the Diagrams rework (mockup: sidebar subsection, per-diagram routes, levels,
inspector), Nav first; then #347 attribution, #348 Muse, #349 Freebuff.

**Done (develop, pushed, installed service + app):**
- c9adada48 / cb8087e86 — automated prompts are transcript turns with origin; your turns only
  in counts and first-try; transcript landing re-queues analysis. Live: 812 automated turns.
- e6da28452 / 7b9a872c2 — slice 1: sidebar Diagrams subsection (8 entries), `diagrams/[lens]`
  over one shared CodeMapState (kept by workspace id value); retired pages removed.
- 8d08aa9fa — daemon: L0 blocks carry files per role + package; L0 carries referenced libraries;
  L2 carries the file's role.
- d10652550 / 504aea6a5 — slices 2–4: header (shows/use/read, remembered), 300px inspector at
  every level, one-click open, hover lights bands, search, band filters, colour by direction,
  Fit/100%, Show at L0, legend, complexity tiers, Database · ER reach, lists under 640px.
- 67c625d8b — Tokens chart threw on a range with no tokens (axisTicks); stale e2e chip.
- e2e: full suite 129/130 on a fresh DB, the one failure fixed and rerun green; live inspect
  codemap (filter = 108 of 213 bands, derived independently), schema reach, screen shell green.

**Waiting on the user:** review of the diagrams; decision on Group by Communities (module →
community rule; recommendation: dominant community, small ones folded).

**Next:** user feedback on diagrams → fixes; then #347.

**Notes:** run e2e via `make test-app-e2e` or `make reset-e2e-db` first — a stale `sensei_e2e`
stamped "deployed for this version" never gets new columns. rokkit d9cbf302 (dark edge 200)
still unreleased (user). Open: title-bar scribble (screenshot), #346, #282 part 3.

**Known broken:** Keychain test on rebuilt binaries (#282).
