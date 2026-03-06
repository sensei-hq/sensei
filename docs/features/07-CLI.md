# CLI

Developers switch between projects, between personal and company contexts, and between different tech stacks. Without tooling to manage this, context is lost, standards drift apart, and each new project requires manual setup. The `skills` CLI makes context switching fast, profile management structured, and repo setup a single command.

## Features

### Repo Setup

The CLI sets up a new repo or adds to an existing one without overwriting anything already configured.

```gherkin
Feature: New Repo Setup

  Scenario: Developer sets up a brand new repo
    Given an empty directory or a repo with no .skills/ directory
    When the developer runs skills init
    Then the CLI indexes the repo
    And generates .llmspec.yaml, CLAUDE.md, llms.txt, and .index/
    And creates .skills/project.yaml with the selected profile references
    And asks which profiles to activate (personal + optional company)
    And asks whether to install the pre-commit drift hook
    And confirms what was created

  Scenario: Developer adds skills to an existing repo
    Given a repo that already has source files and docs
    When the developer runs skills add
    Then existing files are not overwritten
    And missing artifacts are created (.llmspec.yaml if absent, CLAUDE.md if absent)
    And the developer is told what was added and what was skipped

  Scenario: Developer upgrades skills in a repo
    Given a repo set up with an older version of skills
    When the developer runs skills upgrade
    Then the MCP server is rebuilt to the latest version
    And .index/ is refreshed
    And CLAUDE.md and llms.txt are regenerated
    And .llmspec.yaml is not overwritten (preserves manual edits)
    And the developer is told what changed
```

### Profile Management

Profiles store the developer's standards, workflow preferences, and skill configuration. They are editable and apply across all projects.

```gherkin
Feature: Personal Profile Management

  Scenario: Developer creates a personal profile
    Given no personal profile exists
    When the developer runs skills profile create
    Then the CLI prompts for name and workflow preferences interactively
    And creates ~/.skills/profiles/personal/profile.yaml
    And creates ~/.skills/profiles/personal/guidelines.md with the workflow template
    And creates ~/.skills/profiles/personal/skills.yaml with default skill set

  Scenario: Developer edits their personal guidelines
    Given a personal profile exists
    When the developer runs skills guidelines edit
    Then the guidelines.md file opens in the system editor ($EDITOR or default)
    And changes are saved when the editor closes

  Scenario: Developer views their active guidelines
    Given a personal profile with guidelines
    When the developer runs skills guidelines
    Then the merged guidelines are printed: personal first, then company if applicable
    And each section is labelled by source (personal / company name)

  Scenario: Developer switches active profile
    Given multiple profiles (personal, work)
    When the developer runs skills profile use work
    Then the active profile is updated in ~/.skills/config.yaml
    And subsequent skills commands use the work profile
```

### Company Profile Management

Company profiles define shared standards all developers on a project should follow, with optional remote MCP integration.

```gherkin
Feature: Company Profile Management

  Scenario: Developer creates a company profile
    Given no company profile for "acme" exists
    When the developer runs skills company create acme
    Then the CLI creates ~/.skills/profiles/companies/acme/profile.yaml
    And creates ~/.skills/profiles/companies/acme/guidelines.md
    And prompts for optional remote MCP endpoint
    And prompts for optional metrics configuration

  Scenario: Developer registers a remote company MCP
    Given a company profile for "acme"
    When the developer runs skills company register-mcp https://mcp.acme.internal
    Then the remote endpoint is saved to the acme profile
    And a local companion MCP is configured to cache remote responses
    And the registration is added to ~/.claude/mcp.json

  Scenario: Company guidelines are separate from personal
    Given a personal profile and a company profile for "acme"
    When the developer runs skills guidelines show --company acme
    Then only the acme company guidelines are shown
    And personal guidelines are not included

  Scenario: Project links to a company profile
    Given a company profile for "acme"
    And a repo with .skills/project.yaml
    When the developer edits project.yaml to reference company: acme
    Then skills commands in that repo merge personal + acme guidelines
    And the acme remote MCP is activated for that repo's sessions
```

### Context Switching

When a developer moves between projects, the CLI surfaces what's relevant for the current repo.

