# dojo-diagrams

## ADDED Requirements

### Requirement: Graph reads are repository- and branch-scoped under member RLS
The dōjō SHALL read a repository's code graph for one tracked branch
from `dojo.graph_files`, `graph_nodes`, `graph_edges`, and
`graph_schema_*`. Every read SHALL pass through
`dojo.can_read_repository`: a member outside the owning teams reads
nothing (empty by policy, indistinguishable from no sync — the read
SHALL NOT reveal which).

#### Scenario: Non-member reads nothing
- **GIVEN** a synced repository in tenant T
- **WHEN** a caller outside T's teams reads its graph
- **THEN** the read answers empty with no error and no membership signal

### Requirement: References resolve at read time; the unresolved show as external
The read SHALL resolve edge targets and schema references — `(folder,
fqn)` keys — against the same branch's nodes. A key with no node SHALL
appear as an external reference, never dropped and never an error.

#### Scenario: Dangling edge target
- **GIVEN** an edge whose `(target_folder, target_fqn)` has no node on the branch
- **WHEN** the graph is read
- **THEN** the edge lists under externals with its folder + fqn intact

### Requirement: The dōjō port answers the codemap wire shapes
The dōjō SHALL implement `CodeMapPort` over its graph reads: `level0`
(structure/layers), `module`, `file`, `zones`. `cochange` SHALL answer
`commits: 0` with no pairs (no history is synced); `complexity` SHALL
answer `ok: false` naming that Measures are not synced. A branch with
no synced files SHALL answer an honest empty, never fabricated blocks.

#### Scenario: Structure of a synced branch
- **GIVEN** a branch with synced files, nodes, and edges
- **WHEN** `level0('folders', null)` is read
- **THEN** blocks cover the synced modules, bands cover the resolved
  edges, and externals list under libraries

### Requirement: Branch picker defaults to the tracked default
The diagram screens SHALL offer the repository's tracked branches from
`my_graph_branches`, defaulting to the branch flagged `is_default`.
Switching branch SHALL switch the graph (and metrics on screens that
show both).

#### Scenario: Switch branch switches graph
- **GIVEN** a repository with tracked branches `main` (default) and `dev`
- **WHEN** the picker switches to `dev`
- **THEN** every diagram re-reads for `dev`

### Requirement: Tracked branches are managed behind repository authority
Adding or removing a tracked branch SHALL go through
`dojo.set_graph_branch` and SHALL be offered only to whoever holds
authority over the repository. Any other caller SHALL NOT see the
control.

#### Scenario: Non-authority sees no control
- **GIVEN** a team member without repository authority
- **WHEN** they open the repository's diagram settings
- **THEN** no add/remove-branch control is rendered
