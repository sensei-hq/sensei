---
name: Workflow System — Documentation Disposition
description: Plan for reorganizing existing docs/ folders — what to archive, keep, or update
date: 2026-04-17
parent: 01-workflow-system.md
---

# Existing Documentation — Disposition

The current `docs/` folder has accumulated artifacts from multiple architectural eras (SaaS → local-first → Rust rewrite). This is the plan for cleanup.

| Current location | Action | Notes |
|-----------------|--------|-------|
| `docs/superpowers/plans/` | Archive to `docs/_archive/superpowers/plans/` | 20 date-prefixed plan files from prior plugin-building phase |
| `docs/superpowers/specs/` | Archive to `docs/_archive/superpowers/specs/` | 23 date-prefixed spec files from prior plugin-building phase |
| `docs/superpowers/backlog.md` | Archive | Superseded by `/sensei:refocus` |
| `docs/design/` | Reorganize | Remove stale Supabase references. Keep architectural decisions that are still valid. Add note about evolution. |
| `docs/features/` | Reorganize | Good Gherkin-based template. Update to reflect current capabilities. |
| `docs/roadmap/` | Reorganize | Some docs still valid (Rust daemon plan), others stale (Supabase references). |
| `docs/gap-analysis.md` | Keep | Good model for what `/sensei:analyze` produces. Reference as an example. |
| `docs/templates/` | Update | Add templates for new phases: idea, analysis, blueprint, experiment, plan. Keep existing design and feature templates. |
