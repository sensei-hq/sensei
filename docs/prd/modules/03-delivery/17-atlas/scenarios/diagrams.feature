Feature: Atlas — the Diagrams sub-tree (structure, layers, zones, neighbourhood, schema, world)

  # pinned: structure_diagram_rejects_an_unknown_level_or_kind
  Scenario: the structure diagram refuses an unknown level or kind
    Given a diagram request with an unknown level or kind
    When it is served
    Then it is rejected as a bad request, never an empty payload

  # pinned: structure_level_module_groups_by_the_modules_top_segment
  Scenario: module level groups by the module's top segment
    Given modules with paths
    When the structure diagram is built at module level
    Then nodes group by the module's top path segment

  # pinned: layering_endpoint_ranks_a_seeded_graph_and_names_the_cut
  Scenario: layering ranks a seeded graph and names the cut
    Given a seeded dependency graph with a back edge
    When the layering endpoint runs
    Then it ranks the nodes and names the weakest link as the cut

  # pinned: zones_endpoint_computes_the_main_sequence_and_refuses_to_invent_one
  Scenario: zones computes the main sequence or refuses
    Given a graph with a computable main sequence
    When the zones endpoint runs
    Then it computes the main sequence
    Given a graph where it cannot be computed
    When the zones endpoint runs
    Then it refuses rather than inventing one

  # pinned: the_world_endpoint_nests_every_project_and_refuses_an_unknown_grouping
  Scenario: world nests every project and refuses an unknown grouping
    Given several projects
    When the world diagram is built
    Then every project is nested
    Given an unknown grouping
    When the world diagram is requested
    Then it is refused

  # pinned: the_neighbourhood_endpoint_walks_rings_and_refuses_what_it_cannot_draw
  Scenario: the neighbourhood walks rings and refuses what it cannot draw
    Given a symbol with a call neighbourhood
    When the neighbourhood endpoint runs
    Then it walks the rings outward
    Given a neighbourhood it cannot draw
    When it is requested
    Then it is refused rather than faked

  # pinned: the_callers_hop_stays_in_the_project_and_weighs_call_sites
  Scenario: the callers hop stays in the project and weighs call sites
    Given a symbol called from inside and outside the project
    When the callers hop runs
    Then it stays in the project and weighs the call sites

  # pinned: the_callees_hop_reaches_a_library_but_marks_it_a_leaf
  Scenario: the callees hop reaches a library but marks it a leaf
    Given a symbol calling into a library
    When the callees hop runs
    Then the library is reached but marked a leaf

  # pinned: the_unplaced_counts_are_kept_apart
  Scenario: unplaced edges are counted apart from placed ones
    Given a graph with placed and unplaced edges
    When the neighbourhood is built
    Then the unplaced counts are kept apart

  # pinned: a_changed_version_is_a_miss_and_never_a_stale_hit
  Scenario: a changed version is a cache miss, never a stale hit
    Given a cached diagram for one version
    When the diagram is requested at a changed version
    Then it is a miss and never a stale hit

  # pinned: the_cache_is_bounded_and_drops_the_least_recently_used
  Scenario: the diagram cache is bounded and evicts LRU
    Given a diagram cache at its bound
    When a new diagram is cached
    Then the least recently used is dropped

  # pinned: every_graph_layout_kind_is_a_declared_edge_kind
  Scenario: every layout kind is a declared edge kind
    Given the set of graph layout kinds
    When they are compared to the declared edge kinds
    Then every layout kind is a declared edge kind
