## 1. Implementation (red first)
- [ ] 1.1 Survey the daemon's error body shapes (axum handlers) and list them
- [ ] 1.2 Shared `refusalReason(body, statusText)` in `@sensei/state`; the relay client and `api.ts` use it; specs for both shapes
## 2. Verification
- [ ] 2.1 `openspec validate fix-app-api-error-reason --strict`
