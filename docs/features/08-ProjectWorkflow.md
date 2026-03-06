# Project Workflow

Agents working across sessions on the same project lose context silently. Without a persistent knowledge layer, every session re-discovers the same patterns, re-reads the same files, and re-negotiates the same decisions. The project workflow skill gives agents a structured protocol that is process-centric — not token-reduction-centric — and delegates all file management to MCP tools so tracking doesn't add bloat.

## Features

### Session Resume

Agents orient from a compressed project snapshot instead of reading multiple files manually.

```gherkin
Feature: Session Resume

  Scenario: Agent resumes a project after a break
    Given a project with prior checkpoints
    When the agent calls get_session_context()
    Then the response contains: project memory, recent decisions, open items, and active plan steps
    And the total tokens are under 400
    And no full journal history is included

  Scenario: Agent starts fresh on a project with no checkpoints
    Given an indexed project with no prior checkpoints
    When the agent calls get_session_context()
    Then the response falls back to get_llmspec() content
    And the agent receives orientation in under 500 tokens

  Scenario: Context stays flat as project grows
    Given a project with 6 months of checkpoint history
    When the agent calls get_session_context()
    Then the response is still under 400 tokens
    And older sessions are archived, not loaded
```

### Decision Capture

Confirmed decisions are captured immediately without file read/write overhead.

```gherkin
Feature: Decision Capture

  Scenario: Agent records a confirmed decision
    Given a project session where a decision was just made
    When the agent calls add_decision("Use repository pattern for all DB access")
    Then the decision is appended to project memory
    And the agent receives confirmation without needing to re-read the file

  Scenario: Duplicate decisions are merged not appended
    Given project memory already contains "Use repository pattern for all DB access"
    When the agent calls add_decision("Use repository pattern for all DB access")
    Then the memory is not duplicated
    And the existing entry is updated with a newer timestamp

  Scenario: Decisions survive across sessions
    Given a decision captured in session 1
    When a new session calls get_session_context()
    Then the decision is present in the returned memory
```

### Pattern Capture

Proven patterns are recorded when used a second time — not speculatively.

```gherkin
Feature: Pattern Capture

  Scenario: Agent records a proven pattern
    Given a pattern has been used in 2+ places in the codebase
    When the agent calls add_pattern("data-attribute DOM", "Use data-{component} on root, data-{component}-{part} on children")
    Then the pattern is added to project patterns
    And future sessions can retrieve it via find_pattern()

  Scenario: Pattern is available cross-session
    Given a pattern captured in a prior session
    When a new session calls find_pattern("data-attribute")
    Then the pattern is returned
    And no file reads are required by the agent
```

### Session Checkpoint

At session end, the agent distills the session and hands off to the MCP tool — which compresses and stores it.

```gherkin
Feature: Session Checkpoint

  Scenario: Agent checkpoints at session end
    Given an agent that has completed a task
    When the agent calls checkpoint("Added POST /users endpoint. Tests pass. Next: add validation middleware.")
    Then the note is merged into the session snapshot
    And open items are updated
    And the session is archived
    And the agent receives: "Checkpointed. Resume with get_session_context()."

  Scenario: Checkpoint before task switch
    Given an agent switching between two tasks
    When the agent calls checkpoint("Auth module done. Switching to billing.")
    Then the prior task context is archived
    And the agent starts the next task with a clean slate

  Scenario: Checkpoint context stays bounded
    Given a project with 50 prior checkpoints
    When the agent calls checkpoint("...")
    Then only the last 2 session snapshots are retained in active memory
    And older sessions are in archived storage only
```

### Open Items Tracking

Unresolved questions and next steps are tracked without agent file management.

```gherkin
Feature: Open Items Tracking

  Scenario: Agent queues a question for the user
    Given a session where a decision needs user input
    When the agent calls ask_question("Should we use optimistic locking or row versioning?")
    Then the question is added to open items
    And the agent continues without blocking

  Scenario: Agent retrieves open items at session start
    Given open items from a previous session
    When get_session_context() is called
    Then the open items are included in the response
    And the agent knows what needs resolution before proceeding

  Scenario: Agent closes a resolved item
    Given an open question that has been answered in conversation
    When the agent calls close_item("Should we use optimistic locking or row versioning?")
    Then the item is removed from open items
    And the resolution can optionally be captured as a decision via add_decision()
```

### Migration from Manual agents/ Structure

For repos that use a manual `agents/` folder pattern, the CLI migrates to the MCP-managed checkpoint system.

```gherkin
Feature: Migration from agents/ folder

  Scenario: Developer migrates an existing repo
    Given a repo with agents/memory.md, agents/journal.md, agents/design-patterns.md
    When the developer runs sensei migrate
    Then agents/memory.md content is distilled into .index/checkpoints/memory.yaml
    And agents/design-patterns.md is migrated into .index/checkpoints/patterns.yaml
    And the last journal entry is extracted as an open item / next step
    And agents/ is archived to agents/_archived/ (not deleted)
    And a new CLAUDE.md is generated referencing the sensei workflow

  Scenario: Migration preserves nuanced decisions
    Given a memory.md with 30 confirmed architectural decisions
    When sensei migrate runs
    Then all decisions appear in .index/checkpoints/memory.yaml
    And none are silently lost

  Scenario: Developer reviews migration output before removing archived folder
    Given a completed migration
    When the developer inspects agents/_archived/ and .index/checkpoints/
    Then they can verify parity before deleting the archived folder
```

## Status

| Feature | Status |
|---------|--------|
| Session resume (get_session_context) | 🔲 Planned |
| Decision capture (add_decision) | 🔲 Planned |
| Pattern capture (add_pattern) | 🔲 Planned |
| Session checkpoint with distillation | 🔲 Planned |
| Open items (ask_question, close_item, get_open_items) | 🔲 Planned |
| Migration from agents/ folder (sensei migrate) | 🔲 Planned |
| Context budget stays flat over time | 🔲 Planned |
