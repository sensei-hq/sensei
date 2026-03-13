---
id: session-continuity
type: feature
---

# Session Continuity

> Sensei remembers where you were and picks up from there

When a coding session is interrupted — IDE crash, token limit, accidental close — the agent wakes up in the next session with no memory of what it was doing. Sensei tracks progress, snapshots state at natural step boundaries, and detects interruptions so that `get_session_context()` can surface exactly where the agent was and what it needs to do next.

## Features

### Session Orientation

`get_session_context()` is the first tool call in any new session. It returns a project summary assembled from `sensei.project_profile`, surfaces any interrupted session context that was in progress, and instructs the agent to verify completion or continue from the last known state.

```gherkin
Feature: Session Orientation

  Scenario: Agent orients in a new session with no prior work
    Given a freshly indexed repo with no prior sessions
    When the agent calls get_session_context()
    Then the response includes project name, stack, entry points, and active shortcuts from project_profile
    And the response includes any user guidelines from project_config
    And no interrupted session context is surfaced

  Scenario: Agent detects an interrupted session and resumes
    Given a prior session that was interrupted mid-task with a snapshot saved
    When the agent calls get_session_context() in a new session
    Then the interrupted session's progress_md is embedded in the orientation response
    And the response instructs the agent to verify completion or continue
    And the next_step_hint from the snapshot is prominently surfaced

  Scenario: Orientation response is concise
    Given a repo with a full project_profile and project_config
    When the agent calls get_session_context()
    Then the response is under 600 tokens for a typical project
    And long detail sections are referenced by path, not inlined

  Scenario: Multiple interrupted sessions are prioritized by recency
    Given two prior sessions that were both interrupted
    When the agent calls get_session_context()
    Then the most recently interrupted session is surfaced first
    And the agent is informed that a second interrupted session also exists
```

### Interrupt Detection and Recovery

Sensei listens for three interruption types: IDE or terminal close (process `exit` hook), IDE restart (same hook), and Claude token or session limit (connection drop). On clean exit, a final snapshot is written and the session is marked complete. On hard kill, the 30-second heartbeat acts as a safety net: after 10 minutes of idle, the session is marked crashed and the last snapshot is preserved for recovery.

```gherkin
Feature: Interrupt Detection and Recovery

  Scenario: Exit hook writes final snapshot on clean close
    Given an active session with work in progress
    When the IDE or terminal is closed cleanly
    Then the process exit hook fires
    And a final snapshot is written to sensei.session_snapshots
    And the session is marked as crashed with interrupted_at recorded

  Scenario: Heartbeat detects hard kill after idle timeout
    Given an active session where the process was hard-killed
    And no heartbeat ping has been received for 10 minutes
    When the heartbeat monitor checks session liveness
    Then the session is marked crashed
    And the last written snapshot is preserved for recovery in the next session

  Scenario: Recovery context is embedded in next orientation
    Given a crashed session with a snapshot recording progress_md and files_in_flight
    When the agent calls get_session_context() in the next session
    Then the crashed session's progress_md is included in the orientation
    And files_in_flight are listed so the agent knows what was being modified
    And the agent is prompted to check whether those files are in a consistent state

  Scenario: Completed sessions do not surface as interrupted
    Given a session that was explicitly ended with checkpoint()
    When the agent calls get_session_context() in the next session
    Then the completed session is not surfaced as an interrupted session
    And only incomplete or crashed sessions are highlighted
```

### Worktree-Aware Snapshots

Session snapshots record active git worktree references — branch name, path, and status — not just a list of files. Since uncommitted changes in a worktree are not visible from the main workspace, recovery explicitly surfaces worktree branches so the agent can check merge status before proceeding.

```gherkin
Feature: Worktree-Aware Snapshots

  Scenario: Snapshot records active worktree references
    Given a session working across two git worktrees
    When the agent calls snapshot()
    Then the snapshot written to sensei.session_snapshots includes worktree_refs
    And each entry records branch name, worktree path, and git status

  Scenario: Recovery surfaces worktree branches for inspection
    Given a crashed session with a snapshot that recorded two active worktrees
    When the agent calls get_session_context() in the next session
    Then the orientation includes the worktree branch names from the snapshot
    And the agent is instructed to check whether uncommitted changes in those worktrees were merged or abandoned

  Scenario: Snapshot captures a git diff stat summary
    Given a session with uncommitted changes across 8 files
    When the agent calls snapshot()
    Then the snapshot stores a git diff --stat summary
    And the diff stat is capped at 2KB to avoid bloating the snapshot

  Scenario: Snapshot stores structured progress fields
    Given an agent that has completed 3 of 5 planned steps
    When the agent calls snapshot() with completed_steps and next_step_hint
    Then the snapshot stores completed_steps, next_step_hint, progress_md, files_in_flight, and worktree_refs
    And all fields are retrievable on recovery
```

