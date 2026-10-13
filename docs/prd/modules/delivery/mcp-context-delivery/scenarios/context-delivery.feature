Feature: MCP & Context Delivery — pack assembly, scopes and the token budget

  # pinned: assemble_context_blends_three_scopes
  Scenario: context assembles from three scopes
    Given memories at global, stack and project scope
    When the context is assembled
    Then all three scopes are blended

  # pinned: assemble_context_leads_with_slot_anchored_memory
  Scenario: a slot-anchored memory leads the bundle
    Given a memory anchored to the requested slot
    When the context is assembled
    Then that memory leads the bundle

  # pinned: get_context_slot_hint_leads_the_bundle
  Scenario: the slot hint leads the returned bundle
    Given a context request carrying a slot hint
    When GET /api/context is called
    Then the hint leads the bundle

  # pinned: mcp_proxy_knowledge_context_and_rules_resolve_by_project_name
  Scenario: context and rules resolve by project name
    Given a project name rather than an id
    When context and rules are requested
    Then both resolve by the project name

  # pinned: scoped_search_includes_child_folders
  Scenario: a scoped search includes child folders
    Given a project with child folders
    When a scoped search runs
    Then results include the child folders

  NO TEST
  Scenario: a pack carries a per-node resolution level from L0 to L3
    Given a node with a stored summary at a chosen level
    When the pack is assembled
    Then each node carries its smallest sufficient level
    And cost rises monotonically with the level

  NO TEST
  Scenario: a node already sent this session is skipped or upgraded
    Given a per-session sent-log
    When the pack is assembled a second time
    Then a node already seen at this level is skipped
    And an upgrade to a higher level is still sent

  NO TEST
  Scenario: the returned pack fits its token budget
    Given a pack request with a max_tokens budget
    When the pack is assembled
    Then it fits the budget within tolerance
    And any omission is recorded in omitted[]
