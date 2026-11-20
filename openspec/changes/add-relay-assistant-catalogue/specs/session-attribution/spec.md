## ADDED Requirements

### Requirement: A session carries the editor it ran in
`activity.sessions` SHALL have a `host` naming the editor a session ran in. Every transcript adapter SHALL declare
its host. Cursor, Zed and VS Code transcripts SHALL set `host` to `cursor`, `zed` and `vscode`. Claude Code,
Copilot CLI and OpenCode transcripts SHALL leave it null. A session with no evidence of an editor SHALL have a null
host and SHALL NOT get a guessed one. A later write with no host SHALL NOT clear a host already set. A transcript
turn SHALL reach its host through its session. The sessions API SHALL serve `host`.

#### Scenario: Cursor transcript
- **WHEN** a Cursor transcript whose model is `claude-sonnet-4-5` is synthesized into a session
- **THEN** the session has `host = 'cursor'` and `model = 'claude-sonnet-4-5'`

#### Scenario: Terminal transcript
- **WHEN** a Claude Code transcript is synthesized
- **THEN** the session's `host` is null

### Requirement: Work is attributed to the assistant's model, not the editor
Model metrics (effectiveness and token metrics) SHALL count an editor-hosted session under its model. No metric row
SHALL be keyed by an editor name.

#### Scenario: Model effectiveness
- **GIVEN** an analyzed Cursor-hosted session with model `claude-sonnet-4-5`
- **WHEN** model effectiveness is read
- **THEN** the session counts under `claude-sonnet-4-5`
- **AND** no row is named `cursor`
