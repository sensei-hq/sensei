# Checkpoint

**State 2026-10-13.** Slice **#236 step 4e — the daemon acts as the person,
signing in is joining** (`docs/history/plan/2026-10-12-dojo-device-plane-as-person.md`):
DONE on `develop` (`9cae35fd`), ships as **0.13.0** with its dōjō side.

| increment | status |
|---|---|
| v0.12.0 released + merged (#257), dōjō live at `5ecceca` | DONE |
| v0.12.1 hotfix: install schema converges with reconcile (enum values on upgrade), dbd 0.26 pinned everywhere (#259 → main `cc322f8e`) | DONE — CI green, dōjō `/version` 0.12.1 |
| dōjō: tenant calls session-only, `device_token_hash` dropped, round trip live (401/403) | DONE `21647161` |
| dōjō: `/v1/you/memberships` carries routing fields | DONE `9dc4bd83` |
| daemon: unbound → SignedOut, `await_reply` stops on SignedOut | DONE `33f08a6c` |
| app: connections screen → sign-in, load error ≠ empty | DONE `52893422` |
| daemon: sync joins memberships, POST 405, `credential_ref` dropped | DONE `d54ffed0` |
| knowledge sources hold no key | DONE `cdf2910c` |
| docs/spec | DONE `9cae35fd` |

**Next command:** `make bump v=minor` from develop once the user approves 0.13.0
(merges develop → main → dōjō deploys 4e at once).

**Open questions (user's):**
- A CLI-only `brew upgrade` never deploys the schema (only first install and the
  app's upgrade flow do) — recommend a boot-time deploy on version change.
- `deploy-dojo` jobs for v0.12.0 / v0.12.1 await environment approval.
- Cloudflare Workers Builds (branch) is bound to a wrongly named Worker.

**Known-broken / not run:**
- App e2e (`app/e2e/tests/dojo-binding.spec.ts`) updated but not executed.
- Live join not exercised end-to-end (needs dōjō ≥ `9dc4bd83`); unit + stub tests only.
- This machine's `personal/jerry` membership is unbound and its persona signed
  out: after 0.13.0 it needs a sign-in.
