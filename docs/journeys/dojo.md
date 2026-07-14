# The Dōjō journey

> How a team extends the same loop **without leaking client work**. Distilled from
> [`../mockups/Sensei/Sensei Dōjō Journey Map.html`](../mockups/Sensei/Sensei%20D%C5%8Djo%20Journey%20Map.html).
> Traces to [objectives DJ1–DJ5](../requirements/objectives.md#dōjō--the-cross-cutting-team-layer);
> the layer design is [architecture/dojo](../architecture/dojo.md).

The Dōjō is a **cross-cutting layer**, not a fifth segment: the developer's flows
live **in the Observatory**, plus a web **console**. Every user logs in
(developer · maintainer · admin · lead); the developer's is both in-app *and* a
personal console view. Deploys **in-house or as SaaS**.

**Membership contexts** — one developer, many orgs: *Employer · Client orgs ·
Communities · Personal.* The org boundary is the **membership**; client-precedence
is the tiebreaker.

## Developer (in-app)

```mermaid
flowchart LR
    d1[Discover<br/>my org runs a Dōjō] --> d2[Authenticate<br/>SSO · OAuth · device-code]
    d2 --> d3[Bind project → org] --> d4[A finding forms<br/>memory / pattern / guard]
    d4 --> d5[Share upstream<br/>right attribution + confidentiality]
    d5 --> d6[Watch it travel] --> d7[Receive downstream<br/>approved org knowledge where I work]
```

## Maintainer (console)

```mermaid
flowchart LR
    m1[Open the queue<br/>grouped by scope, ranked] --> m2[Evaluate a candidate]
    m2 --> m3[Decide<br/>approve · revise · decline, with a trail]
    m3 --> m4[Set distribution<br/>who receives it] --> m5[Publish &amp; measure<br/>did it land + help]
```

## Org admin (console)

```mermaid
flowchart LR
    a1[Stand up<br/>our own infra] --> a2[Connect identity<br/>OIDC / SAML + git OAuth]
    a2 --> a3[Provision members<br/>right roles, fast] --> a4[Scopes &amp; policies<br/>hierarchy + attribution/confidentiality]
    a4 --> a5[Monitor<br/>keep the Dōjō healthy]
```

## Lead — client / engagement confidentiality (console)

> The role formerly called *client-lead* is now **lead**. It guards
> confidentiality on **client engagements** (the engagement concept is unchanged;
> only the role name shortened).

```mermaid
flowchart LR
    l1[Define engagement<br/>register a client, route correctly] --> l2[Anonymize, always<br/>share the lesson, never the source]
    l2 --> l3[No per-item review<br/>trust the universal strip] --> l4[Audit trail<br/>prove confidentiality held]
    l4 --> l5[Incident handling<br/>contain a near-leak fast]
```

## The lifecycle — what flows through it

```mermaid
flowchart LR
    c1[contribute] --> c2[accumulate] --> c3[triage] --> c4[approve] --> c5[distribute]
    c5 -.->|received downstream, opt-in pull| c1
```

## The principles

From the journey map — the non-negotiables that make the boundary exact:

- **Identity is the discovery** — you learn your org runs a Dōjō by authenticating.
- **Pull, never push** — downstream knowledge arrives by opt-in, never forced.
- **Never blind, always preview** — nothing leaves without showing what leaves.
- **One universal strip** — the same anonymization runs on every client lesson; a lesson that can't be stripped doesn't leave.
- **Specificity wins conflicts** — distribution follows priority ladders, not a rigid hierarchy.
