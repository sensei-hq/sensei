## ADDED Requirements

### Requirement: Windows prerequisites are checked
`sensei doctor` on Windows SHALL check Git for Windows, the Docker engine, the `sensei-pg` Postgres
container, Ollama, the daemon and the database, resolve what it can (as it does on every platform), and
SHALL report each component's real status instead of "not yet implemented".

#### Scenario: A fresh Windows host
- **GIVEN** a Windows host with none of the prerequisites installed
- **WHEN** `sensei doctor` runs
- **THEN** each component it cannot resolve itself (Docker, the credentials) is listed as failed with its
  remedy, and the command exits non-zero

### Requirement: Postgres runs in a Docker container on Windows
On Windows the Postgres prerequisite SHALL be the `sensei-pg` container from `pgvector/pgvector:pg17`,
published on port 5432 with a named data volume. `sensei doctor` SHALL create the container when it is
missing and start it when it is stopped. The daemon SHALL NOT start or stop the container.

#### Scenario: Container stopped
- **GIVEN** Docker is running and `sensei-pg` exists but is stopped
- **WHEN** `sensei doctor` runs
- **THEN** the container is started, port 5432 answers, and the Postgres check passes

#### Scenario: Docker not running
- **GIVEN** Docker is installed but its engine is not running
- **WHEN** `sensei doctor` runs
- **THEN** the Docker check fails with a remedy to start Docker, and the container check is not reported as passing

### Requirement: Database credentials come from .env
The daemon SHALL read `~/.sensei/.env` when present and connect with its `PGUSER` and `PGPASSWORD`, with
process environment variables taking precedence. Without the file, macOS and Linux SHALL connect as the
OS user as before. On Windows a missing file or a missing `PGUSER` or `PGPASSWORD` SHALL fail the database
check with a remedy naming the file and keys; no password SHALL be generated or defaulted, and the password SHALL NOT
appear in logs or error messages.

#### Scenario: Windows with credentials set
- **GIVEN** `~/.sensei/.env` holds `PGUSER=sensei` and `PGPASSWORD` matching the `sensei-pg` container
- **WHEN** the daemon starts
- **THEN** it connects to Postgres on port 5432 as `sensei` and applies the schema

#### Scenario: Windows without a password
- **GIVEN** no `~/.sensei/.env` on a Windows host
- **WHEN** `sensei doctor` runs
- **THEN** the database check fails, naming `~/.sensei/.env`, `PGUSER` and `PGPASSWORD`, and no connection is attempted with a default password

#### Scenario: macOS unchanged
- **GIVEN** a macOS install with no `~/.sensei/.env`
- **WHEN** the daemon starts
- **THEN** it connects as the OS user exactly as before

### Requirement: Windows remedies use winget
Missing Git for Windows and Ollama SHALL each have a remedy that installs them with winget
(`Git.Git`, `Ollama.Ollama`).

#### Scenario: Ollama missing
- **GIVEN** a Windows host without Ollama
- **WHEN** `sensei doctor` runs
- **THEN** it runs `winget install Ollama.Ollama` and re-checks Ollama

### Requirement: Postgres tools need no host install on Windows
On Windows the database setup and its check SHALL run `psql`, `createdb` and `pg_isready` inside the
`sensei-pg` container, so the host needs no PostgreSQL tools, and the pre-upgrade backup SHALL be written
to the host.

#### Scenario: Database created without host tools
- **GIVEN** a Windows host with `sensei-pg` running and no PostgreSQL tools installed
- **WHEN** `sensei doctor` sets up the database
- **THEN** the database is created and its schema deployed
