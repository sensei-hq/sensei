## 1. Implementation (red first)
- [x] 1.1 Survey the daemon's error body shapes (axum handlers) and list them — `{error: <code>, message: <reason>}`:
  the relay routes (`api/handlers/relay.rs`); `{error: <code or reason>}`: ~96 other sites. The app's `tryGet`/
  `tryPost`/`tryPut`/`tryDelete`/`tryDeleteJson` read no body at all (status text only); `tryJson` read `error`.
- [x] 1.2 Shared `failureFrom(res)` (was: `refusalReason(body, statusText)`) in `@sensei/state`; the relay client and `api.ts` use it; specs for both shapes
## 2. Verification
- [x] 2.1 `openspec validate fix-app-api-error-reason --strict`
