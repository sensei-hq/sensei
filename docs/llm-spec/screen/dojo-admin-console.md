# 長 · Dōjō · Admin console

**Segment:** Dōjō (SaaS) — console
**Route:** `dojo.sensei-hq.org/{origin}/{org}/console/admin` OR self-hosted equivalent
**Source mockup:** [`lib/dojo-console.jsx`](../../mockups/Sensei/lib/dojo-console.jsx)

## Purpose

The org admin runs the Dōjō server, wires identity, provisions
members, sets scopes and policies, and monitors health.

Five stages:

| Stage | Kanji | Purpose |
|---|---|---|
| Stand up | 基 | Run a Dōjō on our infra (self-hosted) OR create a SaaS tenant |
| Connect identity | 鍵 | Wire SSO — no anonymous access |
| Provision members | 任 | Get people in at right roles fast |
| Scopes & policies | 規 | Define hierarchy + attribution / confidentiality rules |
| Monitor | 観 | Keep the hive-mind healthy |

Kanji is 長 — *elder / senior*.

## Data invariants

- Reads from `dojo.memberships`, `dojo.roles`, `dojo.identities`,
  `dojo.policies`, `dojo.events`.
- Role provisioning derives from git provider by default (see
  [[pipeline/dojo-lifecycle]] role provisioning) with admin
  overrides.
- Identity providers: SSO (OIDC / SAML), GitHub OAuth,
  device-code — see [[screen/observatory-dojo-connections]].

## Signals shown

| Element | Value |
|---|---|
| Members list | with role · git-derived role · last-active · override |
| Identity config | SSO status · GitHub app connection · device-code enrollments |
| Policy grid | attribution defaults per scope · confidentiality rules · retention windows |
| Health strip | connection count · queue depth · publish rate · error rate |
| Audit log | every admin action + who + when |

## Done gate

- Adding a member auto-provisions with git-derived role;
  overrides work.
- SSO login end-to-end works.
- Policy edits take effect on next batch.
- Health strip shows real numbers.
- Audit log persists every admin action.

## Wrong gate

- **A member without any role can read the queue.** Auth default
  wrong.
- **Policy edits don't propagate to the next batch.**
- **Audit log missing an admin action.** Non-repudiation
  broken.
- **Health strip masks a broken queue** — needs alerting.

## Related

- [[pipeline/dojo-lifecycle]] — role provisioning + policies
- [[screen/dojo-maintainer-console]] · [[screen/dojo-client-lead-console]]
