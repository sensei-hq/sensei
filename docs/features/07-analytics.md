---
id: analytics
type: feature
---

# Analytics

> Sensei measures quality, attribution, and improvement — and makes it visible

Knowing that an agent finished a task is not the same as knowing it finished it well. Sensei captures every tool call, every turn, and every session outcome, then distils those signals into a First-Time-Right score that distinguishes a vague spec from a bad agent. Developers rate goals, run benchmarks, and compare models — not with intuition, but with reproducible numbers persisted and surfaced in a dashboard that keeps improving as the team keeps working. Anonymised aggregate data across all sensei users powers real-world benchmarks and a personalised coaching layer that tells each developer exactly what changes would improve their implementation cycle — starting with the highest-leverage variable: requirement quality.

## Features

### Telemetry Collection

Every tool call is captured as a telemetry event and stored durably. Events are queued locally when the backend is unavailable and flushed automatically on reconnect. Each event records the tool used, phase, duration, outcome, session, project, model, provider, and token counts.

```gherkin
Feature: Telemetry Collection

  Scenario: Tool call events are recorded
    Given the sensei daemon is running and the backend is reachable
    When the agent calls a tool
    Then an event is recorded before the call executes
    And an event is recorded after the call completes
    And both events include: tool name, session, duration, and success status

  Scenario: Local fallback activates when the backend is unavailable
    Given the backend is unreachable
    When the agent calls any MCP tool
    Then the event is queued locally
    And no error is surfaced to the agent

  Scenario: Queued events are drained on reconnect
    Given events were queued locally during an outage
    When the sensei daemon starts and successfully connects to the backend
    Then all queued events are stored persistently
    And the local queue is cleared after successful drain
```

### Task and Turn Tracking

Each agent session is recorded with metadata including the repo, agent name, model, task description, task type, and status. Individual turns capture per-turn model and token data. Task type is auto-detected from the task description.

```gherkin
Feature: Task and Turn Tracking

  Scenario: Session is created when a task begins
    Given the developer starts a new agent session in a sensei-instrumented repo
    When a new task is submitted with "fix the broken auth middleware"
    Then a session record is created with status "in_progress"
    And the task type is auto-detected as "fix" from the keyword

  Scenario: Per-turn token data is recorded including mid-session model switches
    Given a session is in progress and the developer switches models mid-session
    Then each turn is recorded with the model used for that specific turn
    And user tokens, agent tokens, and context pack tokens are recorded per turn

  Scenario: Session is marked completed when the agent signals task done
    Given a task session with status "in_progress"
    When the agent signals task completion with a summary
    Then the session is updated to status "completed"
    And the end time is recorded

  Scenario: Abandoned session is detected after inactivity
    Given a task session with status "in_progress" and no turns for 4 hours
    When the sensei daemon runs its session cleanup job
    Then the session status is updated to "abandoned"
    And the event is recorded in telemetry
```

### FTR Scoring (First-Time-Right)

FTR is a composite score from 0.0 to 1.0 stored with each task outcome. It aggregates multiple signals including turns taken, refinements requested, test results, revert history, and task revisit rate. Developer vs agent attribution distinguishes a vague spec from a clear spec with wrong output.

```gherkin
Feature: FTR Scoring

  Scenario: FTR score is computed after session completion
    Given a completed session where the task took 3 turns, no refinements, tests passed, and no revert within 10 commits
    When sensei computes the FTR score for that session
    Then an outcome record is created with a score between 0.7 and 1.0
    And each contributing signal is stored alongside the composite score

  Scenario: Task revisit within 7 days reduces the FTR score
    Given a task was completed 5 days ago and a new session opens for the same file and feature area
    When the new session is linked to the original task
    Then the original outcome score is reduced by the revisit penalty
    And the outcome record is updated

  Scenario: Attribution distinguishes vague spec from bad agent output
    Given a task where the developer rated the spec as vague
    And the agent produced multiple refinements
    When sensei computes attribution for that outcome
    Then developer attribution is high and agent attribution is low
    And the FTR penalty is applied to the developer signal, not the agent signal

  Scenario: FTR heuristic fallback runs when no user rating is provided
    Given a completed session with no developer rating after 24 hours
    When sensei runs the heuristic FTR calculation
    Then a score is computed from automated signals only (turns, reverts, test results)
    And the outcome record is created indicating the score was computed by heuristic rather than developer rating
```

### Quality Rating UI

The web dashboard groups session turns by summarised goal and lets the developer rate each goal as pass, fail, or partial. Ratings are stored alongside the task outcome and integrated with Claude's existing feedback mechanism. Historical FTR trends are visible per repo, per agent, and per model.

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
    Then the outcome record for that goal is updated with a "partial" rating
    And the overall session FTR score is recalculated

  Scenario: FTR trend is visible per model in the dashboard
    Given 30 days of task outcomes across two different models
    When the developer opens the Model Comparison view
    Then a trend chart shows average FTR score per week for each model
    And the chart allows filtering by repo and task type
