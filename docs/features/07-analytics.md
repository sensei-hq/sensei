---
id: analytics
type: feature
---

# Analytics

> Sensei measures quality, attribution, and improvement — and makes it visible

Knowing that an agent finished a task is not the same as knowing it finished it well. Sensei captures every tool call, every turn, and every session outcome, then distils those signals into a First-Time-Right score that distinguishes a vague spec from a bad agent. Developers rate goals, run benchmarks, and compare models — not with intuition, but with reproducible numbers stored in Supabase and surfaced in a dashboard that keeps improving as the team keeps working. Anonymised aggregate data across all sensei users powers real-world benchmarks and a personalised coaching layer that tells each developer exactly what changes would improve their implementation cycle — starting with the highest-leverage variable: requirement quality.

## Features

### Telemetry Collection

PreToolUse and PostToolUse hook events are captured for every tool call and stored in `sensei.events`. When Supabase is unavailable the collector writes JSONL to disk and drains the file to Supabase on reconnect. Each event records: tool name, phase, duration_ms, success, session_id, project_path, model_id, model_provider, and token counts. The telemetry daemon runs on `localhost:51789`.

```gherkin
Feature: Telemetry Collection

  Scenario: Tool call events are recorded in Supabase
    Given the sensei daemon is running and Supabase is reachable
    When the agent calls the read_file MCP tool
    Then a PreToolUse event is inserted into sensei.events before the call executes
    And a PostToolUse event is inserted after the call completes
    And both events include: tool_name "read_file", session_id, duration_ms, and success

  Scenario: JSONL fallback activates when Supabase is unavailable
    Given Supabase is unreachable
    When the agent calls any MCP tool
    Then the event is written to the JSONL fallback file on disk
    And no error is surfaced to the agent

  Scenario: JSONL events are drained on reconnect
    Given the JSONL fallback file contains 47 events written during an outage
    When the sensei daemon starts and successfully connects to Supabase
    Then all 47 events are inserted into sensei.events
    And the JSONL file is cleared after successful drain
```

### Task and Turn Tracking

Each agent session is recorded in `task_sessions` with metadata including repo_id, agent name, model_id, task description, task type, and status. Individual turns are recorded in `task_turns` capturing per-turn model and token data. Task type is auto-detected from keywords or an optional local LLM classifier.

```gherkin
Feature: Task and Turn Tracking

  Scenario: Session is created when a task begins
    Given the developer starts a new Claude session in a sensei-instrumented repo
    When the UserPromptSubmit hook fires with "fix the broken auth middleware"
    Then a row is inserted into task_sessions with status "in_progress"
    And task_type is auto-detected as "fix" from the keyword "fix"

  Scenario: Per-turn token data is recorded including mid-session model switches
    Given a session is in progress and the developer switches from claude-3-5-sonnet to claude-3-opus mid-session
    Then a new task_turns row is created for each turn
    And the model_id and model_provider columns reflect the model used for that specific turn
    And user_tokens, agent_tokens, and pack_tokens are recorded per turn

  Scenario: Session is marked completed when the agent signals task done
    Given a task_session with status "in_progress"
    When the agent calls the MCP tool complete_task with a summary
    Then the task_sessions row is updated to status "completed"
    And ended_at is set to the current timestamp

  Scenario: Abandoned session is detected after inactivity
    Given a task_session with status "in_progress" and no turns for 4 hours
    When the sensei daemon runs its session cleanup job
    Then the session status is updated to "abandoned"
    And the event is logged to sensei.events
```

### FTR Scoring (First-Time-Right)

FTR is a composite score from 0.0 to 1.0 stored in `sensei.task_outcomes`. It aggregates six weighted signals: turns taken (30%), refinements requested (20%), no reverts within N commits (20%), tests pass on first run (15%), no task revisit within 7 days (10%), and no bugs filed against session output (5%). Developer vs agent attribution distinguishes a vague spec from a clear spec with wrong output.

