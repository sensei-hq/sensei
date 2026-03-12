---
id: patterns
type: feature
---

# Patterns

When agents implement a feature or fix a bug, they often discover or apply a pattern without capturing it. The next session — or the next developer — re-discovers it from scratch. Pattern identification gives agents the tools to detect patterns from existing code and docs, match new work against known patterns, and persist patterns as local repo skills that travel with the project.

## Features

### Pattern Detection

The indexer scans code and design docs to detect recurring structural patterns.

```gherkin
Feature: Pattern Detection

  Scenario: Indexer detects a repeated structural pattern
    Given a repo where 3+ components follow the same file layout
    When the agent calls reindex_repo()
    Then .sensei/patterns.md contains an entry describing the pattern
    And each entry includes: name, description, and example file paths

  Scenario: Pattern detected from design docs
    Given a design doc describing an adapter pattern
    When the indexer processes docs/design/
    Then the pattern is added to .sensei/patterns.md with a doc reference
    And the entry links back to the source design doc

  Scenario: Pattern detection skips one-offs
    Given a file layout used only once in the repo
    When reindex_repo() runs
    Then no pattern entry is created for it
    And only patterns with 2+ usages are recorded
```

### Pattern Templates

Named pattern templates give agents a canonical starting point for common structural choices.

```gherkin
Feature: Pattern Templates

  Scenario: Agent retrieves a pattern template by name
    Given pattern templates registered in .sensei/patterns.md
    When the agent calls get_pattern("tree-vs-flat")
    Then the response describes when to use a tree structure vs flat list
    And includes a concrete code example for each

  Scenario: Available templates cover common structural decisions
    Given the default pattern template set
    Then templates exist for: tree vs flat list, adapter, strategy, command, observer
    And each template includes: name, when to use, when NOT to use, example

  Scenario: Agent selects pattern for a new component
    Given a task to implement a new data structure
    When the agent calls find_pattern("hierarchical data with parent-child relationships")
    Then the response recommends the tree pattern
    And explains why flat list would not suit this case
```

### Pattern Capture

Agents capture patterns at the moment of recognition — not speculatively.

```gherkin
Feature: Pattern Capture

  Scenario: Agent records a new pattern
    Given a pattern the agent has just applied for the second time
    When the agent calls add_pattern("adapter-per-package", "Each package exposes a typed adapter. Consumer imports adapter, not internal implementation.")
    Then the pattern is saved to .sensei/patterns.md
    And future sessions can find it via find_pattern()

  Scenario: Duplicate pattern is merged
    Given .sensei/patterns.md already contains an "adapter-per-package" entry
    When the agent calls add_pattern("adapter-per-package", "...")
    Then the existing entry is updated, not duplicated

  Scenario: Pattern survives re-index
    Given a manually captured pattern in .sensei/patterns.md
    When reindex_repo() runs
    Then the manually captured pattern is preserved
    And auto-detected patterns are added without replacing manual ones
```

### Pattern-to-Skill Export

Patterns can be exported as local repo skills so they guide future agent work.

```gherkin
Feature: Pattern-to-Skill Export

  Scenario: Developer promotes a pattern to a skill
    Given a pattern "adapter-per-package" in .sensei/patterns.md
    When the developer runs sensei pattern export adapter-per-package
    Then a skill file is created at skills/adapter-per-package/SKILL.md
    And the skill describes when to apply the pattern, how to structure it, and an example

  Scenario: Exported skill is used by agent in next session
    Given a skills/adapter-per-package/SKILL.md in the repo
    When an agent is implementing a new package adapter
    Then the skill is available and guides the implementation
    And the agent doesn't re-derive the pattern from scratch

  Scenario: Skill stays in sync with pattern evolution
    Given a pattern that has been updated
    When the developer runs sensei pattern export adapter-per-package --update
    Then the existing SKILL.md is updated to reflect the current pattern definition
```

### Pattern Search

Agents find the right pattern for a task using semantic queries.

```gherkin
Feature: Pattern Search

  Scenario: Agent finds pattern by description
    Given patterns in .sensei/patterns.md
    When the agent calls find_pattern("components that need consistent DOM structure")
    Then the most relevant pattern is returned with a confidence score
    And the agent receives: name, description, when to use, example paths

  Scenario: No matching pattern returns graceful response
    Given a query that doesn't match any known pattern
    When the agent calls find_pattern("non-existent pattern")
    Then the response says no pattern found
    And suggests running reindex_repo() if the codebase has grown since last index

  Scenario: Agent lists all patterns
    Given .sensei/patterns.md with 8 patterns
    When the agent calls list_patterns()
    Then all 8 pattern names and one-line descriptions are returned
    And token usage is under 200 tokens
```
