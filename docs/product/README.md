# Sensei — Product Model Overview

This folder documents the product models considered for Sensei.

| Model | File | Status |
| ----- | ---- | ------ |
| Self-Hosted (Personal Supabase) | [self-hosted-model.md](./self-hosted-model.md) | **Active** — current direction |
| Multi-Account SaaS Platform | [saas-model.md](./saas-model.md) | **Deferred** — archived for potential future revival |

## Decision

As of 2026-04-08, Sensei pivots from a shared multi-account SaaS platform to a
**self-hosted, individually managed system**. Each developer brings their own
Supabase project (local Docker or personal cloud instance). No shared infra,
no org/team accounts, no billing.

The multi-account SaaS design is fully preserved in `docs/design/24-platform-architecture.md`
and summarised in `saas-model.md`. It can be revived when the use case grows to
require cross-team analytics or a hosted offering.

The `.sensei/` folder in each repo provides enough configuration for contributors
to reproduce the same local environment.
