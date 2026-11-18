## ADDED Requirements

### Requirement: Every tool is classified as an assistant or an editor
senseid SHALL keep one catalogue of known tools. Each entry SHALL have an id, a display name, and a `kind` of
`assistant` or `editor`. Cursor, Zed, VS Code, Windsurf and Kiro SHALL be editors. Claude Code, Codex, Copilot CLI,
OpenCode, Aider, Gemini CLI and Continue SHALL be assistants. An id the catalogue does not know SHALL read as an
assistant named from its words, and SHALL NOT be classified as an editor. Every surface that names a tool SHALL
take the name from this catalogue, including tool detection and every relay read that carries a lane's family
(the Now lane, the Now run task lanes, the run detail members, and the task view lane).

#### Scenario: Known editor and assistant
- **WHEN** the catalogue is asked about `cursor` and `claude-code`
- **THEN** `cursor` is an editor named "Cursor"
- **AND** `claude-code` is an assistant named "Claude Code"

#### Scenario: Unknown tool
- **WHEN** the catalogue is asked about `muse`
- **THEN** it is an assistant named "Muse", not an editor

#### Scenario: The screen takes names from the daemon
- **GIVEN** a lane registered with `family: "claude-code"` and `host: "zed"`
- **WHEN** `GET /relay/now` is read
- **THEN** the lane carries `family_name: "Claude Code"` and `host_name: "Zed"`

### Requirement: The daemon serves what this machine has seen
`GET /relay/catalogue` SHALL return `assistants` and `editors`. A tool SHALL be listed when detection finds it
installed or when a session names it. Each assistant SHALL carry `last_used_at` and `sessions_week` (sessions
started in the last 7 days). Each editor SHALL carry `last_used_at`, `models` (the distinct models of its sessions,
newest first), and `mcp_config` (where sensei's MCP config was found, or null). Assistants SHALL be ordered by
`sessions_week` and then `last_used_at`, and editors by `last_used_at`. A value nobody reported SHALL be null. A
failed read SHALL be an error response, never empty lists.

#### Scenario: Seeded sessions
- **GIVEN** a `claude` session started today and a Cursor-hosted session with model `claude-sonnet-4-5`
- **WHEN** `GET /relay/catalogue` is read
- **THEN** Claude Code is under `assistants` with `sessions_week: 1`
- **AND** Cursor is under `editors` with `models: ["claude-sonnet-4-5"]`

#### Scenario: Old usage
- **GIVEN** the only `codex` session started 9 days ago
- **THEN** Codex is listed with `sessions_week: 0` and that session's start as `last_used_at`

#### Scenario: Read failure
- **WHEN** the session read fails
- **THEN** the response is HTTP 500 and carries no lists