### Snapshot Cadence

The agent is expected to call `snapshot()` at natural step boundaries, especially before spawning parallel subagents. As a backstop for threads that get stuck or forget, an automatic snapshot is taken every 10 turns and stored in `sensei.session_snapshots`.

```gherkin
Feature: Snapshot Cadence

  Scenario: Agent snapshots before spawning parallel subagents
    Given an agent about to launch two parallel subagent threads
    When the agent calls snapshot() before dispatching them
    Then a snapshot is written recording current progress and files_in_flight
    And subagents can read the snapshot to understand what the parent has already done

  Scenario: Automatic snapshot fires every 10 turns
    Given an active session where the agent has not called snapshot() manually
    When the session reaches turn 10
    Then an automatic snapshot is written to sensei.session_snapshots
    And it records the current progress_md and files_in_flight at that point in the session

  Scenario: Manual snapshot at step boundary supersedes automatic cadence
    Given a session where the agent calls snapshot() at turn 7
    When turn 10 is reached
    Then the automatic snapshot at turn 10 still fires as a backstop
    And both snapshots are stored, with the turn-10 one marked as automatic

  Scenario: Snapshots are retained for the duration of the session
    Given a session with 3 manual snapshots and 2 automatic snapshots
    When the session ends
    Then all 5 snapshots remain in sensei.session_snapshots
    And the most recent snapshot is used for recovery if the session was interrupted
```

### Project Memory

Decisions, patterns, and open questions persist across sessions in Supabase via `sensei.repo_memory`. The agent can add items using `add_decision()`, `add_pattern()`, `ask_question()`, and `close_item()`. Relevant memory is surfaced in `get_session_context()` so the agent does not have to rediscover project conventions or revisit closed decisions.

```gherkin
Feature: Project Memory

  Scenario: Agent records a design decision
    Given a session where the agent decided to use optimistic locking for invoice updates
    When the agent calls add_decision("Use optimistic locking for invoice updates — avoids table locks under high concurrency")
    Then a row is written to sensei.repo_memory with type=decision and the description
    And a timestamp and session_id are recorded

  Scenario: Recorded decisions surface in future sessions
    Given a repo_memory entry for the optimistic locking decision
    When the agent calls get_session_context() in a later session about the invoicing module
    Then the relevant decision is included in the orientation context
    And the agent does not need to rediscover or re-evaluate that choice

  Scenario: Agent records an open question and later closes it
    Given an open question recorded as "Should we paginate the invoice list API?"
    When the agent calls close_item(id, resolution: "Yes — paginate with cursor, default page size 50")
    Then the item is marked closed in sensei.repo_memory
    And the resolution is stored alongside the original question
    And it no longer surfaces as an open item in get_session_context()

  Scenario: Patterns are surfaced as coding guidelines
    Given a pattern recorded as "Always wrap Supabase calls in a try/catch and return a Result type"
    When the agent calls get_session_context() for any task in the repo
    Then the pattern is included in the orientation response
    And the agent applies it without needing to read existing code for examples
```

### Task Transition

When switching tasks, the agent calls `checkpoint()` to close out the current context, then `recommend_next()` to get a prescription for the next task's context slice. This ensures the agent never carries stale context from a previous task into new work.

```gherkin
Feature: Task Transition

  Scenario: Agent checkpoints before switching tasks
    Given an agent that has completed work on the payments module
    When the agent calls checkpoint("Finished retry logic for payment processor")
    Then a final snapshot is written marking the task complete
    And the session is updated with the completed task recorded

  Scenario: recommend_next guides context loading for the next task
    Given a repo with several open tasks tracked in project memory
    When the agent calls recommend_next("add email notifications for failed payments")
    Then the response prescribes a specific context scope and resolution level
    And references only files not already in context that are relevant to the new task

  Scenario: Stale context is not carried into the new task
    Given an agent that loaded 15 files for the payments task
    When the agent calls checkpoint() and then context_pack() for a new auth task
    Then the new ContextPack does not include payments-module files
    And session deduplication marks the payments files as prior-session reads

  Scenario: Task transition is recorded in session history
    Given a session with two completed task transitions
    When the agent calls get_session_context() in a follow-up session
    Then the completed tasks from the prior session are listed in the orientation
    And the agent can see what was finished before deciding what to do next
```
