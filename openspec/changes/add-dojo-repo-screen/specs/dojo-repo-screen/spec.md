# dojo-repo-screen

## ADDED Requirements

### Requirement: Member-readable per-repository metric series
The dōjō SHALL serve one repository's daily metric series to tenant
members at `GET /v1/t/{origin}/{org}/repositories/[id]/metrics`,
reading `repository_metric_series` as the caller (its invoker policy
decides row by row). A repository outside the addressed tenant SHALL
answer 404; a read failure SHALL answer 5xx, never [].

#### Scenario: Outside the tenant
- **GIVEN** a repository of tenant B
- **WHEN** a member of tenant A reads its metrics through A's URL
- **THEN** the answer is 404

### Requirement: One repo screen with metrics and diagrams
Each repository SHALL have a screen showing its header (name, key,
visibility, activity, tracked branches), its metric cards over the
window, and a link into its diagrams route preserving the branch. A
member outside the owning teams SHALL see nothing (the reads gate);
an empty window SHALL read as empty, a failed read as an error.

#### Scenario: Branch preserved into diagrams
- **GIVEN** the repo screen on branch `dev`
- **WHEN** the reader follows the diagrams link
- **THEN** the diagrams open on `dev`
