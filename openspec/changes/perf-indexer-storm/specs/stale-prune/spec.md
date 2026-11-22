## ADDED Requirements

### Requirement: Vanished folders prune instead of failing

A folder-scoped handler whose directory no longer exists SHALL delete its
folder tree (existing cascades) and return Ok with zero items — never a
failure, never a retry. Re-created paths re-enter through discovery.

#### Scenario: Deleted directories converge silently

- **WHEN** `detect_communities` and `scan_git_history` run for 10 deleted
  directories
- **THEN** all 10 folder rows are gone, zero failures record, zero alerts
  derive, and zero retries enqueue

### Requirement: Live folders never prune

The existence check SHALL run immediately before the delete in the same
handler invocation; a directory that exists SHALL never be deleted by
this path, and deletion failures SHALL surface as errors, not skips.

#### Scenario: A present folder is untouched

- **WHEN** the handler runs for an existing directory
- **THEN** the folder row survives and the handler proceeds normally
