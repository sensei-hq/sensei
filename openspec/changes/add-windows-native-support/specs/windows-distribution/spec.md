## ADDED Requirements

### Requirement: Releases include Windows assets
Every release SHALL attach `sensei-windows-x86_64.zip` (containing `senseid.exe`, `sensei.exe` and
`sensei-mcp.exe`) and an installer for the desktop app, both built for `x86_64-pc-windows-msvc` from the
tagged commit.

#### Scenario: A tagged release
- **GIVEN** a version tag on `main`
- **WHEN** `release.yml` completes
- **THEN** the GitHub release lists the Windows zip and the Windows installer next to the macOS and Linux assets

### Requirement: The Windows installer puts the CLI tools on PATH
The desktop app installer SHALL install `senseid`, `sensei` and `sensei-mcp` and add them to the user's
`PATH`, so the CLI and MCP server work without the zip.

#### Scenario: After installing the desktop app
- **GIVEN** a Windows user ran the installer
- **WHEN** they open a new terminal and run `sensei --version`
- **THEN** it prints the release's version

### Requirement: Windows builds are unsigned until signing is adopted
Windows assets SHALL be published unsigned, and the README SHALL state the SmartScreen prompt and how to
proceed past it.

#### Scenario: First launch of the installer
- **GIVEN** a tester downloads the Windows installer from the release
- **WHEN** SmartScreen blocks it
- **THEN** the README's Windows section tells them to choose More info → Run anyway
