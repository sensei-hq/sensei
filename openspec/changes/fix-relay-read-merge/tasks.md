## 1. Implementation (red first)
- [ ] 1.1 relay-state spec: add-during-read keeps the row at every moment; remove-during-read stays removed (queue items too)
- [ ] 1.2 Track per-row stream changes since each read started; merge on landing
## 2. Verification
- [ ] 2.1 Run journey e2e: the roster check needs no settle-poll (three cards from the first)
- [ ] 2.2 `openspec validate fix-relay-read-merge --strict`
