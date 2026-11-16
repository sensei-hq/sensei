## 1. Daemon (red first)
- [ ] 1.1 A run-tasks read (in Now, or `GET /relay/runs/{id}/tasks`): priority (item position, Task 0, plan order), claimed_at, completed_at, implementer + reviewer lanes (codename, family, version, model); handler + store pins
- [ ] 1.2 Task events carry the fields so the screen updates without a re-read
- [ ] 1.3 DDL via dbd full files; `dbd diff` clean; `make test-template`

## 2. State and selectors (red first)
- [ ] 2.1 Types + `toRunGroups(now)` and `toAgentGroups(now)` in `@sensei/state` (ordering, attribution, "not reported")
- [ ] 2.2 The view choice kept per viewer

## 3. UI (red first)
- [ ] 3.1 Run group component (header + waiting / in progress / done), agent group component, the switch
- [ ] 3.2 Mobile-first layout (phone: stacked rows; `md:` columns)
- [ ] 3.3 `/preview` fixtures for each group state; capture at both widths and themes

## 4. Verification
- [ ] 4.1 Run journey e2e (#426) pins: a claim moves a task, done order and attribution, By agent switch
- [ ] 4.2 `openspec validate add-relay-run-grouped-lanes --strict`
