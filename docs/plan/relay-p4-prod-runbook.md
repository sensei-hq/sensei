---
name: Relay P4 — prod activation runbook
description: Prod activation for relay P4 (Web Push + realtime). Schema/RLS/grants/publication + read-only validation DONE 2026-07-18. VAPID Web Push secrets remain — blocked on a Jerry-gated prod dojo-Worker deploy.
date: 2026-07-18
status: schema+RLS+realtime DONE & validated on prod · VAPID = Jerry (needs a Worker deploy)
---

# Relay P4 — prod activation runbook

P4 shipped on `develop` and released as **v0.4.0** to `main`. The **DB side** of prod
activation (relay schema, RLS, grants, realtime publication) is now **done and
validated against prod** (`db.lagwuqrtshjtlcuvjfnd.supabase.co`). What remains is the
**Web Push VAPID** config on the prod **dojo** Cloudflare Worker — blocked on a
prod Worker deploy, which is a Jerry call (see step 1).

## ✅ DONE on prod — 2026-07-18

- **Step 3 — dojo schema + relay RLS + grants** — applied via
  `dbd apply --scope dojo --with-policies --deps include -e prod` against the prod
  `DATABASE_URL`. (Not `reconcile`: this prod DB — the dojo cloud Supabase — has only
  `dojo._dbd_meta`, no project-level `public._dbd_meta`, so the project-level
  `reconcile` errors `relation "_dbd_meta" does not exist`. Scoped `apply` is the
  proven, purely-additive `create … if not exists` path — the same one that built the
  existing dojo schema here; it created the relay tables/enums + `push_subscriptions`
  + `notification_prefs` + `relay_inbox_seq_bump`, no-op'd the 25 existing tables, and
  did **not** execute `seed_global_dojo`, so no data changed.) Result:
  `Fresh install at v0 — 53 entities applied.`
- **Step 4 — realtime publication** — was a separate `supabase db push --linked`
  of `supabase/migrations/20260718000000_relay_realtime_publication.sql`.
  **No longer a step**: publication membership moved into `database/policies/dojo/
  relay_{sessions,segments,inbox}.sql` on 2026-08-25 and now rides `dbd deploy`.
  Dropping a table removes it from a publication and re-creating it does NOT put
  it back, so a `dbd reset` + redeploy used to leave Realtime silently delivering
  nothing until someone re-ran this migration by hand.
- **Step 5 — read-only validation on prod** — all invariants hold:
  | check | expect | prod |
  |---|---|---|
  | RLS enabled on relay_{sessions,segments,inbox} | 3 | **3** |
  | own-rows SELECT policies | 3 | **3** |
  | `authenticated` INSERT/UPDATE/DELETE grants | 0 | **0** |
  | `anon` grants on relay tables | 0 | **0** |
  | relay tables in `supabase_realtime` | 3 | **3** |

## ⏳ REMAINING — Web Push VAPID (Jerry)

Web Push needs three values on the prod **dojo** Worker: `PUBLIC_VAPID_KEY`,
`VAPID_SUBJECT`, `VAPID_PRIVATE_KEY` (the sender reads the last two via
`$env/dynamic/private`; the client reads `PUBLIC_VAPID_KEY`). None are set yet
(only `PUBLIC_SUPABASE_ANON_KEY` / `PUBLIC_SUPABASE_URL` / `SUPABASE_SECRET_KEY` are).

### Why this is blocked (and Jerry-gated)
`wrangler secret put` fails with *"the latest version of your Worker isn't currently
deployed."* The dojo Worker has **version uploads from 2026-07-18 (12:52 & 12:53) that
were never promoted** — its active deployment is still 2026-07-15. `secret put` refuses
until the latest version is the deployed one. Clearing that requires **deploying the
prod dojo Worker** — an outward-facing release of the web app. Whether to promote those
uploaded versions or deploy current `dojo/` source is Jerry's call (I won't push
unreviewed code to prod), so this whole step is handed off.

### Step 1 — deploy the dojo Worker (choose one)
```bash
cd /Users/Jerry/Developer/sensei-hq/sensei/dojo
bunx wrangler versions list                 # inspect the pending 07-18 uploads first
# either deploy current source:
bunx wrangler deploy
# …or promote a specific known-good version to 100%:
# bunx wrangler versions deploy <VERSION_ID>
```

### Step 2 — generate a fresh VAPID keypair + set all three secrets
Run **after** step 1. Self-contained: generates a fresh P-256 keypair, pipes each
secret to wrangler via stdin (**the private key is never printed**), and prints only
the public key at the end:
```bash
cd /Users/Jerry/Developer/sensei-hq/sensei/dojo
node --input-type=module -e '
import {webcrypto as c} from "node:crypto";
import {execFileSync} from "node:child_process";
const k   = await c.subtle.generateKey({name:"ECDSA",namedCurve:"P-256"}, true, ["sign","verify"]);
const pub = Buffer.from(await c.subtle.exportKey("raw", k.publicKey)).toString("base64url");
const d   = (await c.subtle.exportKey("jwk", k.privateKey)).d;
const put = (n,v) => execFileSync("bunx", ["wrangler","secret","put",n], {input:v, stdio:["pipe","inherit","inherit"]});
put("PUBLIC_VAPID_KEY", pub);
put("VAPID_SUBJECT",    "mailto:hi@sensei-hq.com");
put("VAPID_PRIVATE_KEY", d);
console.log("\nDONE — PUBLIC_VAPID_KEY (public, safe to share):\n" + pub);
'
```
(`VAPID_SUBJECT` can be any `mailto:` ops contact.) After this, Web Push is fully live
in prod; nothing else is required.

## Note — prod DB password exposure (minor)
While running dbd, its `--help` echoed the `DATABASE_URL` env value (clap prints env
defaults), so the prod DB password surfaced in the session transcript. It was already
in the local `.env`; no external exposure. Rotating the Supabase DB password is an
optional precaution.
