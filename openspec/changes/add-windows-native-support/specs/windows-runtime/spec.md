## ADDED Requirements

### Requirement: The workspace builds for Windows
Every crate in the Rust workspace SHALL compile for `x86_64-pc-windows-msvc`, and CI on develop SHALL fail
when it does not.

#### Scenario: Unix-only code is introduced
- **GIVEN** a change that calls a Unix-only API without a Windows path
- **WHEN** CI runs the Windows check job
- **THEN** the job fails and the change cannot merge

### Requirement: The daemon resolves the user's home directory on Windows
`home_dir()` SHALL return the user's profile directory on Windows and `HOME` on Unix, and SHALL return an
error when neither is available, never a temporary directory.

#### Scenario: No HOME on Windows
- **GIVEN** a Windows user session where `HOME` is unset and the profile directory is `C:\Users\ana`
- **WHEN** the daemon starts
- **THEN** it reads and writes `C:\Users\ana\.sensei`

### Requirement: The daemon shuts down gracefully on Windows
The daemon SHALL run its graceful shutdown when Windows asks the process to stop (Ctrl-C, Ctrl-Break,
console close, or system shutdown), as it does for SIGINT and SIGTERM on Unix.

#### Scenario: Stopping the daemon on Windows
- **GIVEN** a running daemon on Windows
- **WHEN** `sensei stop` runs
- **THEN** the daemon logs its graceful shutdown and exits, and `sensei status` reports it stopped

### Requirement: Shell commands run through Git for Windows
Where the daemon runs a POSIX shell command, it SHALL use `sh` on Unix and Git for Windows' `sh.exe` on
Windows, and SHALL report an error naming Git for Windows when no shell is found.

#### Scenario: Git for Windows missing
- **GIVEN** a Windows host without Git for Windows
- **WHEN** the daemon runs a coverage command
- **THEN** the task fails with an error that names Git for Windows, and no coverage value is recorded

### Requirement: The daemon runs as a per-user logon task on Windows
`sensei start` on Windows SHALL register and run a per-user Task Scheduler task that starts the daemon at
logon, and `sensei stop` SHALL stop it, so the daemon runs as the signed-in user.

#### Scenario: Daemon returns after sign-in
- **GIVEN** `sensei start` has run on Windows
- **WHEN** the user signs out and signs back in
- **THEN** the daemon is running as that user and reads that user's Credential Manager entries
