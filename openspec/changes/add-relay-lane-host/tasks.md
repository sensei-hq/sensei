## 1. Lane host (red first)
- [ ] 1.1 Register `host` → lane column (dbd full file) → now/crew lane shapes; handler + store pins
- [ ] 1.2 Selector: `editor: editorTag(host, model)`; member card shows it (bench fixture exists)
- [ ] 1.3 `/sensei:register` and the guidance tell an editor-hosted session to pass `host`
## 2. Verification
- [ ] 2.1 Relay e2e: a lane registered with a host shows "in <editor>"; `openspec validate add-relay-lane-host --strict`
