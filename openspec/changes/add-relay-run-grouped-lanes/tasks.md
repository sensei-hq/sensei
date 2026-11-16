## 1. Daemon (red first)
- [x] 1.1 A run-tasks read (in Now: `run_tasks`, built in `relay_now`, not the view): priority (item position, Task 0, plan order), claimed_at, completed_at, implementer + reviewer lanes (codename, family, version, model); handler + store pins
- [x] 1.2 Live: a task event (ids only) re-reads Now, which carries `run_tasks` — no new event shape needed
- [x] 1.3 No DDL: the read is a query in `relay_now` (`make test-template` after develop's DDL)

## 2. State and selectors (red first)
- [x] 2.1 Types + `toRunGroups(now)` and `toAgentGroups(now)` in `@sensei/state` (ordering, attribution, "not reported")
- [x] 2.2 The view choice kept per viewer

## 3. UI (red first)
- [x] 3.1 Run group component (header + waiting / in progress / done), agent group component, the switch
- [x] 3.2 Mobile-first layout (phone: stacked rows; `md:` columns)
- [x] 3.3 `/preview` fixtures for each group state; capture at both widths and themes

## 4. Verification
- [x] 4.1 Run journey e2e (#426) pins: a claim moves a task, done order and attribution, By agent switch — written;
  green 2026-11-16 on the installed daemon
- [x] 4.2 `openspec validate add-relay-run-grouped-lanes --strict`
