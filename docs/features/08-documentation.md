---
id: documentation
type: feature
---

# Documentation

Documentation quality degrades in two ways: existing docs drift into ad-hoc formats, and new docs get created inconsistently. The documentation module addresses both: a doc-doctor skill that reformats existing docs to canonical templates without losing content, and a set of skills and MCP tools that guide agents in creating, finding, and maintaining docs consistently.

## Features

### Doc Guide

A skill that explains the documentation system: conventions, structure, and the feature/design split.

```gherkin
Feature: Doc Guide

  Scenario: Agent creates a feature doc for a new capability
    Given a task to document a new caching feature
    When the agent invokes the doc-guide skill
    Then the agent creates docs/features/NN-PascalCase.md using the feature template
    And does NOT include implementation details (algorithms, schemas, APIs)
    And includes Gherkin scenarios covering the happy path and error cases

  Scenario: Agent creates a design doc for an implementation
    Given a task to document how the caching implementation works
    When the agent invokes the doc-guide skill
    Then the agent creates docs/design/NN-PascalCase.md using the design template
    And does NOT include user-facing needs or "why" reasoning
    And includes: data schema, algorithm/flow, API contracts, error handling, testing strategy

  Scenario: Agent correctly pairs feature and design docs
    Given a new module that needs both user-facing and implementation docs
    When the agent invokes the doc-guide skill
    Then the agent creates both docs/features/NN-Name.md and docs/design/NN-Name.md
    And each file's footer references the other

  Scenario: Agent resolves naming conflicts
    Given existing docs numbered 01 through 14
    When the agent needs to create a new doc pair
    Then the agent uses 15 as the prefix
    And names the file in PascalCase matching the module name
```

### Find Doc

Given a query, surface the right existing doc without the agent reading all of them.

```gherkin
Feature: Find Doc

  Scenario: Agent finds a design doc by topic
    Given 16 design docs in docs/design/
    When the agent calls find_doc("how the indexer handles incremental updates")
    Then the most relevant design doc path is returned
    And a one-line summary of the relevant section is included
    And only the matched doc path is returned, not full content

  Scenario: Agent finds a feature doc by capability
    Given feature docs in docs/features/
    When the agent calls find_doc("how session context is loaded at startup")
    Then docs/features/workflow.md is returned
    And the agent can load it with get_file_context() if needed

  Scenario: Agent finds the right doc before creating a new one
    Given an agent about to create a new doc
    When the agent calls find_doc(topic) first
    Then if a doc exists, the agent updates it instead of creating a duplicate
    And if no doc exists, the agent proceeds to create one using the doc-guide skill

  Scenario: No matching doc returns graceful response
    Given a query that doesn't match any existing doc
    When the agent calls find_doc("non-existent topic")
    Then the response says no matching doc found
    And suggests using doc-guide skill to create one
```

### New Feature Scaffold

Creates a matched feature+design doc pair from a single command.

```gherkin
Feature: New Feature Scaffold

  Scenario: Developer scaffolds a new feature pair
    Given no existing docs for "Response Caching"
    When the developer runs sensei doc new "Response Caching"
    Then docs/features/NN-ResponseCaching.md is created from the feature template
    And docs/design/NN-ResponseCaching.md is created from the design template
    And both files contain the correct NN prefix (next available number)
    And each file's footer links to the other

  Scenario: Scaffold fails if name already exists
    Given docs/features/08-ProjectWorkflow.md already exists
    When the developer runs sensei doc new "Project Workflow"
    Then the command errors with: "08-ProjectWorkflow.md already exists. Use sensei doctor to update it."
    And no files are created

  Scenario: Scaffold updates the README module list
    Given docs/features/README.md with a module list and status table
    When sensei doc new "Response Caching" completes
    Then the new module is added to the Modules list in README.md
    And a placeholder status row is added to the Feature Status table
```

### External Doc Reference

Fetch and cache external API documentation for use within the session and across sessions.

