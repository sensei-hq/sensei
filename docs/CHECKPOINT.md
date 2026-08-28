# Checkpoint

**Slice:** Repository sharing — **DESIGN ONLY. Nothing is implemented.**
(`docs/requirements/repository-sharing.md`, `daemon-sync.md` §8a/§8b/§8c,
`docs/architecture/reason-codes.md`)

## The model

Sharing is TWO questions: **entitlement** (*may it?* — the dōjō) and **election**
(*did whoever holds authority choose it?*). `sync_enabled = may_share AND elected`.

Authority: personal → user · org PUBLIC → user · org PRIVATE → **the
organization, mandatory**, overriding the daemon's local gate 1 (user-confirmed).

## Status: FOUR adversarial reviews, all NOT-READY

Depth · claims · data-correctness · security. Roughly 40 findings; most are fixed
in `2d659e02`…`HEAD`. **Two decisions are outstanding and are the user's:**

1. **Disclosure scope for B1.** To find out whether a repo is org-mandated the
   daemon must register repos the user has NOT elected — the daemon-side mirror
   of the sign-in inventory upload this design explicitly REJECTS. Unresolved.
2. **Personal + private + no subscription.** §IV.3 says `origin='personal' →
   ALLOW` unconditionally; §2a says private repos are subscription-gated. The two
   disagree, and every personal tenant is the common case.

## Still open (verified, not yet fixed)

- `configurable_by_me` grants `lead`; `member_role.ddl` gives policy to `admin`
  alone, and every remedy string says "ask an admin".
- §8c's "re-derived in four places" names two that do not exist (console, UI
  toggle) and omits two that do (`unpushed_metric_rows`, `unpushed_metric_count`).
- The blast-radius count is wrong a THIRD time: the list enumerates 3+12=15.
- `seats_included = 0` means both "unbounded" and "cap of zero".
- Section order runs 8 → 9a → 8a → 8b → 8c → 9.
- B1 and B2 are documented, not fixed — `dojo_sync.rs:124` and `sync.rs:182`.

## Build order (load-bearing)

schema + ALTER → sign-in capture → **backfill, verified to return 0** → view →
daemon (B1/B2). Rewriting the view first makes every org repo silently mandated:
the unpopulated default reads as `private`, and `sensei-hq/dbd` — public on
GitHub — resolves to ORG-MANDATED today.

## Next command

```
rg -n 'Disclosure scope|personal.*private' docs/spec/dojo/daemon-sync.md
```

## Carry-forward

- kavach repinned to the published **1.1.3**; no local patch remains.
- `~/.sensei/config.json` `dojo_url` → `http://127.0.0.1:5173` (backup at
  `/tmp/sensei-config-backup.json`). `dbd` shared, `dojo_sync` at 60s, debug
  binaries installed.
