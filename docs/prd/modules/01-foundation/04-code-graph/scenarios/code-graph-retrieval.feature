Feature: Code graph retrieval — the API contract the Project window and Atlas consume

  # pinned: get_graph_imports_breakdown_endpoint
  Scenario: the imports breakdown endpoint lists per-file import relationships
    Given a scanned fixture repo
    When GET /api/graph/imports/breakdown is called
    Then it returns the known import relationships for the repo

  Scenario: the node list returns per-node community membership and layout edges
    Given a scanned fixture repo
    When GET /api/graph/nodes?repoId=<fixture> is called
    Then every returned node carries community_id
    And the returned edges include calls, imports, extends (not only calls)

  Scenario: layout edges are the structural set, containment stays overlay-only
    Given a scanned fixture repo
    When the graph API response is inspected
    Then layout is served by calls, imports, extends, implements edges
    And covers, references, rationale_for remain overlay-only

  Scenario: a removed symbol's inbound edges resolve to unresolved rather than vanish
    Given a symbol deleted from its file after a scan
    When the incremental reindex completes
    Then other files' edges that pointed at it keep target_name and set target_id to null

  Scenario: the communities overview is driven by live membership, not the stored count
    Given a scanned fixture repo
    When GET /api/graph/communities/info is called
    Then each community is sized and coloured from live per-node membership
    And no stale node_count drives the overview

  # gap: G-1
  Scenario: the layered hierarchy endpoint is readable at every zoom level
    Given GET /api/graph/<repo>/tree returns the hierarchy
    When a repo with more than 500 nodes is viewed
    Then an L0/L1/L2 level-of-detail exists so no view requires ~1000% zoom