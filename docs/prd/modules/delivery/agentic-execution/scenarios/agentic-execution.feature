Feature: Agentic Execution — the designed autonomous loop (all unbuilt)

  # gap: agentic-unbuilt
  NO TEST
  Scenario: an agent run is created from a goal
    Given a goal and a project
    When an agent run is created
    Then it stores the goal and the initial plan graph
    This is unbuilt — no agent_runs table, no agents crate.

  # gap: agentic-unbuilt
  NO TEST
  Scenario: the agent loop drives, verifies and revises
    Given an agent run with a plan
    When the loop runs
    Then it drives a step, verifies, and revises the plan if needed
    This is unbuilt — no agents/loop.rs, no verify integration.

  # gap: agentic-unbuilt
  NO TEST
  Scenario: the verifier runs the project's test/lint/build commands
    Given a step that produced code changes
    When the verifier runs
    Then it executes the actual test/lint/build and returns pass/fail/partial
    This reuses checker.rs but the integration is unbuilt.

  # gap: agentic-unbuilt
  NO TEST
  Scenario: the loop revises the plan on verification failure
    Given a verification failure
    When the loop revises
    Then the plan graph is updated with a revision record
    This is unbuilt — no revision logic.

  # gap: agentic-unbuilt
  NO TEST
  Scenario: the loop respects the step cap and wall-time budget
    Given an agent run with a step cap and time budget
    When the loop runs
    Then it terminates at the cap or budget
    This reuses run_limits.rs logic but the integration is unbuilt.

  # gap: agentic-unbuilt
  NO TEST
  Scenario: the final agent run record includes the full transcript and verdict
    Given a completed agent run
    When its record is read
    Then it includes the goal, initial plan, step transcript, final verdict
    This is unbuilt — no agent_runs table.

  # gap: agentic-unbuilt
  NO TEST
  Scenario: the benchmark runner compares agent FTR vs human FTR
    Given a fixed task set
    When a benchmark runs
    Then it compares agent first-try rate to human baseline
    This is unbuilt — no benchmark runner code, no test.

  # gap: C-6
  NO TEST
  Scenario: the agent editor creates and edits agent configurations
    Given an agent configuration
    When it is edited
    Then it is stored and used by the agent loop
    This is unbuilt — C-6 agent/persona editors.

  # gap: C-6
  NO TEST
  Scenario: the persona editor creates and edits persona configurations
    Given a persona configuration
    When it is edited
    Then it is stored and used by the agent loop
    This is unbuilt — C-6 agent/persona editors.

  # gap: agentic-unbuilt
  NO TEST
  Scenario: the agent run detail screen shows the live transcript
    Given a running agent run
    When the detail screen opens
    Then it shows steps, verifier output, and plan revisions live
    This is unbuilt — greenfield screen.

  NO TEST
  Scenario: the checker captures pass/fail/spawn-error from a command
    Given a command that passes, fails, or fails to spawn
    When the checker runs it
    Then it captures the correct verdict
    This exists in checker.rs — pinned for reuse.