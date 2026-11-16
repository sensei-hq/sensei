## 1. Implementation (red first)
- [x] 1.1 relay-state spec: add-during-read keeps the row at every moment; remove-during-read stays removed (queue items too)
- [x] 1.2 Track per-row stream changes since each read started; merge on landing
## 2. Verification
- [x] 2.1 Run journey e2e: once the roster shows three it holds (10 checks, 200 ms apart) — green 2026-11-16
- [x] 2.2 `openspec validate fix-relay-read-merge --strict`
