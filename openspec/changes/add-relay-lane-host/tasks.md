## 1. Lane host (red first)
- [ ] 1.1 Register `host` → lane column → now/crew lane shapes; handler + store pins
- [ ] 1.2 Selector: `editor: editorTag(host, model)`; member card shows it; bench fixture already exists
- [ ] 1.3 `/sensei:register` and the guidance tell an editor-hosted session to pass `host`
## 2. Classification (design first; see design.md)
- [ ] 2.1 Tool catalogue `kind: assistant | editor`; session `host`; attribution to the assistant
- [ ] 2.2 AddMember picker: Assistants and Editors seen on this machine
## 3. Verification
- [ ] 3.1 Relay e2e: a lane registered with a host shows "in <editor>"; `openspec validate add-relay-lane-host --strict`
