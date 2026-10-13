Feature: Atlas — the code graph canvas, communities and resolution

  # pinned: graph_nodes_returns_community_and_structural_edges
  Scenario: the graph read returns the community and the structural edges
    Given an indexed project
    When its graph nodes are read
    Then each node carries its community and the structural edges are returned

  # pinned: communities_info_uses_live_membership
  Scenario: community info uses live membership
    Given a project whose communities are detected
    When community info is read
    Then it uses the live membership, not a stale cache

  # pinned: graph_nodes_makes_dependency_counting_a_query
  Scenario: dependency counting is a query, not client-side
    Given a project with dependencies
    When graph nodes are read
    Then the dependency counts come from the query

  # pinned: graph_nodes_names_the_repository_and_leaves_an_unattributed_folder_null
  Scenario: a node names its repository or stays null
    Given a node inside a repository and a node outside any
    When graph nodes are read
    Then the first names its repository and the second is null

  # pinned: folder_branch_is_a_typed_column_and_a_graph_nodes_dimension
  Scenario: the folder branch is a typed column and a graph dimension
    Given folders of different kinds
    When graph nodes are read
    Then the folder branch is a typed column and a graph dimension

  # pinned: detect_communities_assigns_deterministic_ids_by_natural_key
  Scenario: community ids are deterministic under nesting
    Given a graph detected twice
    When community ids are assigned
    Then the ids are deterministic by natural key

  # pinned: community_coverage_full_singletons_inherit_file_community
  Scenario: a loose symbol inherits its file's community
    Given a symbol left as a singleton by detection
    When coverage is filled
    Then it inherits its file's community

  # pinned: god_node_ids_rank_by_adjacency_not_by_a_stored_column
  Scenario: god nodes rank by adjacency, not a stored column
    Given a graph with highly connected symbols
    When god nodes are ranked
    Then the ranking uses adjacency, not a stored column

  # pinned: get_callers_by_name_finds_a_caller_through_an_unresolved_edge
  Scenario: a caller is found through an unresolved edge
    Given a symbol whose caller edge was left unresolved
    When its callers are looked up by name
    Then the caller is still found

  # pinned: callers_carry_the_verdict_that_placed_each_edge
  Scenario: callers carry the verdict that placed each edge
    Given a symbol with several callers
    When its callers are read
    Then each carries the resolution verdict that placed its edge

  # pinned: impact_of_symbol_walks_callers_to_depth_and_says_when_it_stopped_early
  Scenario: impact walks callers to depth and says when it stopped early
    Given a dependency chain deeper than the requested depth
    When the impact of a symbol is walked
    Then it stops at the depth and says it stopped early

  # pinned: graph_boundary_explains_a_miss_and_keeps_one_it_cannot_explain
  Scenario: the boundary explains a miss but keeps an honest unknown
    Given a symbol with no callers inside the project
    When its boundary is read
    Then the miss is explained and an unexplainable one is kept

  # pinned: graph_resolution_says_which_rung_placed_each_edge
  Scenario: resolution reports the rung that placed each edge
    Given edges placed by different resolution rungs
    When resolution is read
    Then each edge reports the rung that placed it

  # pinned: a_package_root_symbol_belongs_to_its_package
  Scenario: a package-root symbol belongs to its package
    Given a symbol declared at a package root
    When its package is resolved
    Then it belongs to that package

  # pinned: a_project_reads_only_its_own_schema
  Scenario: a project reads only its own schema
    Given two projects with schema entities
    When one project's schema is read
    Then only its own entities are returned

  # pinned: a_workspace_member_gets_its_own_folder_and_owns_its_files
  Scenario: a workspace member gets its own folder
    Given a multi-member workspace
    When folders are emitted
    Then each member is its own folder and owns its files

  # pinned: the_repo_root_is_always_git_even_when_it_declares_itself
  Scenario: the repo root is always the git root
    Given a member that declares itself the root
    When folders are emitted
    Then the git root is the repo root

  # pinned: folders_are_emitted_parents_before_children
  Scenario: folders emit parents before children
    Given nested folders
    When they are emitted
    Then every parent precedes its children

  # pinned: an_external_import_is_not_a_structural_tie
  Scenario: an external import is not a structural tie
    Given a module that imports from outside the project
    When its community is detected
    Then the external import does not tie it to another

  # pinned: a_chain_reports_each_caller_at_its_distance
  Scenario: a chain reports each caller at its distance
    Given a caller chain
    When the chain is walked
    Then each caller is reported at its distance

  # pinned: a_cycle_terminates_and_the_root_is_never_its_own_caller
  Scenario: a cycle terminates and the root is never its own caller
    Given a cycle in the call graph
    When it is walked
    Then it terminates and the root is not its own caller

  # pinned: named_returns_every_identity_a_bare_name_could_mean
  Scenario: a bare name returns every identity it could mean
    Given a bare symbol name with several identities
    When it is looked up
    Then every identity it could mean is returned

  # pinned: the_reason_labels_round_trip_and_cover_every_variant
  Scenario: resolution reason labels round-trip and cover every variant
    Given a resolution reason
    When it is rendered and parsed back
    Then it round-trips and every variant is covered

  # pinned: a_container_is_totalled_apart_from_what_can_be_called
  Scenario: a container is totalled apart from what can be called
    Given a container symbol
    When reachability is computed
    Then containers are totalled apart from callable symbols

  # pinned: rust_states_its_tests_inline_and_in_a_sibling_file
  Scenario: Rust tests are read inline and from a sibling file
    Given a Rust module with inline and sibling tests
    When its tests are read
    Then both are stated

  # pinned: every_first_party_edge_names_a_declaration_this_scan_holds
  Scenario: every first-party edge names a declaration this scan holds
    Given a scan with first-party edges
    When the scan is accepted
    Then every first-party edge names a declaration the scan holds

  # pinned: no_two_declarations_mint_one_identity
  Scenario: no two declarations share one identity
    Given a scan with declarations
    When identities are minted
    Then no two declarations share one identity

  # gap: G-1
  NO TEST
  Scenario: Atlas reads at three levels of detail
    Given a project whose graph exceeds a single readable scale
    When the view opens at L0
    Then it shows ≤60 labelled blocks grouped by plane, edges as weighted bands, and no individual symbols
    When a group is clicked
    Then L1 shows that module's files and types with external edges as stubs
    When a node is clicked
    Then L2 shows that symbol and its neighbourhood, clickable through to the code