```

### Benchmarking

A task corpus of representative developer tasks enables A/B evaluation. Runs compare with-skills vs without-skills configurations across metrics: tokens, interactions, tool calls, and task completion. Results are stored and compared via CLI.

```gherkin
Feature: Benchmarking

  Scenario: Benchmark run executes all corpus tasks and stores results
    Given a set of representative tasks is configured
    When the developer runs a benchmark with a named configuration
    Then each task is executed against the current agent configuration
    And results are stored with the run identifier, configuration, and per-task metrics

  Scenario: A/B comparison shows improvement from skills
    Given two benchmark runs: one without skills and one with skills
    When the developer compares the two runs
    Then a report is printed showing delta for: tokens, interactions, tool calls, and completion rate
    And the improvement percentage is shown for each metric

  Scenario: FTR improvement from a skill change is measurable
    Given a baseline benchmark run before a skill file was updated
    And a new benchmark run after the skill update
    When the developer compares the two runs
    Then the FTR delta is shown for tasks in the skill's coverage area
    And tasks outside the coverage area show no significant delta
```

### Requirement Quality Scoring

The single highest-leverage variable in FTR is the quality of the requirement given to the agent. Vague requirements produce refinement loops. Concrete requirements with explicit constraints, acceptance criteria, and examples produce first-time-right output. Sensei measures requirement quality automatically and correlates it with outcomes across all sessions — showing developers precisely where upfront investment in elaboration pays off.

Requirement quality is scored at task start (0.0–1.0) based on factors such as description completeness, presence of acceptance criteria, use of specific language, examples provided, and referenced context. The score is stored with the session and correlated with FTR outcomes.

```gherkin
Feature: Requirement Quality Scoring

  Scenario: Vague requirement receives a low quality score
    Given a developer submits the task: "fix the login bug"
    When sensei scores the requirement at session start
    Then the requirement quality score is below 0.3
    And the session is flagged for elaboration guidance
    And the dashboard shows the suggested elaboration prompt inline

  Scenario: Concrete requirement receives a high quality score
    Given a developer submits a task with a clear description, affected area, and acceptance criteria
    When sensei scores the requirement
    Then the requirement quality score is above 0.8
    And no elaboration prompt is shown

  Scenario: Aggregate shows req quality correlates with FTR
    Given 90 days of sessions with both requirement quality score and FTR recorded
    When the developer opens the Insights view
    Then a scatter chart shows requirement quality on the x-axis and FTR on the y-axis
    And the trend line shows positive correlation with statistical confidence displayed
    And the chart is filterable by task type and agent

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
    Given a developer's FTR for bug-fix tasks is below the aggregate average for similar teams
    When sensei generates coaching recommendations
    Then the dashboard shows the developer's score compared to the average for similar teams
    And the top recommendation identifies the highest-leverage habit change for their task type

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
    Given a large sample of sessions from opted-in users
    When the public benchmark page is loaded
    Then it shows: median token reduction (%), average FTR score, average sessions-to-completion
    And metrics are broken down by agent, stack, and task type
    And each metric shows sample size and confidence interval

  Scenario: Aggregate insight feeds back into coaching
    Given new aggregate data shows that repos using a certain feature have higher FTR on UI tasks
    When the coaching engine runs for a developer who has not enabled that feature
    Then a coaching recommendation is generated highlighting the potential FTR improvement

  Scenario: Team benchmark compares against cohort
    Given a team using sensei for 60 days
    When they open the Team Insights view
    Then their aggregate FTR, token reduction, and average turns are shown
    And each metric is compared against the anonymised cohort of teams with the same stack and team size
    And a percentile ranking is shown for each metric
```

---

### Analytics CLI and Dashboard

A CLI command provides a 7-day summary of tool usage, session activity, and token spend. A gaps flag surfaces files and tools that agents bypassed in favour of raw shell commands. The dashboard adds visual FTR scores, model comparisons, benchmark results, and per-session turn timelines.

```gherkin
Feature: Analytics CLI and Dashboard

  Scenario: Stats command shows 7-day summary
    Given the last 7 days contain sessions across multiple repos
    When the developer runs the stats command
    Then the output shows: total sessions, total turns, total tokens, top tools by call count
    And the output fits in a terminal without scrolling for a typical week

  Scenario: Gaps flag reports missed tool opportunities
    Given sessions where the agent ran raw shell commands instead of sensei tools
    When the developer requests the gaps report
    Then each bypassed tool usage is listed with: session, command used, and the sensei tool that could have been used
    And a summary count of total missed opportunities is shown

  Scenario: Machine-readable output is available
    Given a week of session data
    When the developer requests JSON output
    Then the output is valid JSON with the same fields as the default summary
    And it can be piped into standard tools or written to a file without parsing errors

  Scenario: Dashboard shows turn timeline for a session
    Given a session with 8 turns and attached telemetry events
    When the developer opens the session detail view
    Then a timeline is displayed showing each turn's start time, duration, and tool calls
    And the timeline highlights turns with high token spend or errors
```
