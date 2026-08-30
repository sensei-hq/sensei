# Checkpoint

**Slice:** Forge-token lifecycle — **DONE, verified live.**
`92c21577` · `e039601b` · `e4202b90` · `7ed7be0c` · `9e48b9c7`

## The design, after Jerry rejected the first one

`e4202b90` copied the GitHub App client secret into the Cloudflare Worker so the
dōjō could redeem refresh tokens. Jerry caught the maintenance cost: recreating
the client id would mean updating two dashboards, and the missed copy fails
silently months later. **I should have raised that before building it.**

Checked whether the secret could be read from Supabase instead — it cannot.
`auth.custom_oauth_providers.client_secret` has 0 rows (custom OIDC only),
`vault.secrets` is empty, and provider config is control-plane, unreachable by
`service_role`. Only the Management API exposes it, which needs a strictly more
powerful token.

So renewal now re-runs the **authorize flow Supabase already owns**. One config
location, forever.

```
near expiry  ->  status: renewalDue=true
             ->  sensei auth renew-if-needed
             ->  Supabase /authorize (holds the secret)  ->  GitHub  ->  callback
             ->  fresh 8h token, expiry recorded by observe
```

**Silent re-auth proven: token replaced in ~6s, zero prompts, zero clicks.**

## Live-verified

- `renewalDue` across all three states (7h out: false · 25m out: true · dead:
  false + needsSignIn true)
- dōjō stopped → reports the outage, opens **no** browser
- GitHub measured: 8h access token, 182d refresh, client id `Ov23…` = GitHub App

## Bugs found by running it, not by testing it

- step 5's fields landed on the sign-in callback, not `status`
- `Refresh` fired for any unexpired token → would rotate ~16×/lifetime
- CLI opened a browser on a GoTrue **504** — `AuthError` knew rejected-vs-
  unreachable and `status` never put it on the wire
- a renewal that worked still read `renewalDue` — sign-in discarded the expiry
  header it was already receiving

## Not done

**Desktop app has no auth surface at all** — nothing there calls
`/api/auth/signin`. Renewal today is `sensei auth renew-if-needed`.

**Unreproduced flake:** one full-suite run failed with 1 test; name lost to a
pipe. 3 full + 18 targeted reruns clean.

**Gates:** daemon 2569 exit 0 · cli 57 · clippy 0 · fmt 0 · dōjō 1514 · check 0/0.
