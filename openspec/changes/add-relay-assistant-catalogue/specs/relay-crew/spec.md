## ADDED Requirements

### Requirement: AddMember picks from what this machine has seen
AddMember SHALL read `GET /relay/catalogue` when the Crew view opens. It SHALL show two groups.
**Assistants** SHALL show each assistant's name, when it was last used ("not used yet" when never), and its sessions
this week. **Editors** SHALL show each editor's name, when it was last used, the models seen, and where sensei's
tools were found. Picking an assistant SHALL word the note for a session of that assistant opened in the workspace's
repository. Picking an editor SHALL word the note for that editor's agent chat and name its config location. The
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

### Requirement: The host is checked against the session's environment
On `relay_register`, the MCP server SHALL send `host_detected` when the MCP client name or the environment
(`TERM_PROGRAM`, `CURSOR_TRACE_ID`) points to an editor in the catalogue, and SHALL omit it otherwise. The daemon
SHALL keep a reported `host` when there is one, take `host_detected` when no host was reported, and store
`host_detected` on the lane either way. When both are set and differ, the daemon SHALL log a warning, and the member
card's host tag title SHALL add "The session's environment says <Editor>." A `host_detected` longer than 64
characters SHALL be refused.

#### Scenario: Detection fills a blank host (e2e `app/e2e/tests/relay-host.spec.ts`)
- **GIVEN** a lane registers with `host_detected: "zed"` and no `host`
- **WHEN** the person is on the Crew view
- **THEN** its card appears with no reload and shows "in Zed"

#### Scenario: Reported and detected disagree (e2e `app/e2e/tests/relay-host.spec.ts`)
- **GIVEN** a lane registers with `host: "cursor"` and `host_detected: "zed"`
- **THEN** its card shows "in Cursor"
- **AND** the tag's title contains "The session's environment says Zed."

#### Scenario: Client name detection
- **WHEN** the MCP client's `initialize` names Cursor and `relay_register` runs with no `host`
- **THEN** the register body carries `host_detected: "cursor"`

#### Scenario: Terminal session
- **WHEN** the client is Claude Code and `TERM_PROGRAM` is `Apple_Terminal`
- **THEN** the register body has no `host_detected`, and the card shows no host tag
