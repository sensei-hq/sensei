# ADR — Supabase identity linking + optional sensei login

Status: **proposed** (your call). Extends
[`2026-08-24-platform-restructure.md`](./2026-08-24-platform-restructure.md).

---

## 1. First: this is not the reversal it looks like

There is a locked decision on record:

> **Fork 1** — `dojo.*` lives in the Dōjō Rust service's PG; **Supabase = auth ONLY**
> — `docs/spec/park/_dojo-build-plan.md`

and the daemon enforces it in comments:

> *"dual-plane auth: the daemon is credential-BEARING, using a Keychain-backed
> Bearer token — **never Supabase**"* — `crates/senseid/src/dojo/client.rs:8`
> *"humans use Supabase in the web console only"* — `api/handlers/dojo.rs:10`

**But the architecture has already drifted away from it**, and I'd rather show
you that than design as though the decision were intact:

| Evidence | Implication |
|---|---|
| `dojo/src/lib/server/projects-data.ts` → `.from('projects')`, `contributions-data.ts` → `.from('memberships')` | dōjō entity tables are being read **from Supabase**, not from the service |
| `dojo/src/lib/triage-data.ts` → `${dojoApiUrl}/v1/t/{tenant}/triage` | triage/artifacts still go **to the Rust service** |
| `crates/` = bootstrap, cli, dojo-protocol, logger, mcp, sensei-config, senseid | **there is no dōjō service crate in this repo** — only the wire types |
| `database/design.yaml:34` says *"dojo.\* lives in the Dōjō service's own Postgres"* and `:39` says *"Supabase gets them here"* | the config file contradicts itself |
| the source doc lives in `docs/spec/**park**/` | it was parked, not maintained |

So today dōjō is **split**: entity tables in Supabase, artifact/triage behind a
service that isn't in this tree. Your proposal doesn't reverse Fork 1 so much as
**finish resolving a fork that is already half-resolved**.

Flagging it explicitly because the project rules require deliberate deviations
from architecture to be raised and documented rather than quietly coded around.
If the Rust service is still live in a sibling deployment, that changes the
recommendation and I should know before we commit.

---

## 2. What Supabase identity linking actually buys

`auth.identities` gives natively what I proposed building:

```
auth.users        one row per human            → replaces my `people` table
auth.identities   (provider, provider_id) UQ   → replaces my `identities` table
                  identity_data jsonb          → provider email, name, avatar
supabase.auth.linkIdentity({ provider })       → Scenario 6, for free
```

That removes **two** tables from the plan and, more importantly, removes the
hard part: reconciling "same human, several providers" is Supabase's problem,
not ours.

It also fixes Gap 1 from the addendum at the root: `dojo.identities` today is
`unique(tenant_id, provider, subject)`, so one GitHub sign-in provisioning four
tenants makes four identity rows for one human. With `auth.identities` there is
exactly one row per provider per human, tenant-independent — which is the shape
the auth doc assumed all along.

### What it does NOT buy — and this is the important part

**Git commit emails are not auth identities.** Supabase can link `github_oauth`
to `google`; it cannot tell you that `dev@sensei-hq.com` in a commit trailer is
you.

**And linking is not always wanted.** The local data is *not* one human with six
aliases — it is two or three **personas** deliberately kept apart:

```
me@jerrythomas.name             422 / 26 repos / 2019-06-25 → 2026-08-24  ┐ personal
owner@example.com       84 / 2        / 2026-06-15 → 2026-08-21  ┘
hi@sensei-hq.com                108 / 2        / 2026-07-26 → 2026-08-24  ┐ sensei-hq
dev@sensei-hq.com                74 / 1        / 2026-06-13 → 2026-08-21  ┘
dev@example-corp.com    62 / 9        / 2018-09-08 → 2026-03-20  ← employer, ended
contributor@example.com        17 / 1        / 2025-05-24 → 2025-06-25  ← may be another human
```

In dōjō these are **separate logins on purpose** — sensei-hq work must not
surface under the personal account. Locally the user wants all of them visible,
tagged by persona.

This is the opposite of Supabase identity linking, and it matters: **`linkIdentity`
is the wrong tool here.** Linking merges providers into one `auth.users`; the
requirement is to keep two `auth.users` apart while a single local install serves
both.

