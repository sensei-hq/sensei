Feature: Code graph retrieval — the API contract the Workspace window and the Code map consume

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

  # gap: file roles
  NO TEST
  Scenario: every file carries one role
    Given a repository with source, test, doc, config and generated files
    When it is indexed
    Then each file node carries exactly one role of code, test, docs, config or generated
    And the role comes from path conventions and the indexer's file kind

  # gap: Fix 2
  NO TEST
  Scenario: edges between two communities aggregate into one weighted relation
    Given communities with calls and imports between them
    When community info is read
    Then each community pair carries one relation weighted by the edges behind it

  # gap: co-change mining
  NO TEST
  Scenario: co-change pairs are mined from git history
    Given commits that touch the same two files
    And a commit that touches more than 50 files
    When co-change is mined
    Then the two files are a co-change pair weighted by their shared commits
    And the large commit contributes no pairs

  # gap: branch scope
  NO TEST
  Scenario: only environment branches sync, and the latest push per branch wins
    Given production mapped to main and development mapped to develop
    When graphs are pushed for main, develop and a feature branch
    Then the main and develop graphs are synced, each tagged with its environment
    And the feature branch graph is not synced
    When develop is pushed again
    Then the newer develop graph replaces the older one

  # gap: branch scope
  NO TEST
  Scenario: an unmapped environment falls back to production
    Given a repository whose default branch is master and no testing or development mapping
    When a sprint target reads development
    Then it reads master's graph
