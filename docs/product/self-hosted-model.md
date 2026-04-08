# Product Model: Self-Hosted (Personal Supabase)

> **STATUS: SUPERSEDED** — See `docs/roadmap/` for the current direction (local-first Tauri app,
> SQLite, no Supabase dependency). This document preserved for historical reference.

---

## Vision

Sensei as a **locally installed, individually managed tool**. Each developer
brings their own Supabase project — either Docker-local or a personal cloud
instance on supabase.com. No shared platform, no org/team accounts, no billing.
The developer runs `sensei init`, points it at their Supabase project, and gets
codebase intelligence and session continuity entirely under their own control.

## What Stays the Same

The core architecture (`docs/design/01-architecture.md`) is unchanged:

- `sensei init` / `sensei index` scans and indexes into Supabase
- MCP server exposes `get_session_context`, `search`, `context_pack`, etc.
- Collector daemon captures session events from Claude hooks
- SvelteKit dashboard reads Supabase directly for analytics and FTR scores
- `.sensei/config.yaml` is the only on-disk config (Supabase URL + repo ID)

## What Changes vs the SaaS Model

| SaaS Model | Self-Hosted Model |
| ---------- | ----------------- |
| Shared cloud Supabase (multi-account) | Developer's own Supabase project |
| `core.accounts` / RLS / org invites | Not needed |
| Ingest API syncing session data | Not needed — collector writes directly to personal Supabase |
| Billing / plan tiers | Not needed |
| Platform admin view | Not needed |
| `account_id` column on every table | Not needed |
| Per-account encryption (KEK/DEK) | Not needed — developer owns the project |

## Setup Flow

```
1. supabase start                          # or use personal cloud project
2. sensei init                             # detects stack, runs first index,
                                           # writes .sensei/config.yaml,
                                           # generates CLAUDE.md + AGENTS.md
3. claude (or any MCP-enabled agent)       # picks up sensei MCP server automatically
```

## Sharing Between Team Members

Teams work with the same codebase but each member manages their own Supabase.
Sharing happens through:

1. **Git** — `.sensei/config.yaml` is committed (URL + repo ID, not secret).
   Each contributor substitutes their own Supabase URL on first setup.
2. **`.sensei/README.md`** — setup guide checked into the repo so onboarding
   is a single document to follow (see `../../.sensei/README.md`).
3. **Skills / hooks** — `sensei init` installs them; same behaviour for everyone.

No shared data store is needed for team collaboration on the codebase. Each
developer's session analytics are their own.

## Implementation Phases

### Phase 0 — Already built (local Supabase)
The existing implementation already supports personal Supabase. All packages
work against a single Supabase project with no `account_id` partitioning.

### Phase 1 — `sensei init` polish
Make `sensei init` idempotent, reliable, and cross-platform. This is the
primary onboarding surface for the self-hosted model.

### Phase 2 — Dashboard completeness
Ensure the SvelteKit dashboard covers all useful views for individual use:
FTR trends, skill usage, session costs, library drift, traceability matrix.

### Phase 3 — Distribution
Package `sensei` for easy install:
- `npm install -g sensei` / `bun add -g sensei`
- Homebrew formula
- GitHub Releases with pre-built binaries

### Phase 4 — Lightweight team mode (optional)
Before the full SaaS model, offer a lightweight team option:
each developer still has their own Supabase, but a shared read-only dashboard
can aggregate across multiple Supabase URLs. No shared write path.

## Open Questions

- [ ] Should the dashboard be bundled into the CLI (`sensei dashboard`) or
      remain a separate `apps/dashboard` SvelteKit dev server?
- [ ] Should `sensei init` offer Docker setup (run Supabase locally) or only
      accept an existing Supabase URL?
- [ ] What is the migration story if a team later wants the full SaaS model?
      (Answer: add `account_id`, run the migration from `docs/design/24-platform-architecture.md` §2.3)
