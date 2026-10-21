Feature: Storage & Data Model — the schema as the single source of truth

  # pinned: every_schema_fact_survives_the_round_trip
  Scenario: every schema fact survives a round-trip
    Given the declared DDL
    When the schema is applied and read back
    Then every schema fact survives the round trip

  # pinned: a_dbd_design_lands_its_detail_on_the_walked_nodes
  Scenario: the dbd design lands its detail on the walked nodes
    Given a dbd design
    When the design is walked
    Then the detail lands on the walked nodes

  # pinned: a_project_reads_only_its_own_schema
  Scenario: a schema-scoped read stays in its own workspace
    Given a workspace-scoped schema
    When a read is issued from that workspace
    Then only its own schema is visible

  # pinned: repositories_schema_invariants
  Scenario: the repository junction holds its schema invariants
    Given repositories with workspace bindings
    When the junction is inspected
    Then the schema invariants hold

  Scenario: a full DDL edit applies without an ALTER
    Given a table whose definition is edited in its .ddl
    When dbd deploy applies it
    Then the resulting object matches the full definition
    And no ALTER-append path was used

  Scenario: enum variants deploy alphabetically
    Given an enum with variants in non-alphabetical declaration order
    When the enum is deployed
    Then the stored order is alphabetical
    And any ranking uses a CASE, never the declaration order

  Scenario: a bridge table enforces one global row
    Given a bridge table with project_id nullable
    When a second global row is inserted
    Then the partial unique index rejects it
    And a workspace-scoped row for the same channel is allowed

  Scenario: a content wipe never destroys user preferences
    Given discovered filesystem state and user intent rows
    When a content wipe runs
    Then the discovered state is cleared
    And the user intent rows survive

  Scenario: seeding is idempotent and timestamp-guarded
    Given a staging table being reseeded
    When the import procedure runs against data with an older timestamp
    Then the existing production data is not overwritten

  Scenario: retention prunes analyzed sessions and keeps unanalyzed ones
    Given sessions past the retention cutoff
    When the retention sweep runs
    Then analyzed sessions and their children are deleted
    And unanalyzed sessions are kept

  Scenario: the one-owner invariant holds after reconcile
    Given a repo whose structural folder nodes duplicated
    When the reconcile runs
    Then every file resolves to exactly one owning folder
    And structural subfolders own no code nodes