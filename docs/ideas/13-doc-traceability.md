---
name: Documentation Traceability
description: Link design docs to code, detect drift, enforce coverage, and lint documentation quality
date: 2026-04-17
status: idea
sources: features/06-documentation-traceability.md, design/13-traceability-matrix.md, design/07-drift.md
---

# Documentation Traceability

## Problem

Design docs go stale. Code evolves but docs don't follow. There's no way to know which docs are current, which are stale, and which code has no design docs at all. This erodes trust in documentation and leads the AI to make decisions based on outdated information.

## Current state

- Doc-to-code traceability: partial — records manual links in .llmspec.yaml; auto-detection during index
- Drift detection: partial — cross-references commits vs traceability; drift flagging works
- Enforcement modes (block on stale docs): not fully integrated
- Doc doctor (linting): planned, not implemented
- Auto-generated docs from code: planned, not implemented
- Traceability CSV export: exists

## What this idea covers

- **Traceability matrix**: bidirectional links between design docs and code symbols, maintained automatically during indexing
- **Drift detection**: flag when code changes break documented contracts; configurable enforcement (warn, block)
- **Doc doctor**: lint docs for completeness, accuracy, and consistency rules
- **Auto-documentation**: generate docs from code when no design doc exists, as a starting point for human review
- **Traceability visualization**: dashboard page showing coverage map (see idea 10)

## Open questions

- Should drift detection block commits or just warn? Configurable per project?
- How does this interact with the workflow system? `/sensei:validate` should include drift check.
- Is auto-generated documentation useful or does it create false confidence?