So the model is:

```sql
create table personas (              -- LOCAL only
  id            uuid primary key default gen_random_uuid()
, label         text not null        -- 'sensei-hq' | 'personal' | 'example-corp'
, dojo_user_id  uuid                 -- auth.users.id; NULL until that persona logs in
, is_self       boolean not null default true   -- false = a contributor who is not you
, unique (dojo_user_id)              -- one login per persona, and never shared
);

create table persona_emails (
  persona_id  uuid not null references personas(id) on delete cascade
, email       citext not null
, source      text not null          -- 'git' | 'claimed'
, linked_at   timestamptz not null default now()
, removed_at  timestamptz
, primary key (persona_id, email)
);
create unique index persona_emails_live_unique
    on persona_emails(email) where removed_at is null;
```

`auth.identities` still earns its place — it removes provider-linking *within*
one persona (a persona that signs in with GitHub and later with Google). It just
must not be used to link *across* personas.

`unique(dojo_user_id)` is the schema making the privacy boundary structural: two
personas cannot resolve to the same dōjō account by accident.

### 2.1 What the Supabase docs actually say (researched 2026-08-24)

An earlier revision of this ADR described identity linking from memory. Checked
properly, and **one finding materially constrains the persona design**:

| Behaviour | Reality |
|---|---|
| **Automatic linking** | **ON, and cannot be turned off.** *"Supabase Auth automatically links identities with the same email address to a single user."* There is no configuration flag; Auth is architecturally built on *"all user emails are unique"*. A maintainer has declined to add a disable switch, citing backward compatibility and UX. |
| `GOTRUE_SECURITY_MANUAL_LINKING_ENABLED` | Commonly misread as the off-switch. It is **not** — it *adds* `linkIdentity()` on top of auto-linking, and is **disabled by default**. |
| Unverified emails | Auto-linking requires a confirmed email — *"it would also be an insecure practice to automatically link an identity to a user with an unverified email address"* (pre-account-takeover). |
| Destructive side effect | *"when a new identity can be linked to an existing user, Supabase Auth will remove any other unconfirmed identities linked to an existing user."* Linking **deletes** unconfirmed identities. |
| SAML SSO | Never a linking target, automatic or manual. |
| Unlink | Requires ≥2 linked identities; it does **not** split one user back into two. |

**The consequence for personas.** Two dōjō logins stay separate only while their
verified emails are **disjoint**. The moment `hi@sensei-hq.com` (or any address)
is verified on both GitHub accounts, Supabase silently merges them into one
`auth.users` — and there is no unlink that undoes it, because unlinking removes
an *identity*, not a merge.

So the separation you want is **not enforceable inside Supabase**. Three
responses, in order of preference:

1. **Keep persona emails disjoint** — operational discipline. It is the normal
   case (different GitHub accounts, different addresses) and costs nothing.
2. **Make the failure loud rather than silent** — `unique(dojo_user_id)` on
   `personas` already does this: if two personas ever resolve to the same
   `auth.users.id`, the insert fails and we surface it, instead of quietly
   attributing sensei-hq work to the personal account. Worth adding an explicit
   startup check with a clear message.
3. **Intercept** — override the callback or use a DB trigger on
   `auth.users.raw_app_meta_data->>'providers'` to reject the merge. Real
   complexity; only if (1) proves impractical.

**What this does *not* break:** the recommendation to use `auth.users` as the
login and `auth.identities` for provider linking still stands. Auto-linking is
exactly what you want *within* a persona (sign in with GitHub today, Google
tomorrow, same account). It is only *across* personas that it is a hazard, and
that hazard is bounded by email disjointness.

One incidental benefit: because **SSO identities never auto-link**, a client dōjō
on SAML SSO is structurally safe from cross-persona merging.

