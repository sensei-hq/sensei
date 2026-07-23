# dojo2 — org consoles missing from the rebuild (for the designer)

> The dojo2 design (`lib/dojo2/`) reorganized the app around **NAV_YOU** (personal) +
> **NAV_ORG** (org). Comparing against the prior design + the **shipped, tested** `dojo/`
> app, several **org consoles were dropped** — confirmed an oversight, not intentional.
> These stay in the codebase; this note lists them so they can be **added back into the
> dojo2 IA + kit**. Each entry cites the existing mockup + the shipped route to bring forward.
>
> dojo2 NAV_ORG today = **Overview** (Home · Constitution · Projects) + **Admin** (Members &
> roles · Scopes & policies · Audit · Plan & billing). The items below are the gap.

## Suggested nav shape (role-scoped)

Per the ladder/role model (`docs/design/dojo-web.md` §3 — grants are additive), NAV_ORG should
grow two role groups + two admin items:

- **Govern (maintainer)** — Triage · Approvals · Knowledge *(+ Catalog)*
- **Clients (lead)** — Engagements · Incidents · Client audit
- **Admin (add)** — Identity & SSO · Health / Monitor

## The missing consoles

### 1. Triage — maintainer *(the contribution-review workflow)*
- **Does:** the queue of candidate learnings awaiting a maintainer decision, grouped + ranked by scope.
- **Source:** mockup `lib/dojo/dojo-maintainer.jsx` (`DojoTriage`, `DojoCandidate`); shipped routes `/console/triage` + `/console/triage/[signature]`.
- **Screens / blocks:** scope-grouped ranked candidate rows (origin · confidence · conflicts/dups badges); **candidate detail** — the learning/cause/context, evidence, conflict-diff, near-duplicate merge, a confidence ring, distribution-scope picker, and approve / revise / decline (high-impact → second-approver gate).

### 2. Approvals — maintainer
- **Does:** the second-approval queue for high-impact/safety-relevant candidates.
- **Source:** `dojo-maintainer.jsx` (`DojoApprovals`).
- **Blocks:** rows with the first approver + Review/Approve; honest empty state.

### 3. Knowledge (+ Catalog) — maintainer
- **Does:** the published library of adopted knowledge + prune policy; the shared skills/agents/commands catalog.
- **Source:** `dojo-maintainer.jsx` (`DojoKnowledge`); `dojo-extensions.jsx` (`DojoExtensions`).
- **Blocks:** prune-policy dropdown · Active table · Disabled/pending-pruning; catalog of extensions.

### 4. Engagements — lead *(client confidentiality)*
- **Does:** register a client engagement, route findings correctly, bind a project → engagement, and prove confidentiality held.
- **Source:** mockup `lib/dojo/dojo-lead.jsx` (`DojoClients`); shipped routes `/console/engagements` + `/console/engagements/[id]`.
- **Blocks:** engagements register; per-engagement artifact/compliance **audit + CSV/JSON export**; the confidentiality model (kept-vs-dropped, anonymized-code example, "share the lesson, never the source"); project-bind.

### 5. Incidents — lead
- **Does:** contain a near-leak fast; confidentiality incident handling.
- **Source:** `dojo-lead.jsx`; shipped route `/console/incidents`.
- **Blocks:** incident CRUD · incident-response/containment flow · retention · client read-access toggles.

### 6. Client audit trail — lead
- **Does:** the immutable confidentiality ledger proving what left and what was stripped (distinct from the admin action-audit dojo2 already has).
- **Source:** `dojo-lead.jsx` (`DojoAudit`); shipped `/console/audit` (lead floor slice).
- **Blocks:** immutable ledger with filters · retention · client read-access.

### 7. Identity & SSO — admin
- **Does:** connect org identity — OIDC/SAML SSO, SCIM provisioning, and GitHub / device-code identity mappings.
- **Source:** mockup `lib/dojo/dojo-identity.jsx` (`DojoIdentity`); shipped route `/console/identities`.
- **Blocks:** IdP config · SCIM · identity-mapping CRUD (GitHub org / magic-link / device-code → member).

### 8. Health / Monitor — admin
- **Does:** monitor the shared mind's health + audit-fed anomalies.
- **Source:** mockup `lib/dojo/dojo-admin.jsx` (`DojoMonitor`); shipped route `/console/health`.
- **Blocks:** throughput / adoption / leak-guard signal cards · contributions-vs-approvals chart · leak-guard alerts.

## Notes for the designer
- All eight are **already built + tested** in `dojo/` (routes above) and/or the `dojo-mind` backend — the task is to **bring them into the dojo2 IA**: give each a NAV_ORG destination (role-grouped as above) and re-skin to the dojo2 **kit** (`lib/dojo2/dojo2-kit.jsx` — `K2SectionHead`/`K2ListSection`/`K2RoleTag`/`K2Chip`/`K2Btn`/`K2Banner`/`K2EmptyState`, etc.).
- Keep them **role-scoped**: a viewer sees only the groups their role unlocks (maintainer → Govern; lead → Clients; admin → Admin). Personal (NAV_YOU) is unaffected.
- `Scopes & policies` (dojo2 `ScrScopes`) already covers the old **policies** grid + scope ownership; and dojo2's **Audit** tab covers the admin action-audit — so those don't need re-adding, only the eight above.
