Feature: Health tool signals — per-tool cards, curation and the copy contract

  # pinned: unused_when_no_activity_in_two_weeks
  Scenario: a tool with no activity in two weeks becomes unused
    Given a tool whose last use is more than fourteen days ago
    When the signal surface is derived
    Then an unused signal is emitted for it

  # pinned: zero_calls_row_becomes_unused_regardless_of_last_used
  Scenario: a zero-call row reads as unused
    Given a tool with zero calls recorded
    When the signal surface is derived
    Then it is unused regardless of its last-used timestamp

  # pinned: warn_title_includes_tool_name_and_failure_rate
  Scenario: a warn card names the tool and its failure rate
    Given a tool above the warn thresholds
    When the warn signal is rendered
    Then the title carries the tool's short name
    And the failure rate

  # pinned: opportunity_when_moderate_traffic_and_high_error_rate
  Scenario: moderate traffic with errors is an opportunity
    Given a tool with moderate call volume and a high error rate
    When the signal surface is derived
    Then it is an opportunity card

  # pinned: win_when_high_traffic_and_clean
  Scenario: heavy use with a clean record is a win
    Given a tool with high call volume and no errors
    When the signal surface is derived
    Then it is a win card

  # pinned: no_signal_for_light_use_low_error_tools
  Scenario: light use with low errors earns no signal
    Given a tool with few calls and a low error rate
    When the signal surface is derived
    Then no signal is emitted for it

  # pinned: serializes_variant_lowercase_and_optional_action
  Scenario: the wire contract is lowercase variants with an omittable action
    Given a rendered signal
    When it is serialized
    Then the variant is lowercase
    And the action field is absent when there is no action

  # pinned: curate_collapses_multiple_dormants_into_one_summary
  Scenario: several dormant tools collapse into one summary card
    Given more than one dormant tool
    When the list is curated
    Then exactly one unused summary card remains

  # pinned: curate_keeps_single_dormant_as_is
  Scenario: a single dormant tool keeps its own card
    Given exactly one dormant tool
    When the list is curated
    Then it is not collapsed

  # pinned: curate_keeps_all_warns_and_opportunities
  Scenario: every warn and opportunity card survives curation
    Given several warn and opportunity cards
    When the list is curated
    Then all of them remain

  # pinned: curate_collapses_multiple_wins_into_one_summary
  Scenario: several wins collapse into one summary card
    Given more than one win
    When the list is curated
    Then exactly one win summary card remains

  # pinned: curate_orders_warn_then_opportunity_then_unused_then_win
  Scenario: cards render in severity order
    Given a mixed set of cards
    When the list is curated
    Then the order is warn, opportunity, unused, win

  # pinned: short_name_strips_sensei_prefix
  Scenario: every title carries the tool's short name
    Given a prefixed tool name
    When its card is rendered
    Then the displayed name strips the prefix

  # pinned: signal_copy_inputs_summary_dormant_uses_summary_kind_and_count
  Scenario: summary copy carries the count, not a verbatim fallback
    Given a curated unused summary
    When its copy is generated
    Then the kind is summary
    And the count of dormant tools is carried

  # pinned: parse_rejects_over_limit_title
  Scenario: generated copy respects the budget and falls back safely
    Given a generated copy candidate that exceeds the title budget
    When it is parsed
    Then it is rejected
    And the labelled fallback is used instead