## ADDED Requirements

### Requirement: The sync files a repository under its shared dōjō workspace
The dōjō SHALL file a synced repository under its shared workspace, without any relay run: when the daemon offers the repository's sensei workspace and the dōjō maps the repository to a tenant, it finds or creates that tenant's workspace keyed `(tenant_id, slug)` and links the repository under it.

#### Scenario: An org member's synced repository appears under the org's Workspaces
- **GIVEN** a person is a member of the org dōjō connected to the repository's GitHub org
- **AND** the repository belongs to exactly one sensei workspace
- **WHEN** one `dojo_sync` cycle registers the repository
- **THEN** the register response carries a `workspace_id` for it
- **AND** the org's Workspaces screen (`/org/<slug>/workspaces`) lists that workspace

#### Scenario: A second sync changes nothing
- **GIVEN** the repository was already filed under its workspace
- **WHEN** another sync cycle runs
- **THEN** the Workspaces screen lists the same workspace once, with the same id

#### Scenario: Two members converge on one workspace
- **GIVEN** two members of the same org dōjō sync the same sensei workspace
- **WHEN** both register the repository
- **THEN** the dōjō holds one workspace for `(tenant_id, slug)` and one link for the repository

### Requirement: A refused repository files nothing
The dōjō SHALL file no workspace for a repository it does not map to a tenant, and SHALL refuse to file one
for a caller who is not a member of the repository's tenant.

#### Scenario: An unmapped repository
- **GIVEN** a repository whose GitHub org has no dōjō connection
- **WHEN** the sync registers it with a workspace
- **THEN** it is reported `unmapped` with reason `no_connection`
- **AND** no workspace is created and no link is written

#### Scenario: A non-member caller
- **GIVEN** a caller who is not a member of the repository's tenant
- **WHEN** `dojo.file_repository_workspace` is called for that repository
- **THEN** it raises `insufficient_privilege` and writes nothing

### Requirement: Workspace visibility follows repository access
A person SHALL see a workspace in the dōjō only when they may read at least one of its repositories, are its
viewer, or administer its dōjō.

#### Scenario: A person with no role on the repository
- **GIVEN** a workspace filed in an org dōjō with one repository
- **AND** a signed-in person with no role on that repository and no admin role in that dōjō
- **WHEN** they open the Workspaces screen
- **THEN** the workspace is not listed

### Requirement: A relay run attaches to the shared workspace and never creates one
A relay session push SHALL attach the run to the existing `(tenant_id, slug)` workspace, updating only its
last run time and constitution, and SHALL NOT create a workspace.

#### Scenario: A relay run for a workspace the sync filed
- **GIVEN** the sync filed workspace `sensei` in an org dōjō
- **WHEN** a relay run in that workspace is pushed
- **THEN** the run links to the same workspace id and no second workspace row exists

#### Scenario: A relay run for an unknown workspace
- **GIVEN** no workspace with the run's slug exists in the run's dōjō
- **WHEN** the relay run is pushed
- **THEN** the run has no workspace link and no workspace is created

### Requirement: The production dōjō shows synced workspaces
Before #299 closes, the production dōjō SHALL list the `sensei` workspace under `organization/sensei-hq`
after one sync cycle of the user's enrolled daemon.

#### Scenario: Production evidence
- **GIVEN** the installed daemon's keychain access is approved and it is enrolled with the production dōjō
- **WHEN** one `dojo_sync` cycle completes
- **THEN** the `organization/sensei-hq` Workspaces screen lists `sensei`
