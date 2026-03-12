---
id: workflow
type: feature
---

# Workflow

An agent without a workflow protocol accumulates context, repeats work, and re-discovers the same decisions every session. The workflow module defines two complementary layers: a session protocol for how agents orient, load context, work, and transition within a session; and a project memory layer for what persists across sessions — decisions, patterns, checkpoints, and open items.

## Features

### Session Orientation Protocol

Every session starts the same way: orientation via the LLMSpec, not via file reads.

```gherkin
Feature: Session Orientation Protocol

  Scenario: Agent orients at session start
    Given an indexed repo
    When an agent begins a new session
    Then the agent calls get_llmspec() before reading any files
    And receives project name, description, stack, entry points, and shortcuts
    And spends under 600 tokens on orientation

  Scenario: Agent does not read files to orient
    Given an indexed repo
    When an agent is asked to start working
    Then the agent does not call Read or Glob before loading the LLMSpec
    And uses get_llmspec() as the first action

  Scenario: Agent uses recommend_next before loading context
    Given an agent that has oriented via LLMSpec
    And has received a task
    When the agent prepares to work
    Then the agent calls recommend_next(task) before loading any files
    And loads only the prescribed context slice
```

### Targeted Context Loading

Agents load exactly the context slice prescribed for their task — no more.

```gherkin
Feature: Targeted Context Loading

  Scenario: Agent loads orientation slice for a new task
    Given an agent starting a new task on an indexed repo
    When the agent calls load_context("orientation")
    Then a compact project summary is returned
    And total tokens are under 250

  Scenario: Agent loads module slice for a focused task
    Given an agent working on the auth module
    When the agent calls load_context("src/auth")
    Then only exports from the auth module are returned at L0
    And exports from other modules are not included

  Scenario: Agent loads patterns before editing
    Given an agent about to add a new function
    When the agent calls load_context("patterns")
    Then the repo's coding conventions are returned
    And the agent can follow existing patterns without guessing

  Scenario: Agent does not load full file trees
    Given an indexed repo
    When an agent needs to understand module structure
    Then the agent calls list_exports(module) rather than reading directories
    And total tokens for the operation are under 150
```

### MCP Offload Protocol

Deterministic, repeatable tasks go to MCP tools — not the LLM's reasoning loop.

```gherkin
Feature: MCP Offload Protocol

  Scenario: Agent generates llms.txt via MCP
    Given an indexed repo
    When an agent needs to produce llms.txt
    Then the agent calls generate_llms_txt() rather than writing it from scratch
    And the output is consistent with the current index
    And no tokens are spent generating the content in-context

  Scenario: Agent checks drift via MCP
    Given an indexed repo where docs may have changed
    When an agent needs to know if docs are in sync
    Then the agent calls check_drift() rather than comparing files manually
    And receives a structured drift report in one tool call

  Scenario: Agent lists exports via MCP
    Given an indexed repo
    When an agent needs to know what functions a module exports
    Then the agent calls list_exports(module) rather than reading the source file
    And receives L0 signatures for all exports

  Scenario: Agent finds patterns via MCP
    Given an indexed repo with detected patterns
    When an agent needs to know the DB access pattern
    Then the agent calls find_pattern("repository") rather than searching source files
    And receives the pattern description in one tool call
```

### Task Transition Protocol

Agents checkpoint before switching tasks to avoid carrying stale context.

```gherkin
Feature: Task Transition Protocol

  Scenario: Agent checkpoints before switching tasks
    Given an agent that has completed a task
    And is about to start a different task
    When the agent transitions
    Then the agent calls checkpoint() before starting the new task
    And then calls recommend_next(new_task) for fresh context prescription

  Scenario: Checkpoint saves context state
    Given an agent that has loaded context for task A
    When the agent calls checkpoint("task-a-complete")
    Then the current context state is saved
    And can be referenced later if needed

  Scenario: Agent does not carry previous context into new task
    Given an agent that has finished working on the auth module
    And is now asked to work on the billing module
    When the agent starts the billing task
    Then the agent calls checkpoint() to mark the transition
    And calls load_context("src/billing") for the new scope
    And does not keep auth module content in context
```

### Plan-to-Implementation Efficiency

Plans that include code steps cause double-spend: Claude reasons through code in the plan, then re-generates the same code during implementation. The workflow prevents this by separating plan artifacts from implementation artifacts and reusing plan output during execution.

```gherkin
Feature: Plan-to-Implementation Efficiency

  Scenario: Agent reuses plan output during implementation
    Given a plan step that specifies "add validateEmail(email: string): boolean"
    When the agent implements that step
    Then the agent uses the signature from the plan as the starting point
    And does not re-derive the function signature from scratch
    And does not re-read files already processed during planning

  Scenario: Plan stores intent, not code
    Given an agent creating an implementation plan
    When the agent writes the plan
    Then plan steps describe WHAT to do and WHERE
    And do not include code blocks that will be re-written during implementation
    And implementation is done once — during the execution phase

  Scenario: Agent signals re-derivation risk
    Given a plan step that includes a full code block
    When the agent is about to execute that step
    Then the agent uses the code from the plan directly, without re-generating
    And tokens are spent on the delta (edge cases, adjustments), not the core logic

  Scenario: Workflow skill separates plan phase from execution phase
    Given a task that requires planning then coding
    When the agent invokes the workflow skill
    Then phase 1 produces a plan with steps, files, and signatures — no code
    And phase 2 implements each step, treating plan outputs as input
    And no content from phase 1 is re-generated in phase 2
```

### Analysis-Before-Implementation Gate

Agents start implementing before fully understanding the task. The gate skill enforces a complete analysis phase — requirements, impact, constraints — before any code is written.

```gherkin
Feature: Analysis-Before-Implementation Gate

  Scenario: Agent completes analysis before writing code
    Given a task "add rate limiting to the API"
    When the agent invokes the workflow skill
    Then the agent first calls recommend_next(task) and loads the prescribed context
    And identifies: affected files, dependencies, constraints, and edge cases
    And produces an analysis summary before any implementation begins
    And only proceeds to implementation after the analysis is confirmed

  Scenario: Analysis surfaces scope before commitment
    Given a change that appears small but has broad impact
    When the agent runs the analysis phase
    Then the symbol graph is used to identify callers and dependents
    And the agent reports the impact radius to the user before coding
    And the user can adjust scope before tokens are spent on implementation

  Scenario: Phased implementation for large tasks
    Given a large task spanning multiple modules
    When the agent invokes the workflow skill
    Then the task is broken into phases with clear boundaries
    And each phase is analysed and confirmed before the next begins
    And progress is checkpointed between phases
    And the user can pause between phases without losing state

  Scenario: Gate blocks premature implementation
    Given an agent that has not completed analysis
    When the agent attempts to write implementation code
    Then the workflow skill prompts: "Analysis not confirmed. Complete get_impact_summary() first."
    And waits for explicit user confirmation before proceeding
```

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
