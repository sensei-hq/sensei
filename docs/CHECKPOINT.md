# Checkpoint

**Slice:** Forge-token lifecycle — **all 6 steps DONE, verified live.**
`ee3ccf7a` · `f6c77a8a` · `a37bedf1` · `2afe37f6` · `92c21577` · `e039601b` · `e4202b90`

## What works, measured

```
sign-in ─┬─> observe()  captures state+expiry from calls made for other purposes
         └─> scheduled check (30m): Skip | Verify | VerifyAndMarkDead | Refresh
Refresh ──> POST /v1/you/forge/refresh (dōjō holds the client secret)
            └─> rotate + store both tokens ──> record active + new expiry
/api/auth/status: forgeToken {state, expiresAt} · needsSignIn
```

Live proof, whole chain: expiry nudged to 15:49 → ticker 15:29:56 → new expiry
23:29:56 (+8h) · refresh token `b897379b…`→`b803c860…` (rotated, stored) ·
`/api/auth/orgs` → `['sensei-hq']`.

**GitHub's real numbers:** `expires_in` 28800 (8h), `refresh_token_expires_in`
15724800 (182d), scope preserved. Client id `Ov23…` = a **GitHub App**, which is
why refresh tokens exist here at all.

## Corrections made this session

- Step 5's fields landed on the sign-in CALLBACK, not `status` — the endpoint the
  UI polls. Found by installing and curling, not by tests. Extracted to a shared
  `forge_report` so the two cannot disagree.
- `forge_token_action` returned `Refresh` for any unexpired token: with refresh
  implemented that is ~16 rotations per 8h lifetime. Now bounded to
  `REFRESH_MARGIN_SECS` (1h), asserted against the seeded interval.

## Not done — needs you

**Production `wrangler secret put GITHUB_OAUTH_CLIENT_SECRET`.** Until then the
deployed dōjō answers 503 and tokens die at 8h with no renewal. Local
`dojo/.dev.vars` is configured.

**Unreproduced flake:** one full-suite run failed with 1 test; name lost to my own
pipe. 3 full + 18 targeted reruns clean. Unidentified, so unfixed.

**Gates:** daemon 2572 exit 0 · clippy 0 · fmt 0 · dōjō 1527 · check 0/0.