```gherkin
Feature: Context Switching

  Scenario: Developer checks status in a repo
    Given a repo with an existing index
    When the developer runs skills status
    Then the output shows: active profiles, MCP registration status,
         index age (days since last reindex), drift status (clean / N files drifted),
         and cached external libs

  Scenario: Developer sees project-specific context
    Given two repos — one personal project, one acme company project
    When the developer runs skills status in each
    Then the personal project shows: personal profile active, no company profile
    And the company project shows: personal + acme profiles active, remote MCP registered

  Scenario: Developer re-orients after switching projects
    Given a developer who was working on project A and switches to project B
    When the developer runs skills status in project B
    Then the current index age and drift status are shown
    And the developer knows whether to run skills index or skills drift before starting
```

### Shared Library Cache

External libraries used across multiple projects (personal or company stack) are indexed once and shared.

```gherkin
Feature: Shared Library Cache

  Scenario: Developer adds a local library to the cache
    Given a library at ~/Developer/rokkit
    When the developer runs skills cache add ~/Developer/rokkit --as rokkit
    Then the library is indexed into ~/.skills/cache/rokkit/
    And the MCP server can answer queries about rokkit without loading it into each repo

  Scenario: Developer queries a cached library
    Given rokkit is in the shared cache
    When an agent calls get_file_context("cache:rokkit/src/list.ts", "L0")
    Then the cached L0 signatures are returned
    And the full library path does not need to be in the current repo

  Scenario: Developer updates a cached library after changes
    Given rokkit is in the shared cache and has since been updated
    When the developer runs skills cache update rokkit
    Then the rokkit index is refreshed
    And all projects using the cache benefit immediately

  Scenario: Developer lists cached libraries
    Given two cached libraries: rokkit and kavach
    When the developer runs skills cache list
    Then both libraries are listed with their index age and file count
```

### Pre-Commit Hook

The CLI installs a pre-commit hook that runs drift detection before every commit.

```gherkin
Feature: Pre-Commit Hook

  Scenario: Developer installs the drift hook
    Given a repo with an existing index
    When the developer runs skills hooks install --drift
    Then a pre-commit hook is written to .git/hooks/pre-commit
    And the hook runs skills drift --fail-on-drift on each commit attempt

  Scenario: Hook blocks commit when drift is detected
    Given a repo with the drift hook installed
    And a source file was modified without updating design docs
    When the developer runs git commit
    Then the hook detects drift
    And prints the list of drifted files
    And blocks the commit with exit code 1

  Scenario: Hook passes when docs are in sync
    Given a repo with the drift hook installed
    And all docs are current with the last index
    When the developer runs git commit
    Then the hook reports no drift
    And the commit proceeds normally
```

### Guidelines Editability

Guidelines are plain markdown files, editable by the developer or company admin.

```gherkin
Feature: Guidelines Editability

  Scenario: Developer opens guidelines in editor
    Given a personal profile with guidelines.md
    When the developer runs skills guidelines edit
    Then $EDITOR opens with the guidelines file
    And the developer can freely edit the content
    And changes take effect immediately for future sessions

  Scenario: Developer queries a specific section of active guidelines
    Given active guidelines covering workflow, coding standards, and patterns
    When an agent calls get_guidelines("workflow")
    Then only the workflow section is returned
    And token cost is proportional to section size, not full guidelines

  Scenario: Company admin updates shared guidelines
    Given a company guidelines file in source control
    When the admin updates guidelines.md and pushes
    Then developers who run skills upgrade receive the updated guidelines
    And the MCP serves the new version immediately
```

## Status

| Feature | Status |
|---------|--------|
| New repo setup (skills init) | 🔲 Planned |
| Add to existing repo (skills add) | 🔲 Planned |
| Upgrade (skills upgrade) | 🔲 Planned |
| Personal profile create/edit | 🔲 Planned |
| Company profile create/edit | 🔲 Planned |
| Remote company MCP registration | 🔲 Planned |
| Local companion MCP for remote caching | 🔲 Planned |
| Context status (skills status) | 🔲 Planned |
| Shared library cache (skills cache) | 🔲 Planned |
| Pre-commit drift hook | 🔲 Planned |
| Guidelines view/edit/query | 🔲 Planned |
| Guidelines MCP tool (get_guidelines) | 🔲 Planned |