Sources: [Identity Linking](https://supabase.com/docs/guides/auth/auth-identity-linking) ·
[supabase/gotrue#313](https://github.com/supabase/gotrue/issues/313) ·
[supabase/auth#2472](https://github.com/supabase/auth/issues/2472) ·
[General configuration](https://supabase.com/docs/guides/auth/general-configuration)

### 2.2 Can a merged user be split later, keeping the data?

**The GitHub-account case is already safe.** Two Google OAuth logins with
different emails — `dev@example-corp.com` and `me@jerrythomas.name` —
produce **two separate `auth.users`**. Auto-linking matches on *email*, and
Supabase never learns that both Google accounts sit behind one GitHub user. The
shared GitHub account is invisible to it.

The merge window is narrower than it first appears: it opens only when one
identity's email **equals another user's verified email**. A GitHub sign-in whose
*primary* email is `me@jerrythomas.name` links to the Google user with that
address — which is usually what you want.

**Can Supabase split them afterwards? Effectively no.**

- `unlinkIdentity()` *"unlinks an identity from a user by **deleting** it"*, requires the user to hold **≥2 identities**, and afterwards *"the user will no longer be able to sign in with that identity"*.
- Signing in again with that provider **re-links straight back** to the same user if the email still matches and is confirmed — so unlink-then-signin loops rather than splits.
- To genuinely split you must first make the emails **not match** (change or remove the address on the retained user), then unlink, then sign in fresh to mint a new `auth.users`.
- Nothing carries history across. **There is no split-user operation.**

So your instinct is right: removing `dev@example-corp.com` from the
retained account is precisely the step that makes a split possible at all.

**But this only matters if we let it.** The decisive design rule:

> **Never key our data on `auth.users.id`.**

Use a stable internal principal — locally `personas.id`, and a dōjō-side
`principals` row — with `auth_user_id` as a **mutable pointer**, not an identity:

```sql
-- dojo side
create table principals (
  id            uuid primary key default gen_random_uuid()   -- ← what our data references
, auth_user_id  uuid unique                                   -- ← a POINTER; may be re-pointed
, label         text
);
-- tenant_users, team_members, repository_metrics … all reference principals(id)
```

A split then becomes **our** operation, not Supabase's:

1. user signs in fresh → a new `auth.users` exists
2. create a second `principals` row (or re-point an existing one) at it
3. re-attribute the affected rows from principal A to principal B — one
   transaction, on our side, reversible

And because `repository_metrics` **keeps the raw git `identity` email** (decided
earlier for a different reason — merging would have been destructive), the
re-attribution is a *re-derivation from immutable raw attribution*, not a
destructive edit. Rows move by re-resolving `identity → persona → principal`.

That is what makes the split lossless. The earlier decision to keep the raw email
alongside the resolved FK turns out to be the thing that buys this.

**Consequence for the plan:** add `principals` to the dōjō table set, and make
every FK that would have pointed at `auth.users` point at `principals` instead.
Cheap now, and the only alternative to it is being permanently unable to undo a
merge.

Sources: [unlinkIdentity](https://supabase.com/docs/reference/javascript/auth-unlinkidentity) ·
[Identity Linking](https://supabase.com/docs/guides/auth/auth-identity-linking) ·
[Discussion #18875](https://github.com/orgs/supabase/discussions/18875)

---

## 3. The security finding this exposes

**A git commit email is an unverified assertion.** Anyone can run:

```bash
git config user.email "someone.else@company.com"
```

Today that is harmless — user-scoped metrics are computed locally, about the
local user, and never leave the machine. The moment user-scoped metrics are
**pushed to dōjō and displayed per person**, it becomes an attribution attack: I
can make my commits appear as yours, or inflate a colleague's rework rate.

Therefore:

- **Local:** git email → person mapping is fine. One user, own machine, own data.
- **Shared:** a pushed `scope='user'` row must be attributed to the **authenticated** person (the JWT `sub`), *not* to whatever email git reported. The git email may travel as a property; it must not be the key.
- A git alias may only be **claimed**, never auto-linked: the person proves the address (verified email match against `auth.identities`, or an explicit claim reviewed by a tenant admin).

This is a design constraint, not a nice-to-have. It should be written into the
push policy before any user-scoped metric leaves a machine.

---

## 4. sensei logging into Supabase

The second half of your proposal, and the one with real leverage.

**Flow.** sensei is a Tauri desktop app, so the clean path is Supabase PKCE with
a loopback redirect: daemon opens the browser → user signs in (same account as
dōjō) → redirect to `127.0.0.1:<port>` → daemon exchanges the code → refresh
token into the OS Keychain, exactly where `dojo_memberships.credential_ref`
already puts device tokens. No new secret-storage story.

**What it collapses:**

| Today | With Supabase login |
|---|---|
| device token + custom `/v1/t/{tenant}/…` endpoints | user JWT + PostgREST |
| authorization written in service handlers | **RLS policies** |
| bespoke push/pull protocol (`dojo_outbox`, `upstream_queue`, `last_seq` cursors) | table upserts + `sync_state` watermark |
| "only repos accessible by the user" enforced in endpoint code | enforced by a policy on the §3 authorization path |

That last row is the win. The security requirement you set — *only repos
accessible by the user get the data* — becomes a policy the database enforces on
every query, instead of a check every endpoint must remember.

**What it costs:**

1. **No Rust Supabase client exists** (`crates/` has none). PostgREST is plain HTTP + JWT, so this is a thin client over the existing bounded `federation::http_client`, not a dependency problem — but it is new code.
2. **RLS becomes the only line of defence for writes.** The service could validate; a policy cannot easily express "this push is governed". Governance must move into policies, `CHECK` constraints, and triggers — expressible, but a different discipline. Anything a policy can't express (k-anonymity ≥3 for global tenants, the promotion scoring in `collective/promote.rs`) still needs a service or a Postgres function.
3. **The daemon holds a long-lived refresh token.** Keychain-backed and revocable, but a broader credential than a scoped device token: it *is* the user in dōjō. Worth scoping via a dedicated Postgres role/claim rather than the default `authenticated`.
4. **Offline.** Device tokens work against a self-hosted service; a Supabase JWT needs Supabase reachable to refresh. Self-hosted dōjō (`tenants.self_hosted`) already exists as a concept and would need its own Supabase or a fallback.

---

## 5. Recommendation

**Adopt both, with one boundary.**

1. **Use `auth.users` + `auth.identities` as the person model.** Drop my proposed `people` and `identities` tables. Keep `person_emails` narrowed to *claimed git aliases*, keyed to `auth.users.id`.
2. **Add optional sensei→Supabase login** (PKCE + loopback, refresh token in Keychain) and let **RLS carry read authorization** for the pull path.
3. **Keep a governed write path for anything a policy cannot express.** Metric rows are simple enough for RLS + constraints. Artifact promotion, k-anonymity and triage scoring are not — leave those where they are rather than reimplementing them as policies.
4. **Never key a shared user-scoped row on a git email** (§3).

That gets the simplification without betting the governance model on RLS
expressiveness.

---

## 6. Effect on the phase plan

| Phase | Change |
|---|---|
| **3** (people, aliases) | Smaller. `people` → `auth.users`; drop `identities`; `person_emails` becomes claimed-git-aliases. **Local half is unchanged and still ships first** — local metric unification needs no auth at all. |
| **6** (identity + tenants) | Much smaller. No identity reconciliation to build; provisioning still local, but writes go through Supabase under the user's JWT. |
| **7** (sync + governance) | Reshaped. Pull = PostgREST + RLS. Push = upsert under RLS + constraints, with the governed service path retained for artifacts. Adds: attribution rule from §3. |
| **new** | Supabase auth client in `senseid` (PKCE, loopback, Keychain refresh) — a Phase 6 prerequisite. |

Net: **two fewer tables and one fewer protocol**, at the cost of a Supabase auth
client and a policy-authoring discipline.

---

## 7. Questions this opens

- **Q13** Is the dōjō Rust service still live somewhere? If yes, does it keep artifacts/triage (my recommendation) or get retired? If it's dead, `dojo-protocol` and the daemon's `dojo/client.rs` are carrying a protocol nothing serves.
- **Q14** Self-hosted dōjō — does it bring its own Supabase, or does self-hosting keep the device-token plane? This decides whether the two auth planes can actually collapse into one.
- **Q15** Do we scope the daemon's JWT to a restricted Postgres role, or does it act as the full `authenticated` user?
- **Q16** Git-alias claiming — verified-email auto-match only, or admin review? (§3 says never silent.)

**Answers so far:** Q3 one repo/one project · Q7 all workers local.
