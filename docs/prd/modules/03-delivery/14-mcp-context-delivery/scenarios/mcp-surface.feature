Feature: MCP & Context Delivery — the tool catalog, the defaults contract, and discovery

  # pinned: catalog_exposes_exactly_the_expected_tools
  Scenario: the tool catalog exposes exactly the expected tools
    Given the sensei MCP server
    When the tool catalog is listed
    Then it contains exactly the declared tools and nothing else

  # pinned: every_tool_has_a_well_formed_schema
  Scenario: every tool carries a well-formed input schema
    Given every tool in the catalog
    When its input schema is inspected
    Then it is a well-formed object schema

  # pinned: lib_docs_tools_declare_their_documented_params
  Scenario: the docs tools declare their documented parameters
    Given the library-docs tools
    When their schemas are read
    Then each documented parameter is declared

  # pinned: initialize_reports_protocol_and_server_info
  Scenario: initialize reports the protocol version and server info
    Given an MCP initialize request
    When the server responds
    Then it reports the protocol version and server info

  # pinned: build_daemon_params_injects_repo_id_and_mirrors_query
  Scenario: daemon params inject the repo id and mirror the query
    Given a tool call with a workspace name
    When the daemon params are built
    Then the repo id is injected and the query is mirrored

  # pinned: build_daemon_params_derives_query_from_name_then_pattern
  Scenario: daemon params derive the query from name, then pattern
    Given a call carrying a name and a pattern
    When the daemon params are built
    Then the query is derived from the name first, falling back to the pattern

  # pinned: build_daemon_params_renames_pattern_to_tag
  Scenario: the pattern argument is renamed to tag
    Given a call carrying a pattern argument
    When the daemon params are built
    Then the pattern is renamed to tag

  # pinned: get_lib_docs_defaults_the_asking_folder_to_cwd
  Scenario: the library-docs tool defaults the asking folder to cwd
    Given a library-docs call with no folder
    When the params are built
    Then the asking folder defaults to the current working directory

  # pinned: unknown_tool_defaults_to_the_mcp_proxy
  Scenario: an unknown tool falls through to the MCP proxy
    Given a call naming a tool the daemon does not specialise
    When it is dispatched
    Then it is proxied rather than refused

  # pinned: resolve_project_matches_by_name_not_empty
  Scenario: workspace resolution matches by name, never an empty id
    Given a call carrying a workspace name
    When the workspace is resolved
    Then it resolves by name and is never left empty

  # pinned: resolve_from_cwd_matches_longest_folder_prefix
  Scenario: cwd resolution matches the longest folder prefix
    Given two nested folders registered for a repo
    When resolution runs from a path inside both
    Then the longest matching folder prefix wins

  # pinned: resolve_from_cwd_auto_picks_single_repo_under_parent_else_ambiguous
  Scenario: a single repo under a parent auto-picks, two is ambiguous
    Given a parent folder containing one repo
    When resolution runs from the parent
    Then the single repo is auto-picked
    Given a parent folder containing two repos
    When resolution runs from the parent
    Then the result is ambiguous

  # pinned: memory_writes_target_the_right_endpoints_and_bodies
  Scenario: memory writes target the right endpoints and bodies
    Given a memory write tool call
    When it is dispatched
    Then it targets the correct endpoint with the correct body

  # pinned: every_manifest_has_the_playground_shape
  Scenario: every Playground manifest has the expected shape
    Given the Playground manifests
    When each is inspected
    Then it has the declared shape and a dotted id

  # pinned: every_enum_input_carries_its_options
  Scenario: every enum input carries its options
    Given a tool with an enum input
    When its manifest is read
    Then the input carries its options

  # pinned: every_listed_tool_has_a_match_arm_in_the_dispatcher
  Scenario: every listed tool has a dispatcher arm
    Given every tool listed in the manifests
    When the dispatcher is inspected
    Then each has a match arm and a call dispatch

  # pinned: discover_project_scope_finds_dot_mcp_json
  Scenario: discovery finds a workspace-scope .mcp.json
    Given a workspace with a .mcp.json
    When discovery runs
    Then the workspace-scope servers are found

  # pinned: discover_user_scope_finds_claude_cursor_zed
  Scenario: discovery finds user-scope configs across assistants
    Given user-scope configs for Claude, Cursor and Zed
    When discovery runs
    Then all three families are found

  # pinned: read_jsonc_tolerates_comments
  Scenario: JSONC configs are read with comments tolerated
    Given a config file containing comments
    When it is read
    Then the comments are tolerated and the section parses

  # pinned: acp_family_as_str_matches_ddl_check_values
  Scenario: the assistant-family labels match the DDL check values
    Given the assistant family enum
    When each family is rendered to its database string
    Then every string matches the DDL check values

  NO TEST
  Scenario: the third-party MCP server registry lists, enables and refreshes servers
    Given a discovered third-party MCP server
    When the servers registry is queried
    Then it is listed with its enabled state and its tools
    And refresh re-probes it
