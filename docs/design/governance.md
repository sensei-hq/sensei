---
type: design
---

# Governance — module

Behind-the-scenes design for the [Governance](../features/05-governance.md)
feature (the dōjō plane). The feature doc says what a developer, maintainer,
lead, or admin sees and does; this says how it resolves and where the code
lives.

## Scopes, namespaces & precedence

- Data model, not a hierarchy: `sensei.scopes` (the ladder — `general < user <
  organization < client < technology < team < project < repository`, each with
  a `level`) + `sensei.namespaces` (instances — `(organization, "Sensei-HQ")`,
  `(project, "sensei")`) + `sensei.folder_namespaces` (which repo belongs to
  which namespaces — a set, not a tree).
- Design rationale (parent_id tried and dropped — a repo needs *both* a
  personal and an org source at once): `docs/architecture/concepts/governance.md`
  §"Level, not parent".

## Rules — mandatory · scoped · promoted, resolved live

- A rule *is* a memory with three added columns: `namespace_id`, `enforcement`
  (`advisory|recommended|required|mandatory`), `origin`
  (`authored|promoted|remote`). No parallel rules table.
- Tier-1 deterministic resolution (gather → order by enforcement desc, scope
  level desc → mandatory-lock → dedup): `crates/senseid/src/governance.rs`
  (`structure_ruleset`, `RawRule`, `ResolvedRule`, `ResolvedRuleset`); DB gather
  in `crates/senseid/src/db/pg_store.rs` (`resolve_rules_raw`). Cached 5 min,
  runs on every `get_layered_context` call.
- Tier-2 LLM consolidation (cherry-pick merge on rule change / explicit
  refresh only, not hot path) routes through the existing `consolidation`
  inference role (`database/ddl/enum/sensei/inference_role.ddl`) →
  `gateway.fallback_chains`; draft requires user approval before becoming
  active (mirrors `history.past_memories` versioning).
- Rendering: `~/.sensei/rules.md` = always-on user + general scope + tool
  decision-guide; per-repo specifics resolve live via the `get_rules` /
  `get_layered_context` MCP tools (`crates/mcp/src/lib.rs`,
  `crates/senseid/src/api/handlers/knowledge.rs`).
- Promotion (`origin = promoted`): a `battle_tested` memory elevated to a
  higher scope; MCP surface `mcp__plugin_sensei_sensei__promote_memory`,
  `accept_playbook_rule`, `list_playbook_rule_proposals`.

## Identity, membership & routing

- Auth plane: `dojo/src/lib/server/dojo-auth.ts` (`resolveTenantAccess`,
  role/access-floor logic) — Supabase JWT → `dojo.memberships.user_id` → role →
  access floor (`member < contributor < lead < maintainer < admin`). *This is
  the Worker port of the removed `dojo-mind` Rust auth plane.*
- Membership types (employer · client · community · personal) bind a project
  to exactly one dōjō; a finding routes only there; client takes precedence —
  see `dojo.member_role` DDL comment and `docs/architecture/dojo.md`
  §"The boundary".
- Identity source: root `README.md` frontmatter (`organization`/`project`/
  `team`/`role`/`stack`) is the two-way sync anchor —
  `docs/architecture/concepts/governance.md` §"Identity from the repo".

## Promotion / triage lifecycle (contribute → distribute)

- Federation store + HTTP surface: the dojo Worker's `/v1` server routes
  (`dojo/src/routes/v1/…/+server.ts` — `/v1/t/{tenant}/rules`, `.../artifacts`,
  `.../triage`, `.../triage/promote`, `.../audit`, `compliance/export`). *Ported
  from the removed `dojo-mind` Rust service (`DojoStore`, `build_router`).*
- Auto-approve vs human triage — the promotion scoring (auto-approve score 0.8,
  per-contributor weight, FTR-delta credit, k-anonymity floor of ≥3 distinct
  contributors before a global-scope artifact can auto-publish, so a published
  finding can never de-anonymize a lone contributor). *Ported into the Worker
  from the removed `dojo-mind` `collective/promote.rs`.*
