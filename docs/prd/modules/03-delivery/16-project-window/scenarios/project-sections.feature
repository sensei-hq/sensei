Feature: Project Window — the section panes, membership, patterns, libraries and merge

  # pinned: project_create_and_get
  Scenario: a project is created and read back
    Given a project is created
    When it is read back
    Then its fields round-trip

  # pinned: get_project_metrics_endpoint
  Scenario: the project metrics endpoint serves the metrics
    Given a project with metrics
    When the metrics endpoint is called
    Then the metrics are returned

  # pinned: list_project_dependencies_joins_target_name_and_folder
  Scenario: project dependencies join their target name and folder
    Given a project with dependencies
    When the dependencies are listed
    Then each joins its target name and folder

  # pinned: pattern_upsert_and_list
  Scenario: patterns upsert and list
    Given a pattern is upserted
    When patterns are listed
    Then it appears

  # pinned: library_list
  Scenario: libraries list
    Given libraries are registered
    When they are listed
    Then each appears

  # pinned: build_tree_nests_folders_files_symbols_and_sections
  Scenario: the tree nests folders, files, symbols and sections
    Given a project with folders, files, symbols and sections
    When the tree is built
    Then they nest in that order

  # pinned: derive_conventions_reports_dominant_naming_per_kind
  Scenario: conventions report the dominant naming per kind
    Given code with a dominant naming style per kind
    When conventions are derived
    Then the dominant style is reported per kind

  # pinned: save_memory_scope_validates_spine_slot
  Scenario: saving a memory validates its spine slot
    Given a memory save carrying a spine slot
    When it is saved
    Then the slot is validated

  # pinned: merge_projects_moves_folders_sessions_memories_and_deletes_source
  Scenario: merging projects moves its data and deletes the source
    Given two projects to merge
    When the merge runs
    Then folders, sessions and memories move and the source is deleted

  NO TEST
  Scenario: the solution-track roll-up spans a multi-project engagement
    Given a desktop engagement spanning several projects
    When the roll-up renders
    Then each project's status is summarised without reading code

  NO TEST
  Scenario: the Tasks section renders the project's plan
    Given a project with a registered plan
    When the Tasks section renders
    Then phases, tasks and their states are shown
