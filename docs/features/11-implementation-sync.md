---
id: implementation-sync
type: feature
---

# Implementation Sync

After implementation, documentation falls behind. Plans contain rationale and flows that belong in design docs. Code changes land without updating traceability. llms.txt goes stale. Implementation Sync closes this gap: a skill and lightweight hook that reads what was built from the plan and git history, enriches design docs with implementation context, updates the traceability matrix, and keeps llms.txt current — without copying code into docs.

## Features

### Post-Implementation Doc Enrichment

After a plan is executed, the plan's rationale, flows, and architectural decisions are distilled into the relevant design docs as a permanent implementation record.

```gherkin
Feature: Post-Implementation Doc Enrichment

  Scenario: Developer syncs docs after completing a plan
    Given a completed implementation plan at docs/plans/2026-03-11-indexer.md
    And a git diff showing which files changed
    When the developer runs sensei sync
    Then each affected design doc gains an "## Implementation Notes" section
    And the section contains: approach taken, key flows, and file/function references
    And no code snippets are copied into the design doc
    And the plan's step-by-step instructions are NOT included

  Scenario: Implementation notes reference specific code locations
    Given a design doc for the indexer
    And src/indexer/index.ts was modified during implementation
    When sensei sync runs
    Then the Implementation Notes section includes: "src/indexer/index.ts:42 — entry point"
    And these are file+line references, not code excerpts

  Scenario: Design doc without implementation notes gets a new section
    Given a design doc with no prior Implementation Notes section
    When sensei sync runs for a plan that covers this design
    Then "## Implementation Notes" is appended after the Testing Strategy section
    And existing doc content is unchanged

  Scenario: Second sync merges, does not duplicate
    Given a design doc that already has an Implementation Notes section from a prior sync
    When sensei sync runs for a follow-on plan covering the same design
    Then new notes are merged into the existing section
    And there is no duplicate Implementation Notes heading
```

### Traceability Status Sync

The traceability matrix is updated from the git diff and plan — no manual status editing required.

```gherkin
Feature: Traceability Status Sync

  Scenario: Completed plan items are marked done
    Given docs/traceability.yaml with items at status: planned
    And a plan whose steps are all checked off
    When the developer runs sensei sync
    Then each feature item covered by the plan is updated to status: done
    And items not covered by the plan are unchanged

  Scenario: Code entries added for changed files
    Given src/indexer/index.ts was created or modified in the git diff
    And the file is not yet in docs/traceability.yaml under code:
    When sensei sync runs
    Then a code entry is added:
      src/indexer/index.ts:
        implements-design: [indexer]
        status: done
    And the implements-design is inferred from the design frontmatter implements list

  Scenario: Changed file with no design coverage is flagged
    Given src/utils/helper.ts was modified
    And no design doc has implements that maps to this file
    When sensei sync runs
    Then a warning is printed: "src/utils/helper.ts has no design coverage — consider updating traceability.yaml"
    And the sync proceeds without failing
```

### Plan Archival

Completed plans are moved out of the active plans directory to keep the workspace clean.

```gherkin
Feature: Plan Archival

  Scenario: Completed plan is archived
    Given a plan where all checkboxes are checked
    When the developer runs sensei sync
    Then the plan file is moved to docs/plans/archive/
    And a one-line summary is appended to docs/plans/archive/index.md
    And the original plan path no longer exists

  Scenario: Partially complete plan is not archived
    Given a plan where some checkboxes are unchecked
    When the developer runs sensei sync
    Then the plan file is not moved
    And a message is shown: "Plan not archived — N steps remain incomplete"

  Scenario: Developer previews what will be archived
    Given one complete and one partial plan
    When the developer runs sensei sync --dry-run
    Then the output shows what would be archived and what would be updated
    And no files are changed
```

### Incremental llms.txt Sync

llms.txt is updated for only the files that changed — no full rebuild required.

```gherkin
Feature: Incremental llms.txt Sync

  Scenario: llms.txt updated for changed files only
    Given a repo with a current llms.txt
    And 3 source files changed in the last commit
    When the developer runs sensei sync --incremental
    Then only the sections of llms.txt covering those 3 files are regenerated
    And unchanged sections are preserved exactly
    And the run completes in under 5 seconds for a 500-file repo

  Scenario: New file added to llms.txt
    Given a new file src/cache/response-cache.ts was created
    When sensei sync --incremental runs
    Then a new entry for response-cache.ts is added to llms.txt
    And the rest of llms.txt is unchanged

  Scenario: Deleted file removed from llms.txt
    Given src/legacy.ts was deleted
    When sensei sync --incremental runs
    Then the entry for src/legacy.ts is removed from llms.txt
    And no stale entries remain
```

### Drift Warning on Sync

Changed files that have no traceability coverage surface as warnings, not failures.

```gherkin
Feature: Drift Warning on Sync

  Scenario: Untracked changed file produces a warning
    Given src/utils/parser.ts changed in the git diff
    And no design doc implements any item referencing this file
    When sensei sync runs
    Then a drift warning is printed for src/utils/parser.ts
    And the warning suggests which design doc to update
    And the sync completes successfully (non-blocking)

  Scenario: All changed files are tracked — no warnings
    Given every changed file maps to at least one design doc via traceability.yaml
    When sensei sync runs
    Then no drift warnings are printed
    And the sync completes with: "All changes tracked."

  Scenario: Developer runs sync in CI to detect drift
    Given sensei sync --fail-on-drift in a CI pipeline
    And one changed file has no traceability coverage
    Then the CI step fails with the drift warning
    And the diff of untracked files is included in the output
```
