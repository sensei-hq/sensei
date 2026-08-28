---
name: Repository sharing
description: Who decides whether a repository's metrics reach the dōjō — the authority model behind gate 1 and can_sync
date: 2026-08-28
---

# Repository sharing — who decides

## The problem

Sharing a repository's metrics has been treated as **one** question with one
answer: *may this sync?* Everything built so far answers only that —
`can_sync` (§IV.3 of `dojo-auth-provisioning.md`) tests forge visibility, claim,
billing and seat, and `dojo.all_my_repositories` hardcodes `sync_enabled = true`
because none of it is wired yet.

That is the **entitlement** question. It is not the whole decision, and the piece
it omits is the one users actually feel:

> **Who is allowed to make the choice at all?**

For a developer's own repository the answer is obviously the developer. For an
employer's private repository on an employer's subscription it is obviously the
employer. Those are different authorities, and nothing in the system currently
records which one applies — so either the user can silently withhold their
employer's governance data, or the employer can silently publish the user's
personal work. Both are wrong, and today's code cannot express the difference.

## Two independent questions

| question | name | owner | inputs |
|---|---|---|---|
| **MAY it sync?** | entitlement | dōjō | forge visibility, claim, billing status, seat |
| **SHOULD it sync?** | **election** | *depends on the repository* | who holds authority, and what they chose |

A repository syncs only when **both** answer yes. Conflating them is what
produced a view that reports `sync_enabled = true` for everything.

## Authority — who may elect

Authority follows **who owns the code and who pays**, on two axes already in the
schema: `dojo.tenants.origin` (`personal | organization`) and
`dojo.repositories.visibility` (the forge's answer).

| owner | forge visibility | authority | why |
|---|---|---|---|
| personal | private | **user** | their code, their call |
| personal | public | **user** | their code, their call |
| organization | public | **user** | open source. The org is not paying for it, and a contributor's own metrics are their own |
| organization | private | **organization** | the org's code, the org's subscription, the org's governance obligation |

**Only the last row is mandated.** Everywhere else the user elects, and an
unelected repository does not sync no matter what the org would prefer.

## What a mandate means

For an org-private repository the organization's election is **binding**:

- The user cannot switch it off — not in the console, and not locally.
- **It overrides the daemon's local gate 1.** Until now the daemon "never even
  asks about a repo the user did not opt in". That remains true for every
  repository where the user holds authority, and stops being true for
  org-mandated ones.

This is a deliberate narrowing of a previously absolute promise, and it should be
stated plainly rather than discovered: **"nothing leaves the machine without local
consent"** becomes **"nothing leaves the machine without local consent, or an
organization's mandate on that organization's own private code"**.

What the user retains: they can decline to sign in, sign out, or not install
sensei. Authority over an employer's repository is not the same as authority over
the machine.

## What must be observable

**One query answers everything.** `dojo.all_my_repositories` is the single source
of truth — the daemon, the API and the UI read the same verdict, so they cannot
disagree. Per repository a user must be able to see:

1. the forge's answer (public / private / not yet captured)
2. who holds authority (them, or the organization)
3. what was elected, by whom, and when
4. whether **they personally** can change it right now (`configurable_by_me`)
5. if it is not syncing: **which question refused** (entitlement or election), a
   human-readable reason, and **what to do about it** — plus who can act, when
   that is not them
6. when it last actually synced

Item 5 is the one that has burned this project repeatedly: a denial that reads as
"nothing to sync" is indistinguishable from having nothing to sync. Every refusal
names itself.

**Reason codes are registered data, not string literals** (`dojo.share_reasons`),
carrying a precedence so that a repository failing several ways at once reports
the one to fix FIRST — rather than whichever SQL branch happened to run first.

### The general rule this establishes

For anything configurable, three things belong together: **the setting as a row**
(never a literal), **a listing that shows configured and default side by side**,
and **a registered human-readable reason** for the current state. Without the
third, "why is this off?" is answered by reading source code — which is how one
question ends up with four different answers in four consumers.

## Acceptance criteria

- [ ] A personal private repository does not sync until the user elects it, and
      syncs after.
- [ ] An org **public** repository does not sync until the **user** elects it —
      the org cannot elect it on their behalf.
- [ ] An org **private** repository on an active subscription syncs because the
      org mandated it, with the user's local setting `private`.
- [ ] The same repository does **not** sync when a billing row exists but is
      inactive (e.g. `past_due`).
- [ ] The same repository does **not** sync when there is **no billing row at
      all**. Split from the criterion above deliberately: absence is the common
      case (all live tenants), and a single "inactive" criterion is satisfiable by
      a `past_due` test while the absent case fails open — so the criterion
      written to catch the leak would have certified it.
- [ ] A repository not syncing reports whether entitlement or election refused,
      and which authority holds the election.
- [ ] Forge visibility is captured from the forge, never inferred from the
      remote URL or the owner's name.
- [ ] A repository whose forge visibility has **not been captured** does not sync,
      and reports that as the reason. It is neither treated as public (which would
      hand it to the user to elect) nor as private (which in an org tenant would
      make it org-mandated and share it with no election at all — verified as a
      real outcome of today's data, not a hypothetical).
- [ ] Changing forge visibility upstream (public → private) changes the
      authority on the next sync, and an election made under the old authority
      does not silently survive it.

## Out of scope here

The entitlement half's remaining machinery — `claimed_at`, `seat_allocations`,
billing period checks — is phase 2 of `dojo-auth-provisioning.md`. This document
defines the election axis and how the two combine; it does not re-specify
billing.
