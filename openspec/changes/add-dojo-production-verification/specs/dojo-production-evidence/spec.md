## ADDED Requirements

### Requirement: One command gathers dōjō evidence
`sensei dojo evidence [--host <host>] [--since <duration>] [--show-keys]` SHALL read, without
writing, the local daemon's `GET /api/auth/status`, `GET /api/dojo/memberships`,
`GET /api/dojo/sync-state` and `GET /api/runs`, and the target dōjō's `GET /version`, and print a
markdown evidence block. The host defaults to `dojo.sensei-hq.com` and `--since` to 24 hours.
A sync row SHALL count for a leg only when it is `synced`, its `synced_at` is within `--since`, and
its `persona` is the `persona_slot` of a connection whose `dojo_url` is on the host. A run SHALL
count only when it has a `dojo_session_id`, was created within `--since`, and its publish could
only reach the host: its workspace is bound to a connection on the host, or it is unbound and every
enabled connection is on the host.

#### Scenario: All legs present
- **WHEN** a connection's `dojo_url` is on the host, its persona has synced `dojo_sync_plan`, `repository`, `repository_metric` and `repository_graph` rows, a run has a `dojo_session_id`, and `/version` reports push configured
- **THEN** the block lists the dōjō gitSha, push configured, each count, and the command exits 0

#### Scenario: A leg is missing
- **WHEN** no `repository_graph` row is synced for a connection on the host
- **THEN** the block marks the graph leg as missing by name and the command exits 1

#### Scenario: A stale row does not count
- **WHEN** the only synced `repository_graph` row has a `synced_at` older than `--since`
- **THEN** the graph leg is marked missing and the command exits 1

#### Scenario: The daemon syncs to a different dōjō
- **WHEN** `GET /api/auth/status` reports `dojo` as `http://127.0.0.1:5173` and the host is `dojo.sensei-hq.com`
- **THEN** the command says the daemon syncs to the other dōjō and exits 1

#### Scenario: A run that may have gone elsewhere does not count
- **WHEN** the only run with a `dojo_session_id` is unbound and an enabled connection points at another host
- **THEN** the relay leg is marked unattributed and the command exits 1

#### Scenario: No connection targets the host
- **WHEN** every connection points at `127.0.0.1:5173`
- **THEN** the command says no connection targets `dojo.sensei-hq.com` and exits 1

### Requirement: Evidence is safe to post publicly
By default the evidence block SHALL contain counts, the host, the dōjō build and timestamps only —
no repository keys, emails, prompts or transcript text. Repository keys SHALL appear only with
`--show-keys`.

#### Scenario: Default output
- **WHEN** the command runs without `--show-keys`
- **THEN** the output contains no repository key and no email address

#### Scenario: Keys requested
- **WHEN** the command runs with `--show-keys`
- **THEN** the synced repository keys are listed under each leg

### Requirement: A failed read is an error, not zero
If the daemon or the dōjō cannot be read, the command SHALL report which read failed and exit
non-zero; it SHALL NOT report zero counts for that leg.

#### Scenario: Daemon down
- **WHEN** `GET /api/dojo/sync-state` returns 500
- **THEN** the command prints that the sync-state read failed and exits non-zero without a counts table
