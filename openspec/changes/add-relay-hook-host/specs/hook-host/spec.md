## ADDED Requirements

### Requirement: The hook payload carries the terminal environment
The plugin SHALL add `term_program` (the `$TERM_PROGRAM` value) and
`cursor_trace` (whether `$CURSOR_TRACE_ID` is set) to the hook payload it
forwards, where `jq` is available. It SHALL never forward the trace id's
value. Without `jq`, the payload goes unenriched, as today.

#### Scenario: Enriched where jq exists
- **GIVEN** `jq` is on `PATH`, `TERM_PROGRAM=zed`, `CURSOR_TRACE_ID` set
- **WHEN** a hook payload is forwarded
- **THEN** it carries `term_program: "zed"` and `cursor_trace: true`, and no
  key holds the trace id

### Requirement: One shared env-to-host table
`sensei-bootstrap` SHALL own the env→host mapping: `zed` → `zed`;
`vscode` + cursor-trace → `cursor`; `vscode` → `vscode`; anything else
(including `Apple_Terminal`) → none. The MCP binary's `detect_host` SHALL
keep its client-name-first behavior and delegate the env half to the shared
table; the daemon SHALL use the shared table for hook payloads. Neither
crate SHALL carry its own copy of the env rules.

#### Scenario: Same rules both sides
- **WHEN** the env is `TERM_PROGRAM=vscode` with a cursor trace
- **THEN** both the MCP binary (no client name) and the daemon map it to
  `cursor`

### Requirement: Hook events fill a missing session host, never clear one
The daemon SHALL fill `activity.sessions.host` from a hook event's env, via
`apply_to_session` (live and drain alike), when the env maps to a host AND
the seeded catalogue knows that host as an editor. A host the transcript
set, or any earlier event set, SHALL stand: the write is `COALESCE`. An
unmapped env, a non-editor proposal, or a session with no row SHALL change
nothing.

#### Scenario: Zed terminal attributes Zed
- **GIVEN** a session row with null host
- **WHEN** a hook event arrives with `term_program: "zed"`
- **THEN** the row's host is `zed`

#### Scenario: Terminal.app leaves no host
- **GIVEN** a session row with null host
- **WHEN** a hook event arrives with `term_program: "Apple_Terminal"`
- **THEN** the row's host stays null

#### Scenario: A transcript host is never cleared
- **GIVEN** a session row with host `cursor` from its transcript
- **WHEN** a hook event arrives with `term_program: "zed"`
- **THEN** the row's host stays `cursor`
