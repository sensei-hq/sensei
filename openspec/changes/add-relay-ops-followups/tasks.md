# Tasks: add-relay-ops-followups (#444)

TDD: red test first for each item.

- [x] 1. Sentinel: pure verdict fn + tests; 15 s probe wiring; SSE
      `health_warning`; claim throttle (503) with endpoint tests;
      emergency janitor hook; < 20 GB escalation with tests
      (escalation is a desktop notification + error log, not open_asked:
      the sentinel is crewless and open_asked is crew-scoped)
- [x] 2. Janitor: four stages + three triggers; per-stage reclaim
      reporting; fail-log-and-continue; tests per stage and trigger
      (implemented first: the sentinel calls it; cargo clean -p deferred,
      needs a crate allowlist — incremental is scavenged instead)
- [x] 3. Shared build: lease acquire/release around broker builds;
      `POST /api/relay/resources/execute` with streamed output +
      props recording + tests; shared cache env for worktrees
      (broker builds serialize on a process mutex, not a lease row:
      the lease holder is a lane and the broker is lanless; the
      endpoint answers one bounded JSON record — the child pipes
      stream into the daemon while the build runs — and a queued
      waiter polls behind the release, bounded 5 min; slot frees on
      completion)
- [x] 4. Gates green, commit, push
