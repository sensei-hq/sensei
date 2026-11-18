## ADDED Requirements

### Requirement: The supported tools are a seeded catalogue
The local database SHALL keep one catalogue of supported tools, `sensei.assistant_catalogue`, seeded through the
repository's import convention (a staging table, its import procedure, and a `database/import/staging` seed file
loaded by `dbd import`). Each row SHALL have an id, a display name, a `kind` of `assistant` or `editor`, the other
spellings the system writes for it (aliases), and whether it can join a crew. Cursor, Zed, VS Code, Windsurf and
Kiro SHALL be editors. Claude Code, Claude Desktop, Codex, Copilot CLI, OpenCode, Aider, Gemini CLI and Continue
SHALL be assistants; Claude Desktop SHALL NOT join a crew. A word with no catalogue row SHALL read `kind:
"unknown"`, named from its own words, and SHALL NOT be classified as an assistant or an editor. Supporting a new
tool SHALL be a new seed row, not a code change. Every surface that names a tool SHALL take the name from this
catalogue, including the assistant detection endpoints and every relay read that carries a lane's family (the
Now lane, the Now run task lanes, the run detail members, and the task view lane).

#### Scenario: Known editor and assistant
- **WHEN** the catalogue is asked about `cursor` and `claude-code`
- **THEN** `cursor` is an editor named "Cursor"
- **AND** `claude-code` resolves through its alias to `claude`, an assistant named "Claude Code"

#### Scenario: Unknown tool
- **WHEN** the catalogue is asked about `muse`
- **THEN** it reads `kind: "unknown"` with the name "Muse", and is neither an assistant nor an editor

#### Scenario: The screen takes names from the daemon
- **GIVEN** a lane registered with `family: "claude-code"` and `host: "zed"`
- **WHEN** `GET /relay/now` is read
- **THEN** the lane carries `family_name: "Claude Code"` and `host_name: "Zed"`

### Requirement: This machine's installs are detected on first run and kept
senseid SHALL store what is installed on this machine in `sensei.assistant_installs`, one row per catalogue tool,
each referencing the catalogue. Detection SHALL run on first run (when the setup wizard reaches its assistants
step, and at daemon start when no install rows exist), after every configure, remove or upgrade, and when the
person asks for it from Settings → Assistants. Settings → Assistants SHALL read the stored rows and show when
they were detected. A tool never detected SHALL have no row and SHALL read as not detected (null), never as not
installed. A detected tool with no catalogue row SHALL NOT be stored. A failed read SHALL be an error, never an
empty list.

#### Scenario: First run stores the installs
- **GIVEN** an empty `assistant_installs` and a machine where Cursor is installed with sensei configured
- **WHEN** the setup wizard reaches its assistants step
- **THEN** `assistant_installs` has a Cursor row with `installed`, `configured`, its config path and `detected_at`

#### Scenario: Configure keeps the row current
- **WHEN** the person configures sensei into an installed assistant
- **THEN** its install row reads `configured` with the time and sensei version

### Requirement: The daemon serves what this machine has seen
`GET /relay/catalogue` SHALL return `assistants`, `editors` and `unknown`. A crew-capable catalogue tool SHALL be
listed in `assistants` or `editors` (by its kind) when its install row says installed or when a session names
it; a word a session names that the catalogue does not know SHALL be listed in `unknown`. Each assistant SHALL
carry `installed` (from its install row; null when never detected), `last_used_at` and `sessions_week` (sessions
started in the last 7 days). Each editor SHALL carry `installed`, `last_used_at`, `models` (the distinct models
of its sessions, newest first), and `mcp_config` (its install row's config path when sensei is configured there,
or null). Assistants SHALL be ordered by `sessions_week` and then `last_used_at`, and editors by
`last_used_at`. The read SHALL NOT probe the filesystem. A value nobody reported SHALL be null. A failed read
SHALL be an error response, never empty lists.

#### Scenario: Seeded sessions
- **GIVEN** a `claude` session started today and a Cursor-hosted session with model `claude-sonnet-4-5`
- **WHEN** `GET /relay/catalogue` is read
- **THEN** Claude Code is under `assistants` with `sessions_week: 1`
- **AND** Cursor is under `editors` with `models: ["claude-sonnet-4-5"]`

#### Scenario: Old usage
- **GIVEN** the only `codex` session started 9 days ago
- **THEN** Codex is listed with `sessions_week: 0` and that session's start as `last_used_at`

#### Scenario: A tool the catalogue does not know
- **GIVEN** a session whose family is `cowork`
- **THEN** `cowork` is listed under `unknown` with `kind: "unknown"`, and not under `assistants` or `editors`

#### Scenario: Read failure
- **WHEN** the session or catalogue read fails
- **THEN** the response is HTTP 500 and carries no lists
