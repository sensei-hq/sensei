---
description: Audit a capability for OWASP, NFR, and code quality issues
argument-hint: Capability name (omit to audit everything)
---

Read `docs/reverse-engineer.md` in full, then execute it with:

mode=audit capability=$ARGUMENTS

If $ARGUMENTS is empty, audit ALL capabilities under `openspec/specs/` plus `specs/common/` and all DB docs.

Follow the workflow exactly as specified in reverse-engineer.md:
- OWASP 2021 security checks
- NFR coverage across 6 dimensions
- Code quality health score
- Apply backlog ID schema under BL-<SCOPE>-*
