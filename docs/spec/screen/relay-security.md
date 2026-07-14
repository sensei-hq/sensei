# 鍵 · Relay · Security

**Segment:** 05 · Relay (mobile companion)
**Route:** Relay iOS app · security & permissions (no web route — phone app)
**Source mockup:** [`lib/relay/relay.jsx`](../../mockups/Sensei/lib/relay/relay.jsx) → `RelaySecurity`
**Data:** greenfield — reads pairing + permission state held by the coordinator/relay for this phone. Proposed: `GET /api/relay/pairing` → `{ device: {machine, state:'paired', e2e: true}, permissions: {approve_commands, send_replies, receive_file_contents}, notifications: {push_when_needed} }`; `PATCH /api/relay/pairing/permissions` to toggle a scope; `POST /api/relay/pairing/new` opens the [Pairing](relay-pairing.md) round-trip for another machine.
**App file:** _greenfield — mobile app not built_
**Daemon files:** _greenfield — coordinator + relay transport not built_
**Status:** greenfield (Relay backend unbuilt) — see [architecture/relay](../../architecture/relay.md)

## Purpose

This is where the user sees — and controls — the trust boundary. It states
plainly that the phone is **paired** with a named machine over an
**end-to-end-encrypted** channel, draws the actual path (*Your Mac → 🔒 relay
→ iPhone*), and makes the one promise that underwrites the whole product:
**"Relay only sees the status you publish — never your code, keys, or files."**
Below that, **scoped, revocable permission toggles** bound exactly what this
phone may do — approve commands, send replies, receive file contents — each
independently switchable. The user leaves this screen certain about what
leaves the machine and confident they can revoke it. It's the theme-5
"the user controls what leaves the machine" surface for the relay.

## Data invariants

- The paired-device card reflects real coordinator/relay state: the paired
  `machine` name, `state == 'paired'`, and `e2e == true` (end-to-end
  encrypted). These are read from the pairing record — never hard-coded
  "encrypted" reassurance with no backing key exchange.
- The **zero-knowledge promise is a literal, load-bearing statement**, not
  copy: "Relay only sees the status you publish — never your code, keys, or
  files." The permission model must actually enforce it — the relay carries
  filtered status, and `receive_file_contents` is the *only* scope that could
  widen that, so it is off by default and separately gated.
- The three permission scopes are independent, revocable booleans:
  - **Approve commands** — this phone may resolve approve gates (default on).
  - **Send replies to the agent** — this phone may answer decide gates (default on).
  - **Receive file contents** — this phone may receive file bodies over the
    relay (default **off**; turning it on is the one setting that relaxes
    zero-knowledge, so it must be an explicit, reversible opt-in).
- Toggling a scope `PATCH`es the coordinator's permission record; the change
  is immediate and revocable — revoking approve-commands means the phone can
  no longer resolve approve gates from that moment.
- **Notifications** (push when an agent needs me) is a device concern, shown
  as its own group, distinct from the trust/permission scopes.
- **Pair another machine** launches the [Pairing](relay-pairing.md) round-trip;
  one phone may be paired to multiple machines (R7), each its own encrypted
  pairing.

## Signals shown

Paired-device card (dark ink card):

| Element | Value | Meaning | Example |
|---|---|---|---|
| Machine | `pairing.machine` (mono) | the paired machine | `macbook-pro` |
| State | `Paired · end-to-end encrypted` (jade) | live pairing + E2E (R6) | — |
| Path | `Your Mac → 🔒 relay → iPhone` | the actual encrypted route | — |
| Promise | "Relay only sees the status you publish — never your code, keys, or files." | the zero-knowledge guarantee (R5) | — |

Permission scopes ("This phone can"):

| Element | Value | Meaning | Example |
|---|---|---|---|
| Approve commands | toggle (on) | may resolve approve gates | on |
| Send replies to the agent | toggle (on) | may answer decide gates | on |
| Receive file contents | toggle (**off**) | may receive file bodies — relaxes zero-knowledge | off (default) |

Notifications + pairing:

| Element | Value | Meaning | Example |
|---|---|---|---|
| Push when an agent needs me | toggle (on) | be pulled in at a gate/stall (R3) | on |
| Pair another machine | dashed accent button → [Pairing](relay-pairing.md) | add a machine (R7) | — |

Real mockup content: paired `macbook-pro`, `Paired · end-to-end encrypted`,
path `Your Mac → relay → iPhone`, the exact promise string; permissions
`Approve commands` (on) / `Send replies to the agent` (on) / `Receive file
contents` (off); notification `Push when an agent needs me` (on); `Pair
another machine`.

## Done gate

The screen shows the real pairing (named machine, paired, end-to-end
encrypted) with the drawn path and the literal zero-knowledge promise, plus
three independently revocable permission toggles — approve commands, send
replies, receive file contents (off by default) — a notifications toggle, and
a Pair-another-machine affordance. Every toggle writes through to the
coordinator's permission record and takes effect immediately.

## Wrong gate

- **"End-to-end encrypted" is shown with no real key exchange behind it** —
  reassurance copy without a backing pairing record is a lie the user will
  trust.
- **`Receive file contents` defaults on**, silently relaxing the zero-knowledge
  boundary the screen promises (R5). It must default off and be an explicit
  opt-in.
- **A revoked permission still works** — toggling off approve-commands but the
  phone can still resolve approve gates. Revocation must be immediate.
- **The zero-knowledge promise is decorative** while the relay actually carries
  code/transcripts by default — the statement and the enforcement disagree.
- **Permissions aren't scoped/independent** (one master switch instead of three
  revocable scopes) — R6 requires explicit, revocable, scoped permissions.
- **Pair-another-machine overwrites the existing pairing** instead of adding a
  second one — multi-machine (R7) is broken.

## Related

- Objectives [R5](../../requirements/objectives.md#relay--supervising-long-runs-from-anywhere) (zero-knowledge by construction), [R6](../../requirements/objectives.md#relay--supervising-long-runs-from-anywhere) (secure pairing + explicit, revocable permissions), [R7](../../requirements/objectives.md#relay--supervising-long-runs-from-anywhere) (multi-machine)
- [architecture/relay](../../architecture/relay.md) — security model (zero-knowledge relay · encrypted pairing · scoped revocable permissions; prefer kavach / Dōjō auth stack over hand-rolled)
- [journeys/relay](../../journeys/relay.md) — onboarding: pair once → grant permissions
- Sibling relay screens: [Pairing](relay-pairing.md) (the round-trip this launches) · [Dashboard](relay-dashboard.md) (lock chip routes here) · [Approve](relay-approve.md) · [Respond](relay-respond.md) · [Task detail](relay-task-detail.md)
