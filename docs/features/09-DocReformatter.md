# Doc Reformatter

Existing repos accumulate docs in ad-hoc formats. When migrating to sensei, there is no mechanism to rewrite those docs to match the canonical templates — the content is right but the structure is wrong. The doc reformatter gives developers a one-command way to hand the reformatting work to Claude, preserving all existing content while applying the correct structure.

## Features

### Reformat a Single Doc

Produces a structured prompt for Claude to rewrite one document to match its canonical template.

```gherkin
Feature: Reformat a Single Doc

  Scenario: Developer reformats a design doc
    Given a file docs/design/03-auth.md in an old format
    And a canonical template at docs/templates/design.md
    When the developer runs sensei reformat docs/design/03-auth.md
    Then Claude receives the template structure and existing content
    And Claude rewrites the file preserving all information
    And the output matches the canonical design doc format

  Scenario: Developer reformats a feature/requirements doc
    Given a file docs/requirements/01-auth.md in an old format
    And a canonical template at docs/templates/feature.md
    When the developer runs sensei reformat docs/requirements/01-auth.md
    Then the correct template is auto-detected from the path
    And Claude rewrites to the feature doc format

  Scenario: Unknown doc type is handled gracefully
    Given a file at docs/notes/scratch.md
    When the developer runs sensei reformat docs/notes/scratch.md
    Then the CLI prompts: which template to use?
    And the developer selects from available templates
```

### Reformat a Directory

Reformats all docs in a directory, one file at a time with review between each.

```gherkin
Feature: Reformat a Directory

  Scenario: Developer reformats all requirements docs
    Given a directory docs/requirements/ with 5 files in old format
    When the developer runs sensei reformat docs/requirements/
    Then each file is presented to Claude for reformatting in sequence
    And the developer can review and approve each reformat before the next
    And a summary is shown: N reformatted, N skipped

  Scenario: Developer skips a file during batch reformat
    Given a batch reformat in progress
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
    When sensei reformat runs
    Then docs/templates/design.md is used automatically
    And no prompt is shown for template selection

  Scenario: Feature doc detected from path
    Given a file at docs/features/03-auth.md or docs/requirements/01-auth.md
    When sensei reformat runs
    Then docs/templates/feature.md is used automatically

  Scenario: Plans doc skipped
    Given a file at docs/plans/2026-03-06-impl.md
    When sensei reformat runs
    Then the CLI warns: plans are not reformatted (they are implementation artifacts)
    And exits without changes
```

### Reformat Rules

Claude follows explicit rules when reformatting to prevent information loss.

```gherkin
Feature: Reformat Rules

  Scenario: All content is preserved
    Given an existing doc with 10 sections of content
    When Claude reformats it
    Then all information from the original doc appears in the output
    And no sections are silently dropped

  Scenario: Missing sections get placeholder text
    Given an existing doc missing a section required by the template
    When Claude reformats it
    Then the missing section is added with placeholder text: "TODO: [section description]"
    And the developer knows what to fill in

  Scenario: Extra sections are preserved under a catch-all heading
    Given an existing doc with sections not in the template
    When Claude reformats it
    Then extra content is placed under "## Additional Notes"
    And nothing is discarded
```

## Status

| Feature | Status |
|---------|--------|
| Single file reformat (sensei reformat <file>) | 🔲 Planned |
| Directory batch reformat (sensei reformat <dir>) | 🔲 Planned |
| Template auto-detection from path | 🔲 Planned |
| Reformat rules (preserve all, placeholder for missing) | 🔲 Planned |
| doc-reformatter skill | 🔲 Planned |
