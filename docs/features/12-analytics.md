---
id: analytics
type: feature
---

# Analytics

The analytics module captures every tool call Claude makes — including MCP tools, built-in tools, and bash commands — and stores them locally in SQLite. This creates a ground-truth record of how sensei tools are actually used, whether they're being bypassed in favour of raw bash, and how often they succeed. The `sensei stats` command surfaces this data so teams can measure sensei's real-world impact and identify missed opportunities.

## Features

### Telemetry Collector

The collector captures `PreToolUse` and `PostToolUse` hook events for every Claude tool call and persists them to SQLite via a lightweight local daemon.

```gherkin
Feature: Telemetry Collector

  Scenario: Daemon starts on login
    Given a user has run sensei setup
    When the operating system starts
    Then the sensei daemon starts automatically via launchd (macOS) or systemd (Linux)
    And it listens on localhost port 51789
    And it is ready to accept POST /event requests

  Scenario: Hook fires on every tool call
    Given the daemon is running
    And Claude executes any tool call
    When PreToolUse or PostToolUse fires
    Then the hook script sends POST /event to localhost:51789 within 100ms
    And the event includes: tool name, phase (pre/post), timestamp, session_id, project_path

  Scenario: PostToolUse captures duration and outcome
    Given a tool call completes
    When PostToolUse fires
    Then the event includes: duration_ms and success (true/false)
    And on failure, the event includes the error message

  Scenario: JSONL fallback when daemon is unavailable
    Given the daemon is not running
    When a hook fires
    Then the event is written to ~/.sensei/<uuid>/events.jsonl
    And the hook exits within 100ms (non-blocking)

  Scenario: Daemon drains JSONL on startup
    Given there are buffered events in events.jsonl
    When the daemon starts
    Then it imports all buffered events into SQLite
    And clears the JSONL file on success

  Scenario: UUID is generated once
    Given no UUID exists at ~/.sensei/uuid
    When the daemon starts for the first time
    Then a random UUID is generated and written to ~/.sensei/uuid
    And all subsequent events include this UUID

  Scenario: Hook scripts are installed by sensei setup
    Given a user runs sensei setup
    Then hook scripts are written to ~/.claude/hooks/pre-tool-use.sh and post-tool-use.sh
    And the hooks are registered in ~/.claude/settings.json
```

### Analytics CLI

The `sensei stats` command queries the local SQLite database and prints summary reports to the terminal.

```gherkin
Feature: Analytics CLI

  Scenario: Default 7-day summary
    Given the daemon has collected events
    When the user runs sensei stats
    Then a summary of the last 7 days is printed
    And it shows: total tool calls, top 5 tools by call count, success rate per tool
    And it shows: active sessions count, projects seen

  Scenario: All-time summary
    Given the user runs sensei stats --all
    Then the summary covers all recorded data, not just 7 days

  Scenario: Filter by tool
    Given the user runs sensei stats --tool search_index
    Then only events for search_index are shown
    And the output includes: call count, avg duration_ms, success rate, last called

  Scenario: Filter by session
    Given the user runs sensei stats --session <session_id>
    Then only events from that session are shown
    And the output shows the full chronological call sequence

  Scenario: Filter by date range
    Given the user runs sensei stats --since 2026-01-01
    Then only events on or after 2026-01-01 are shown

  Scenario: JSON output
    Given the user runs sensei stats --json
    Then the output is valid JSON
    And it includes the same data as the default text report

  Scenario: Gaps report
    Given the user runs sensei stats --gaps
    Then the output lists bash commands that sensei tools could have replaced
    And each gap entry shows: the bash command pattern, how often it appeared,
        and the suggested sensei tool
    And the report is sorted by frequency (most common gaps first)
```