```gherkin
Feature: FTR Scoring

  Scenario: FTR score is computed after session completion
    Given a completed task_session where the task took 3 turns, no refinements, tests passed, and no revert within 10 commits
    When sensei computes the FTR score for that session
    Then a task_outcomes row is created with a score between 0.7 and 1.0
    And each contributing signal is stored alongside the composite score

  Scenario: Task revisit within 7 days reduces the FTR score
    Given a task was completed 5 days ago and a new session opens for the same file and feature area
    When the new session is linked to the original task
    Then the original task_outcomes score is reduced by the revisit penalty (10%)
    And the outcome record is updated in sensei.task_outcomes

  Scenario: Attribution distinguishes vague spec from bad agent output
    Given a task where the developer rated the spec as vague
    And the agent produced multiple refinements
    When sensei computes attribution for that outcome
    Then the developer_attribution field is high and agent_attribution field is low
    And the FTR penalty is applied to the developer signal, not the agent signal

  Scenario: FTR heuristic fallback runs when no user rating is provided
    Given a completed session with no developer rating after 24 hours
    When sensei runs the heuristic FTR calculation
    Then a score is computed from automated signals only (turns, reverts, test results)
    And the task_outcomes row is created with rating_source set to "heuristic"
```

### Quality Rating UI

The web dashboard groups session turns by summarised goal and lets the developer rate each goal as pass, fail, or partial. Ratings are stored in `task_outcomes` and integrated with Claude's existing feedback mechanism. Historical FTR trends are visible per repo, per agent, and per model.

```gherkin
Feature: Quality Rating UI

  Scenario: Dashboard groups turns by summarised goal
    Given a session with 12 turns spanning 3 distinct goals
    When the developer opens the session detail view in the dashboard
    Then turns are grouped into 3 goal cards with auto-generated summaries
    And each card shows the turn count and total token spend for that goal

  Scenario: Developer rates a goal as partial
    Given a goal card for "add rate limiting middleware" in the dashboard
    When the developer clicks the "partial" rating button
    Then a task_outcomes row is created or updated with ftr_rating "partial" for that goal
    And the overall session FTR score is recalculated

  Scenario: FTR trend is visible per model in the dashboard
    Given 30 days of task outcomes across two models: claude-3-5-sonnet and claude-3-opus
    When the developer opens the Model Comparison view
    Then a trend chart shows average FTR score per week for each model
    And the chart allows filtering by repo and task_type
```

### Benchmarking

A task corpus in `tasks/sample.yaml` provides representative developer tasks for A/B evaluation. Runs compare with-skills vs without-skills configurations across metrics: tokens, interactions, tool calls, and task completion. Results are stored in `sensei.benchmark_reports` and compared via CLI.

```gherkin
Feature: Benchmarking

  Scenario: Benchmark run executes all corpus tasks and stores results
    Given tasks/sample.yaml contains 10 representative tasks
    When the developer runs sensei benchmark run --config with-skills
    Then each task is executed against the current agent configuration
    And results are stored in sensei.benchmark_reports with run_id, config, and per-task metrics

  Scenario: A/B comparison shows improvement from skills
    Given two benchmark runs: run-001 (without-skills) and run-002 (with-skills)
    When the developer runs sensei benchmark compare run-001 run-002
    Then a report is printed showing delta for: tokens, interactions, tool_calls, completion_rate
    And the improvement percentage is shown for each metric

  Scenario: FTR improvement from a skill change is measurable
    Given a baseline benchmark run before a skill file was updated
    And a new benchmark run after the skill update
    When the developer runs sensei benchmark compare <before-id> <after-id>
    Then the FTR delta is shown for tasks in the skill's coverage area
    And tasks outside the coverage area show no significant delta
```

### Requirement Quality Scoring

