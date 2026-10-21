Feature: Code map — the graph reads, levels, grouping and lenses

  # pinned: graph_nodes_returns_community_and_structural_edges
  Scenario: the graph read returns the community and the structural edges
    Given an indexed workspace
    When its graph nodes are read
    Then each node carries its community and the structural edges are returned

  # pinned: communities_info_uses_live_membership
  Scenario: community info uses live membership
    Given a workspace whose communities are detected
    When community info is read
    Then it uses the live membership, not a stale cache

  # pinned: graph_nodes_makes_dependency_counting_a_query
  Scenario: dependency counting is a query, not client-side
    Given a workspace with dependencies
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
    Given a symbol with no callers inside the workspace
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
  Scenario: a workspace reads only its own schema
    Given two workspaces with schema entities
    When one workspace's schema is read
    Then only its own entities are returned

  # pinned: a_workspace_member_gets_its_own_folder_and_owns_its_files
  Scenario: a package-workspace member gets its own folder
    Given a multi-member package workspace
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
    Given a module that imports from outside the workspace
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
  Scenario: the Code map reads at three levels of detail
    Given a workspace with more than 2,000 symbols
    When the Code map opens
    Then L0 shows at most 60 module blocks, smaller modules folded into "+N more", one weighted band per module pair, and no files or symbols
    When a module is clicked
    Then L1 shows that module's files in columns by role, at most 12 per column, with depends-on and used-by stubs
    When a file is clicked
    Then L2 shows its symbols with callers and callees one hop out, at most 7 a side
    And the breadcrumb climbs back to the workspace

  # gap: G-1
  NO TEST
  Scenario: group-by re-bands L0 without losing the selection
    Given the Code map at L0 with a module selected
    When the grouping changes between layers, folders and communities
    Then the bands, the Wheel's arcs and the Matrix's diagonal blocks follow the grouping
    And the selection is kept and nothing reloads

  # gap: file roles (M4)
  NO TEST
  Scenario: hiding a role removes it from counts, bars and columns
    Given modules with code, test, docs, config and generated files
    When the Code map opens
    Then config and generated files are hidden by default
    When Tests are hidden
    Then every block's file count shrinks and L1 has no Tests column

  # gap: G-1
  NO TEST
  Scenario: a hub is counted, not drawn
    Given a module whose inbound edges exceed 5% of all edges
    When L0 is drawn
    Then the module sits in the hub lane with its edge count and none of its edges are drawn

  # gap: G-1
  NO TEST
  Scenario: referenced libraries are one block, never members
    Given a workspace that references external libraries
    When L0 is drawn
    Then the libraries are one block and never enter module counts

  # gap: G-1
  NO TEST
  Scenario: the matrix colours cells by direction, not by position
    Given modules with upward, same-layer, skipping and cyclic dependencies
    When the Dependency matrix lens is open
    Then upward cells are red, skip and cycle cells are amber, and same-layer cells are neutral

  # gap: Fix 2 (M4)
  NO TEST
  Scenario: the Wheel bundles dependencies between groups
    Given a workspace grouped by communities
    When the Wheel lens is open
    Then modules sit on a ring in their group's arc and dependencies between two groups bundle into one strand

  # gap: co-change mining (M4)
  NO TEST
  Scenario: Hidden coupling shows co-change with no static edge
    Given two modules changed in the same commits with no dependency between them
    When the Hidden coupling lens is open
    Then a dashed band joins them

  # gap: per-function complexity is not stored
  NO TEST
  Scenario: Complexity tints modules by the share of complex functions
    Given modules with functions over cyclomatic complexity 10
    When the Complexity lens is open
    Then a module over 10% is amber and over 20% is red

  # gap: G-1
  NO TEST
  Scenario: switching lens keeps the level where the lens allows it
    Given the Code map at L1 with a module selected
    When the lens switches between Structure, Cycles, Hidden coupling and Complexity
    Then the level and selection are kept
    When the lens switches to Wheel, Zones or the Dependency matrix
    Then the map returns to L0 and keeps the selection

  # gap: G-1
  NO TEST
  Scenario: under 640px every lens is a list
    Given a viewport narrower than 640px
    When any lens is open
    Then it renders as a list with rows at least 44px tall
