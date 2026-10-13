Feature: Instruments — the health grid, MCP listing and the unbuilt benchmark runner

  # pinned: signal_copy_inputs_per_tool_warn_maps_and_carries_facts
  Scenario: a warn signal maps its copy inputs and carries the facts
    Given a tool with a warn signal
    When its copy inputs are built
    Then they map per tool and carry the failure-rate facts

  # pinned: signal_copy_inputs_variant_to_kind_per_tool
  Scenario: a signal's variant maps to its kind per tool
    Given signals of different variants
    When their copy inputs are built
    Then each variant maps to its kind per tool

  # pinned: signal_copy_inputs_days_change_alters_facts_hash
  Scenario: a change in days alters the facts hash
    Given the same signal with different day windows
    When the copy inputs are built
    Then the facts hash changes

  # pinned: signal_copy_inputs_summary_dormant_uses_summary_kind_and_count
  Scenario: a dormant summary uses the summary kind and count
    Given multiple dormant tools
    When the curated summary is built
    Then it uses the summary kind and the combined count

  # pinned: signal_copy_inputs_summary_wins_uses_summary_kind
  Scenario: a win summary uses the summary kind
    Given multiple win tools
    When the curated summary is built
    Then it uses the summary kind

  # pinned: curate_collapses_multiple_dormants_into_one_summary
  Scenario: multiple dormants collapse into one curated summary
    Given several dormant tools
    When signals are curated
    Then they collapse into one summary

  # pinned: curate_keeps_single_dormant_as_is
  Scenario: a single dormant stays as is
    Given exactly one dormant tool
    When signals are curated
    Then it stays as is

  # pinned: curate_keeps_all_warns_and_opportunities
  Scenario: all warns and opportunities are kept
    Given a mix of warn and opportunity signals
    When signals are curated
    Then all warns and opportunities are kept

  # pinned: curate_collapses_multiple_wins_into_one_summary
  Scenario: multiple wins collapse into one summary
    Given several win tools
    When signals are curated
    Then they collapse into one summary

  # pinned: curate_orders_warn_then_opportunity_then_unused_then_win
  Scenario: curated order is warn → opportunity → unused → win
    Given signals of all four kinds
    When they are curated
    Then the order is warn, opportunity, unused, win

  # pinned: short_name_strips_sensei_prefix
  Scenario: the short name strips the sensei prefix
    Given a tool named sensei__search
    When its short name is taken
    Then it is search

  NO TEST
  Scenario: the tools-health grid renders from live data
    Given tools with real health data
    When the grid renders
    Then it shows servers × tools from live data

  NO TEST
  Scenario: refresh re-reads the health grid
    Given the health grid is rendered
    When refresh is triggered
    Then the grid re-reads from the source

  NO TEST
  Scenario: the MCP server tool listing reads from the manifest
    Given an MCP server with a tool manifest
    When its tools are listed
    Then they come from the manifest

  NO TEST
  Scenario: the Playground issues a tool call and renders the response
    Given a tool in the Playground
    When it is called
    Then the raw response is rendered including errors

  # gap: C-5
  NO TEST
  Scenario: the Benchmark runner executes a suite and stores results
    Given a benchmark suite
    When it is run
    Then results are stored and can be compared over time
    This is unbuilt — no runner code, no test.