The single highest-leverage variable in FTR is the quality of the requirement given to the agent. Vague requirements produce refinement loops. Concrete requirements with explicit constraints, acceptance criteria, and examples produce first-time-right output. Sensei measures requirement quality automatically and correlates it with outcomes across all sessions — showing developers precisely where upfront investment in elaboration pays off.

Requirement quality is scored at task start (0.0–1.0) from: description length, presence of acceptance criteria, specificity keywords ("must", "should not", "returns", "given/when/then"), examples provided, and referenced context (files, symbols, docs). The score is stored in `task_sessions.req_quality_score` and correlated with FTR in `task_outcomes`.

```gherkin
Feature: Requirement Quality Scoring

  Scenario: Vague requirement receives a low quality score
    Given a developer submits the task: "fix the login bug"
    When sensei scores the requirement at session start
    Then req_quality_score is below 0.3
    And the session is flagged for elaboration guidance
    And the dashboard shows the suggested elaboration prompt inline

  Scenario: Concrete requirement receives a high quality score
    Given a developer submits the task: "The login endpoint returns 500 when the email contains a plus sign. It should return 200 and accept the user. Affected file: src/auth/login.ts. Acceptance: existing tests pass plus a new test for plus-sign emails."
    When sensei scores the requirement
    Then req_quality_score is above 0.8
    And no elaboration prompt is shown

  Scenario: Aggregate shows req quality correlates with FTR
    Given 90 days of sessions with both req_quality_score and ftr_score recorded
    When the developer opens the Insights view
    Then a scatter chart shows req_quality_score on the x-axis and ftr_score on the y-axis
    And the trend line shows positive correlation with R² displayed
    And the chart is filterable by task_type and agent

  Scenario: Developer sees time-to-complete vs req quality
    Given sessions where elaboration took >2 minutes before first tool call
    When compared to sessions with immediate first tool call
    Then sessions with elaboration time >2 minutes show 35% fewer total turns on average
    And this finding is surfaced as a personalised insight: "Your sessions with upfront elaboration complete in fewer turns"
```

---

### Developer Coaching Engine

Sensei compares each developer's personal metrics against anonymised aggregate benchmarks from all users with similar stacks, agent configurations, and task types. The result is personalised, actionable guidance — not generic best practices. Coaching covers prompting patterns, requirement elaboration, clarification timing, and context loading habits that correlate with better outcomes.

```gherkin
Feature: Developer Coaching Engine

  Scenario: Developer receives personalised FTR improvement recommendation
    Given a developer's FTR for bug-fix tasks is 0.54
    And the aggregate FTR for bug-fix tasks among TypeScript/Claude users is 0.79
    When sensei generates coaching recommendations
    Then the dashboard shows: "Your bug-fix FTR (0.54) is below the 0.79 average for similar teams"
    And the top recommendation is: "Teams using context_pack before fix sessions see the largest FTR improvement — try calling context_pack with the error description before making changes"

  Scenario: Prompting pattern guidance is surfaced
    Given a developer's sessions show frequent mid-task refinements
    And aggregate data shows that sessions starting with a Given/When/Then structured prompt have 40% fewer refinements
    When coaching recommendations are generated
    Then the developer receives: "Your sessions average 4.2 refinements. Structuring prompts as Given/When/Then reduces refinements to 2.5 on average for your task types"
    And an example rewrite of their most recent vague prompt is shown

  Scenario: Clarification timing guidance is personalised
    Given a developer rarely asks clarifying questions before implementation
    And aggregate data shows that one targeted clarifying question early in a session reduces total turns by 30%
    When sensei analyses the developer's session patterns
    Then the coaching insight is: "Adding one clarifying question before implementation reduces your average session from 9 turns to 6"
    And example clarifying questions for their common task types are suggested

  Scenario: Requirement elaboration ROI is shown
    Given a developer spends an average of 45 seconds on requirement description
    And peers with similar tasks spend 3 minutes and have 28% higher FTR
    When the coaching engine runs its weekly analysis
    Then the developer sees: "3 minutes of requirement elaboration saves an average of 4 turns per session for your team"
    And a link to the requirement quality guide is provided
```

