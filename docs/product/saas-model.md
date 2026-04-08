# Product Model: Multi-Account SaaS Platform

> **STATUS: DEFERRED as of 2026-04-08**
> Full design is preserved in `docs/design/24-platform-architecture.md`.
> This file is a summary with the revival checklist.
> Current active direction: see [self-hosted-model.md](./self-hosted-model.md).

---

## Vision

Sensei as a cloud-hosted analytics platform — like qlty or Code Climate but for
AI-assisted development. Developers install Sensei locally and run their normal
workflow. Sessions produce signals (FTR scores, token costs, tool usage, patterns)
that sync to a shared cloud platform organised by **account**. Teams see aggregate
quality trends; platform admins see anonymised cross-account signals to improve
the default coaching model.

## Full Design

See `docs/design/24-platform-architecture.md` for the complete design:

- Multi-account schema (`core.accounts`, `core.profile_accounts`, `core.account_keys`)
- Account types: individual, team, platform
- Row-level security across all 20 sensei tables
- Per-account encryption (KEK → DEK)
- Team invitations
- Ingest API + analytics engine + per-account dashboard
- Cross-account anonymised coaching signals

## Why It Was Deferred

1. **Complexity before product-market fit** — Auth, org/team model, billing, RLS
   migrations on all 20 tables, and the ingest API are months of work before core
   developer value is proven.
2. **Individual use case comes first** — The primary user is a solo developer who
   wants a smarter local assistant. They don't need a shared platform.
3. **Self-hosted is simpler to ship** — Each user brings their own Supabase; zero
   cloud infra to operate.
4. **No data trust issue** — Developers may be reluctant to sync session data
   (code symbols, tool calls) to a third-party platform early on.

## Reusable Artifacts (when revived)

All current packages are unchanged by this deferral:

- `packages/shared` — types, Supabase client, config schema
- `packages/engine` — scan, parse, index pipeline
- `packages/collector` — event daemon, hook scripts
- `packages/server` — MCP server, tool implementations
- `packages/cli` — all sensei commands
- `apps/dashboard` — SvelteKit dashboard (reads Supabase directly)

The SaaS layer adds multi-tenancy on top — it does not replace the existing packages.

## Revival Checklist

- [ ] Migrate existing single-tenant Supabase tables to `account_id`-partitioned (per `docs/design/24-platform-architecture.md` §2.3)
- [ ] Add `core.accounts`, `core.profile_accounts`, `core.account_keys`, `core.invitations` tables
- [ ] Auth: GitHub OAuth via Supabase Auth; JWT claims carry `account_id`
- [ ] Enable RLS on all tables; add `account_id` filter policies
- [ ] Ingest API: `POST /sync` endpoint; PII scrub before write
- [ ] Per-account analytics engine + dashboard views
- [ ] Platform admin view (anonymised cross-account signals)
- [ ] Billing integration
- [ ] `sensei init --cloud` flow (connect to platform vs local Supabase)
