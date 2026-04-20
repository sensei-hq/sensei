---
name: State class first, test it, then wire UI
description: For any page with server-side data that changes, build a reactive state class first, integration test it, then wire the UI to just render
type: feedback
---

Pattern for every page with dynamic data:
1. Create a reactive state class (*.svelte.ts) that owns data + SSE/polling
2. Write integration tests against the real daemon
3. Build shared components if needed
4. Wire the UI page to just read from the state class and render

**Why:** Testing the state class in isolation catches data flow bugs before they become UI bugs. The UI becomes trivial — just rendering what the class exposes. If the state class tests pass, the page works.

**How to apply:** Never put fetch/SSE/polling logic in a page component. Always in a state class. Test the class first. Page imports the class and reads derived values.
