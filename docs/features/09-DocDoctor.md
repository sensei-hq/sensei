# Doc Doctorter

Existing repos accumulate docs in ad-hoc formats. When migrating to sensei, there is no mechanism to rewrite those docs to match the canonical templates — the content is right but the structure is wrong. The doc doctorter gives developers a one-command way to hand the doctorting work to Claude, preserving all existing content while applying the correct structure.

## Features

### Doctor a Single Doc

Produces a structured prompt for Claude to rewrite one document to match its canonical template.

```gherkin
Feature: Doctor a Single Doc

  Scenario: Developer doctors a design doc
    Given a file docs/design/03-auth.md in an old format
    And a canonical template at docs/templates/design.md
    When the developer runs sensei doctor docs/design/03-auth.md
    Then Claude receives the template structure and existing content
    And Claude rewrites the file preserving all information
    And the output matches the canonical design doc format

  Scenario: Developer doctors a feature/requirements doc
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
    Then each file is presented to Claude for doctorting in sequence
    And the developer can review and approve each doctor before the next
    And a summary is shown: N doctorted, N skipped

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
    Then the CLI warns: plans are not doctorted (they are implementation artifacts)
    And exits without changes
```

### Doctor Rules

Claude follows explicit rules when doctorting to prevent information loss.

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

## Status

| Feature | Status |
|---------|--------|
| Single file doctor (sensei doctor <file>) | 🔲 Planned |
| Directory batch doctor (sensei doctor <dir>) | 🔲 Planned |
| Template auto-detection from path | 🔲 Planned |
| Doctor rules (preserve all, placeholder for missing) | 🔲 Planned |
| doc-doctor skill | 🔲 Planned |
