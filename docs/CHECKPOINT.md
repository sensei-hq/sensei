# Checkpoint

**Slice:** Repository sharing — **VERIFIED LIVE**, not just built.
`d27ffa1f` · `75b00683` · `849fa070` · `1182ab4b` · `b0b68f62` · `76bc040b` · `7b0f53ce`

## The model, on real data

```
corpus   private  authority=organization (MANDATED)  -> not_subscribed
dbd      public   authority=user  elected            -> SYNCING · 132 metrics · 0 pending
gateway
sensei   public   authority=user                     -> not_elected_user
torii
```

Both axes independent; every refusal names itself. `not_subscribed` is the gate
that used to fail **open**.

## `repo` scope proven at the source

GitHub reports `read:org, repo, user:email` for the daemon's token, and it can
see `sensei-hq/corpus` — a **private** repo that would have 404'd before.
Capture recorded it `private`, which is what makes the org mandate reachable.

## Self-heal (`7b0f53ce`)

`refreshForgeVisibility` ran only at **sign-in** while repos keep registering via
`dojo_sync` — so 4 of 5 sat at `forge_visibility_unknown` with remedy "sign in
again", `corpus` included. The daemon holds the token, so it now asks when the
plan's denials say the forge was never consulted. Narrow by design: only
`forge_visibility_unknown|stale`. **Verified live: 4 uncaptured → 0 in one tick.**

## Also fixed this session

- **Election write path** (`849fa070`) — nothing wrote `repository_elections`, so
  every user-authority repo was permanently `elected = false`.
- **Auth-id vs principal-id** (`1182ab4b`) — "My dōjōs" was empty for *everyone*
  and `hasMembership` false, so real members were treated as solo.
- **Concurrent provisioning forked tenants** (`b0b68f62`), 2ms apart.
- **employer/client/community tag dropped** (`76bc040b`).

## Disclosure, measured

67 repo_keys are **transmitted**, only **5 stored** — 0 rows outside a connected
tenant. An earlier warning of mine (that employer paths become rows) was wrong.

## Next

**The election UI.** `setElection` + `PATCH /v1/you/repositories/election` are
proven over HTTP, but nothing renders a toggle — a user cannot elect
gateway/sensei/torii without curl.

Then: PostgREST 1000-row cap on the repositories read · a 404 leaves a stale
value standing · view ACL dropped by `drop view` · `configurable_by_me` grants
`lead` vs `admin` · `seats_included = 0` ambiguous · health `uptimeSeconds`
reports 8.6h for a 28-minute-old process.

**Gates:** daemon 2492 exit 0 · clippy 0 · fmt 0 · dōjō 1488 · check 0/0.
