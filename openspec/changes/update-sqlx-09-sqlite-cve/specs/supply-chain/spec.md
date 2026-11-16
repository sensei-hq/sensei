## ADDED Requirements

### Requirement: No shipped SQLite advisory
`cargo audit` SHALL pass with no ignore entry for RUSTSEC-2026-0459, and the bundled SQLite linked
into senseid SHALL come from `libsqlite3-sys` 0.39 or later.

#### Scenario: The audit after the upgrade
- **GIVEN** the upgraded lockfile
- **WHEN** CI runs `cargo audit`
- **THEN** it passes and `.cargo/audit.toml` no longer lists RUSTSEC-2026-0459

#### Scenario: Transcript readers still read
- **GIVEN** a Zed, VS Code, Copilot CLI and opencode SQLite transcript fixture
- **WHEN** the readers run on the upgraded rusqlite
- **THEN** each yields the same turns as before the upgrade
