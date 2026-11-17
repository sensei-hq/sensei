## 1. Daemon (red first)
- [x] 1.1 Task view: lanes carry `suggested_name`; the handoff carries from-lane and to-lane (when re-claimed) — pins in the handler tests
## 2. State (red first)
- [x] 2.1 `toTaskView` facts with codename; history by-lines by codename (code as title); handoff "by → to"
## 3. UI (red first)
- [x] 3.1 TaskHeader facts, HistoryEntry by-line, HandoffCard; `/preview` fixtures (with and without codename, handoff taken / not)
## 4. Verification
- [x] 4.1 Run journey e2e: the task view names the lane by codename — written in `relay-timeline.spec.ts`; green 2026-11-16
- [ ] 4.2 `openspec validate update-relay-task-view-attribution --strict`; close #425 and #395 on merge
