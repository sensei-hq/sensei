## ADDED Requirements

### Requirement: AddMember picks from what this machine has seen
AddMember SHALL read `GET /relay/catalogue` when the Crew view opens. It SHALL show two groups.
**Assistants** SHALL show each assistant's name, when it was last used ("not used yet" when never), and its sessions
this week. **Editors** SHALL show each editor's name, when it was last used, the models seen, and where sensei's
tools were found. Picking an assistant SHALL word the note for a session of that assistant opened in the workspace's
repository. Picking an editor SHALL word the note for that editor's agent chat and name its config location. Tools the catalogue does not know, and tools that cannot join a crew (Claude Desktop), SHALL NOT be offered. The
paste line text SHALL stay the same; launch templates are out of scope. A failed read SHALL show an error Notice
with Retry, never empty groups.

#### Scenario: The picker lists seeded tools (e2e `app/e2e/tests/relay-catalogue.spec.ts`)
- **GIVEN** the e2e database has a `claude` session from today and a Cursor-hosted `claude-sonnet-4-5` session
- **AND** the isolated home has sensei configured in `.cursor/mcp.json`
- **WHEN** the person opens the Crew view
- **THEN** Assistants lists "Claude Code" with "1 session this week"
- **AND** Editors lists "Cursor" with `claude-sonnet-4-5` and the `.cursor/mcp.json` path

#### Scenario: Picking an editor (e2e `app/e2e/tests/relay-catalogue.spec.ts`)
- **WHEN** the person picks Cursor
- **THEN** Cursor is pressed
- **AND** the note says to paste the line into Cursor's agent chat and names the config location
- **AND** the paste line text is unchanged

#### Scenario: Catalogue read fails
- **WHEN** the catalogue read fails
- **THEN** AddMember shows "Couldn't read the assistants" with Retry, and the paste line stays usable

### Requirement: The host is detected, with the self-report as fallback
On `relay_register`, the MCP server SHALL send `host_detected` when the MCP client name from the `initialize` handshake (`clientInfo.name`) names an editor, or, failing that, when `TERM_PROGRAM` (with `CURSOR_TRACE_ID`) names one, and SHALL omit it otherwise. The daemon SHALL store as the lane's host the detected value when the catalogue knows it as an editor, else the lane's self-reported `host` (trimmed, non-blank), else none. When both are set and differ, the daemon SHALL log a warning naming the lane and both values, and the detected value SHALL win. A `host_detected` the catalogue does not know as an editor SHALL be ignored with a warning and never stored. Detection SHALL NOT guess: a client name or environment it does not recognise SHALL give no detection. A `host_detected` longer than 64 characters SHALL be refused.

#### Scenario: Detection fills a blank host (e2e `app/e2e/tests/relay-host.spec.ts`)
- **GIVEN** a lane registers with `host_detected: "zed"` and no `host`
- **WHEN** the person is on the Crew view
- **THEN** its card appears with no reload and shows "in Zed"

#### Scenario: Detected wins over a different report (e2e `app/e2e/tests/relay-host.spec.ts`)
- **GIVEN** a lane registers with `host: "cursor"` and `host_detected: "zed"`
- **THEN** its card shows "in Zed"
- **AND** the daemon logs a warning naming both values

#### Scenario: The self-report is the fallback (e2e `app/e2e/tests/relay-host.spec.ts`)
- **GIVEN** a lane registers with `host: "cursor"` and no `host_detected` (for example over plain HTTP)
- **THEN** its card shows "in Cursor"

#### Scenario: Client name detection
- **WHEN** the MCP client's `initialize` names Cursor and `relay_register` runs
- **THEN** the register body carries `host_detected: "cursor"`

#### Scenario: Terminal session
- **WHEN** the client is Claude Code and `TERM_PROGRAM` is `Apple_Terminal`, and no `host` is reported
- **THEN** the register body has no `host_detected`, and the card shows no host tag
