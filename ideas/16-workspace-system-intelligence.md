---
name: Workspace & System Intelligence
description: Multi-repo workspace management — system-level analysis, cross-repo contracts, conformance checking, and health scoring
date: 2026-04-17
status: idea
sources: features/10-system-intelligence.md, roadmap/03-workspace-model.md, roadmap/06-graph-intelligence.md
---

# Workspace & System Intelligence

## Problem

Real projects span multiple repos — frontend, backend, microservices, shared libraries, infrastructure. Sensei currently operates per-repo. There's no way to understand the system as a whole, check that repos conform to system-level architecture docs, or resolve cross-repo contracts.

## Current state

- Per-repo indexing: implemented
- Workspace definition: planned (named collections of repos with roles)
- Cross-repo indexing: planned, not built
- System-level analysis: planned, not built
- Architecture conformance: planned, not built
- System health scoring: planned, not built
- Project maturity levels: planned (Seed → Mature auto-derived)

## What this idea covers

- **Workspace model**: define a workspace as a collection of repos with roles (UI, API, data, infra)
- **Cross-repo contract resolution**: REST (OpenAPI), GraphQL, gRPC (Protobuf), async (Kafka/AsyncAPI), implicit contracts via topic matching
- **System-level analysis**: aggregate quality scores across repos, detect cross-repo gaps
- **Architecture conformance**: check per-repo patterns against system docs, flag deviations
- **System health scoring**: composite score from cross-repo metrics
- **Project maturity**: auto-derive maturity from coverage, test health, doc completeness

## Open questions

- How are workspaces defined? A config file at a parent level? A sensei-managed registry?
- What's the minimum viable cross-repo feature? Probably contract resolution between 2 repos.
- Should system intelligence be a premium/enterprise feature?
