## 1. Lane host (red first)
- [x] 1.1 Register `host` → lane column (dbd full file) → now/crew lane shapes; handler + store pins
- [x] 1.2 Selector: `editor: editorTag(host, model)`; member card shows it (bench fixture exists)
- [x] 1.3 `/sensei:register` and the guidance tell an editor-hosted session to pass `host`
## 2. Verification
- [ ] 2.1 Relay e2e: a lane registered with a host shows "in <editor>" — written in the run journey (coral-e2e in
  Cursor); runs after the batched daemon install. `openspec validate add-relay-lane-host --strict` ✓