```gherkin
Feature: External Doc Reference

  Scenario: Agent fetches external API docs on first use
    Given a task requiring the Anthropic API reference
    When the agent calls fetch_doc_ref("anthropic messages api")
    Then the relevant API doc is fetched and returned
    And the result is cached in .index/doc-refs/ with a TTL of 7 days

  Scenario: Agent retrieves cached doc ref without refetching
    Given a cached Anthropic API doc from 3 days ago
    When the agent calls fetch_doc_ref("anthropic messages api")
    Then the cached version is returned immediately
    And no network request is made

  Scenario: Stale doc ref is refreshed
    Given a cached doc ref older than 7 days
    When the agent calls fetch_doc_ref("anthropic messages api")
    Then a fresh fetch is performed
    And the cache is updated
    And the agent receives the refreshed content

  Scenario: Agent searches across cached doc refs
    Given multiple cached doc refs (Anthropic, Stripe, GitHub)
    When the agent calls search_doc_refs("rate limiting headers")
    Then matching sections across all cached refs are returned
    And the source URL is included for each result
```

### Doctor a Single Doc

Produces a structured prompt for Claude to rewrite one document to match its canonical template, preserving all existing content.

```gherkin
Feature: Doctor a Single Doc

  Scenario: Developer doctors a design doc
    Given a file docs/design/03-auth.md in an old format
    And a canonical template at docs/templates/design.md
    When the developer runs sensei doctor docs/design/03-auth.md
    Then Claude receives the template structure and existing content
    And Claude rewrites the file preserving all information
    And the output matches the canonical design doc format

  Scenario: Developer doctors a feature doc
    Given a file docs/requirements/01-auth.md in an old format
    And a canonical template at docs/templates/feature.md
    When the developer runs sensei doctor docs/requirements/01-auth.md
    Then the correct template is auto-detected from the path
    And Claude rewrites to the feature doc format

  Scenario: Unknown doc type is handled gracefully
    Given a file at docs/notes/scratch.md
    When the developer runs sensei doctor docs/notes/scratch.md
    Then the CLI prompts: which template to use?
    And the developer selects from available templates
```

### Doctor a Directory

Doctors all docs in a directory, one file at a time with review between each.

```gherkin
Feature: Doctor a Directory

  Scenario: Developer doctors all requirements docs
    Given a directory docs/requirements/ with 5 files in old format
    When the developer runs sensei doctor docs/requirements/
    Then each file is presented to Claude for doctoring in sequence
    And the developer can review and approve each before the next
    And a summary is shown: N doctored, N skipped

  Scenario: Developer skips a file during batch doctor
    Given a batch doctor in progress
    When the developer chooses to skip the current file
    Then that file is left unchanged
    And the batch continues with the next file
```

### Template Auto-Detection

The correct template is inferred from the file path without developer input.

```gherkin
Feature: Template Auto-Detection

  Scenario: Design doc detected from path
    Given a file at docs/design/05-payments.md
    When sensei doctor runs
    Then docs/templates/design.md is used automatically
    And no prompt is shown for template selection

  Scenario: Feature doc detected from path
    Given a file at docs/features/03-auth.md or docs/requirements/01-auth.md
    When sensei doctor runs
    Then docs/templates/feature.md is used automatically

  Scenario: Plans doc skipped
    Given a file at docs/plans/2026-03-06-impl.md
    When sensei doctor runs
    Then the CLI warns: plans are not doctored (they are implementation artifacts)
    And exits without changes
```

### Doctor Rules

Claude follows explicit rules when doctoring to prevent information loss.

```gherkin
Feature: Doctor Rules

  Scenario: All content is preserved
    Given an existing doc with 10 sections of content
    When Claude doctors it
    Then all information from the original doc appears in the output
    And no sections are silently dropped

  Scenario: Missing sections get placeholder text
    Given an existing doc missing a section required by the template
    When Claude doctors it
    Then the missing section is added with placeholder text: "TODO: [section description]"
    And the developer knows what to fill in

  Scenario: Extra sections are preserved under a catch-all heading
    Given an existing doc with sections not in the template
    When Claude doctors it
    Then extra content is placed under "## Additional Notes"
    And nothing is discarded
```
