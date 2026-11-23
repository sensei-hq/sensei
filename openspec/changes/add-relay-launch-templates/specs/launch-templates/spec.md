## ADDED Requirements

### Requirement: Launch templates live with the catalogue seed
`sensei.assistant_catalogue` SHALL have a nullable `launch_template` text
column: one command string with a `{repo}` placeholder for the repository to
start in. Terminal assistants SHALL carry `cd "{repo}" && <bin>`; editors SHALL
carry their open-the-agent-chat step naming `{repo}` once verified. Tools whose
invocation is not known SHALL keep `null` — the seed carries only verified
invocations, and editors stay null until their steps are confirmed. The staging table, its import procedure, and
`database/import/staging/assistant_catalogue.jsonl` SHALL carry the column, so
supporting a tool's launch stays a seed row.

#### Scenario: Template resolves against a repo
- **GIVEN** the `claude` row carries `cd "{repo}" && claude`
- **WHEN** the catalogue is read for a repo at `/w/sensei`
- **THEN** the item carries `launch: 'cd "/w/sensei" && claude'`

#### Scenario: Unknown invocation renders nothing
- **WHEN** the catalogue is read and a tool's `launch_template` is null
- **THEN** the item carries `launch: null`, and no launch block renders for it

### Requirement: The catalogue read resolves the repo
`GET /relay/catalogue` SHALL accept `workspace_id` and `crew_id`. It SHALL
resolve `{repo}` to the crew's repository when `crew_id` names a crew with one,
else to the workspace's first repository; with no `workspace_id`, or no repo
on either path, every item's `launch` SHALL be `null`. Each item SHALL carry
its resolved `launch` alongside the #455 fields.

#### Scenario: Crew repo wins
- **GIVEN** crew C runs in repo R and its workspace's first repo is F
- **WHEN** `GET /relay/catalogue?workspace_id=W&crew_id=C` is read
- **THEN** `{repo}` resolves to R's path

#### Scenario: No crew, first repo
- **WHEN** `GET /relay/catalogue?workspace_id=W` is read with no `crew_id`
- **THEN** `{repo}` resolves to W's first repository path

### Requirement: The picked tool's launch renders as its own block
AddMember SHALL render the picked catalogue item's `launch`, when non-null, in
its own block under the paste line with a Copy button, reusing the paste-line
styling. With nothing picked, or a null `launch`, no block SHALL render. The
paste line and its note SHALL NOT change.

#### Scenario: Picked assistant shows its launch
- **GIVEN** the catalogue read carries `launch: 'cd "/w/sensei" && claude'`
  for the picked assistant
- **WHEN** the Crew view renders AddMember
- **THEN** a launch block under the paste line shows that text with a Copy
  button that copies exactly it