---

### Aggregate Benchmarks & Public Insights

Opt-in anonymised telemetry from across all sensei users powers public real-world benchmarks. All data is de-identified before aggregation: repo names, file paths, user IDs, and organisation names are stripped. Only aggregatable signals are retained: token counts, FTR scores, tool usage patterns, session durations, task types, and stack identifiers. Open source repos contribute by default with opt-out; private repos opt-in explicitly.

Public benchmarks are published on the sensei website and updated continuously. They provide independently verifiable evidence of sensei's impact — not synthetic benchmarks, but real developer sessions.

```gherkin
Feature: Aggregate Benchmarks

  Scenario: Developer opts in to anonymous telemetry contribution
    Given a developer with a private repo opens sensei settings
    When they toggle "Contribute anonymous telemetry"
    Then future session metrics are included in aggregate calculations
    And repo names, file paths, and user IDs are stripped before transmission
    And the developer can view exactly what fields are contributed

  Scenario: Open source repo contributes by default with opt-out
    Given a repo is marked as open source in sensei
    When sessions are recorded
    Then anonymised metrics are included in aggregate calculations automatically
    And the developer can opt out from the repo settings page at any time

  Scenario: Public benchmark page shows real-world token reduction
    Given 10,000 sessions from opted-in users across 500 repos
    When the benchmark page at sensei.dev/benchmarks is loaded
    Then it shows: median token reduction (%), average FTR score, average sessions-to-completion
    And metrics are broken down by agent, stack, and task type
    And each metric shows sample size and confidence interval

  Scenario: Aggregate insight feeds back into coaching
    Given new aggregate data shows that repos using custom lib indexing have 22% higher FTR on UI tasks
    When the coaching engine runs for a developer with rokkit indexed but custom lib indexing disabled
    Then a coaching recommendation is generated: "Teams with custom lib indexing enabled see 22% better FTR on UI tasks — your rokkit library is a candidate"

  Scenario: Team benchmark compares against cohort
    Given a team of 4 developers using sensei for 60 days
    When they open the Team Insights view
    Then their aggregate FTR, token reduction, and average turns are shown
    And each metric is compared against the anonymised cohort of teams with the same stack and team size
    And percentile ranking is shown: "Your team is in the 72nd percentile for FTR among TypeScript teams"
```

---

### Analytics CLI and Dashboard

`sensei stats` provides a 7-day summary of tool usage, session activity, and token spend from the command line. The `--gaps` flag surfaces files and tools that agents bypassed in favour of raw bash. The dashboard adds visual FTR scores, model comparisons, benchmark results, and per-session turn timelines.

```gherkin
Feature: Analytics CLI and Dashboard

  Scenario: sensei stats shows 7-day summary
    Given the last 7 days contain 14 sessions across 3 repos
    When the developer runs sensei stats
    Then the output shows: total sessions, total turns, total tokens, top 5 tools by call count
    And the output fits in a terminal without scrolling for a typical week

  Scenario: sensei stats --gaps reports missed tool opportunities
    Given 5 sessions where the agent ran raw bash cat/grep commands instead of sensei tools
    When the developer runs sensei stats --gaps
    Then each bypassed tool usage is listed with: session_id, command used, sensei tool that could have been used
    And a summary count of total missed opportunities is shown

  Scenario: sensei stats --json outputs machine-readable data
    Given a week of session data in sensei.events
    When the developer runs sensei stats --json
    Then the output is valid JSON with the same fields as the default summary
    And it can be piped into jq or written to a file without parsing errors

  Scenario: Dashboard shows turn timeline for a session
    Given a session with 8 turns and attached telemetry events
    When the developer opens the session detail view
    Then a timeline is displayed showing each turn's start time, duration, and tool calls
    And the timeline highlights turns with high token spend or errors
```