- SaaS console porting + the "one deployable, no separate Rust host" decision:
  `docs/architecture/dojo-deployment.md` §2 (the old `dojo-mind` `/v1` was ~80%
  CRUD, ported as SvelteKit routes; `engagements` is the reference pattern,
  `ccd08bc2`; the Rust crate is now removed).

## Confidentiality, anonymization & audit

- Origin-based stripping (open-source · personal-closed · employer · client)
  and the "universal strip" (client work is anonymized the same way every
  time, or it doesn't leave): `docs/architecture/dojo.md` §"Principles".
- Immutable audit trail + leak-guard: `/v1/t/{tenant_key}/audit`,
  `/audit/artifacts`, `/compliance/export` routes in the dojo Worker
  (`dojo/src/routes/v1/…`; ported from the removed `dojo-mind` service).

## Collective vs dōjō

- Two lanes on the same mechanism: the public opt-in **Collective**
  (`k-anonymity`-gated auto-publish, tenant scope = global) vs a private
  **dōjō** (org/client/team scope, no anonymity floor needed beyond the
  origin-based strip). Both flow through the same `promote.rs` scoring —
  distinguished by `dojo.tenant_scope`
  (`database/ddl/enum/dojo/tenant_scope.ddl`).

## SaaS vs self-host deployment

- One codebase, two deploy targets: sensei-hosted Cloudflare Worker
  (`dojo.sensei-hq.com`, SvelteKit + `/v1` routes same-origin, Supabase cloud)
  or in-house (org runs the same Worker + console on its own infra, data
  never leaves the company). Full wiring: `docs/architecture/dojo-deployment.md`.
- The `dojo-mind` Rust binary has been **removed** (retirement complete): the
  Worker `/v1` is the only dōjō backend, for the console and for senseid's
  federation (rules + artifacts over the `dojo_protocol` wire — see
  `crates/senseid/src/federation` and `crates/senseid/src/dojo`).

## Budgets & controls (future — not yet built)

- No `budget`/`spend` tables exist yet (checked: no hits under
  `database/ddl/` or the dojo Worker's `/v1` routes). The nearest present-day analog is
  Relay's per-tier metering note (device count, concurrency —
  `docs/architecture/dojo.md` §"Metering") and the plan-tier framing in
  [Pricing](../features/07-pricing.md).
- The "governance stance" concept — **autonomy · sharing · review ·
  anonymization**, authored per scope by an admin — is designed in
  `docs/journeys/dojo.md` §"Org admin (console)" and mocked in
  `docs/mockups/Sensei/lib/dojo/dojo-governance.jsx` (mock-only per
  the feature doc's status table). Cost/budget dials are not part of that
  stance today; extending it (spend ceilings per scope, alert thresholds
  feeding `dojo-billing.jsx`) is open design, not started.

## Local-model inclusion (future — not yet built)

- Which local models are *allowed/preferred* is currently a gateway concern,
  not a dōjō-governed one: `gateway.models.memory_gb` marks Ollama-local
  sizing (null = external-only) and `gateway.fallback_chains` /
  `fallback_chain_models` pick per-role model order
  (`database/ddl/table/gateway/models.ddl`,
  `database/ddl/table/gateway/fallback_chains.ddl`). Tier-2 rule
  consolidation already defaults to a local model via `gateway-embedded`
  before falling back to Opus (`crates/senseid/src/governance.rs` module doc +
  `docs/architecture/concepts/governance.md` §"Tier 2").
  Governing that choice *per dōjō scope* (e.g. an org mandating/forbidding a
  given local model, or preferring embedded inference for cost/privacy) has
  no schema or resolution path yet — it would most naturally land as a new
  `enforcement`-carrying rule namespace (same mechanism as any other governed
  rule) rather than a separate subsystem, but this is a design proposal, not
  shipped work.
</content>
