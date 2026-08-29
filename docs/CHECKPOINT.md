# Checkpoint

**Slice:** Repository sharing — **BUILT.** `d27ffa1f` (capture + view + daemon
gates) and `75b00683` (the `repo` scope).
(`docs/requirements/repository-sharing.md`, `docs/architecture/reason-codes.md`)

## The model

`sync_enabled = may_share` (entitlement, dōjō) **AND** `elected` (whoever holds
authority). Authority: personal → user · org PUBLIC → user · org PRIVATE →
**the organization, mandatory**.

## Two holes found in the workflow's output and closed

1. **B2 was a regression as delivered.** It removed the daemon's local push gate
   on the premise "the dōjō re-decides entitlement at the write". That was
   **false** — `ingestMetrics` checked membership and nothing else, so the slice
   removed the only gate. It now reads `dojo.all_my_repositories`.
2. **`myMemberships` omitted `disabled_at is null`** — a REVOKED member still
   wrote `visibility`, which decides which authority governs sharing for
   everyone left.

## The `repo` scope

Without it capture cannot work: GitHub answers **404, not 403**, so every
private repo stayed permanently uncaptured and never synced. Added at both
sign-in paths, pinned by a drift test — which caught that scopes are
**space**-delimited (a comma grants nothing, silently). Backlog: a GitHub App's
`metadata: read` is the real fix; **do not close it by deleting the scope**.

## Live state (verified, not inferred)

`sensei-hq/dbd` · org tenant · visibility NULL → `sync=false`,
`forge_visibility_unknown`, remedy "Sign in again to refresh". A live provision
returned `forge_unreachable` and **wrote nothing** — GitHub confirmed "Bad
credentials" directly, so fail-closed is verified, not assumed.

Persona/slot (flagged twice) is **fixed**: `label=sensei-hq-org`,
`session_slot=default`; `signed_in_personas()` returns the slot, not the label.

## Next command

```
# mints a repo-scoped token — the only way to verify the positive capture path
open http://127.0.0.1:5173/signin
```

## Blocked on the user

- The GitHub sign-in above.
- **Election write path** — 0 rows in `dojo.repository_elections`, no writer
  anywhere, so scenarios A and D cannot happen. A gap in my planning.

## Still open

PostgREST 1000-row cap on the repositories read · 404 leaves a stale value
standing · view ACL dropped by `drop view` · `configurable_by_me` grants `lead`
vs `admin` · §8c's "four places" wrong · `seats_included = 0` ambiguous.

**Env:** wrangler dev restarted on :5173 with a fresh build; a local magic-link
session was minted. Gates: daemon 2483 · clippy 0 · fmt 0 · dōjō 1462 · check 0/0.
