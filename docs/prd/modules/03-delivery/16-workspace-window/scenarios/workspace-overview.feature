Feature: Workspace Window — the Overview assembler, membership and the warn rule

  # pinned: kanji_from_a_kanji_icon
  Scenario: a kanji icon is read from jsonb
    Given a workspace whose icon is a kanji value
    When the header is composed
    Then the kanji is used

  # pinned: kanji_falls_back_for_image_or_missing_or_blank
  Scenario: the header falls back when the icon is not a kanji
    Given a workspace with an image icon, a missing icon or a blank kanji
    When the header is composed
    Then the default kanji is used

  # pinned: dormant_project_never_warns
  Scenario: a dormant workspace never warns
    Given a workspace with no recent sessions
    When its status is decided
    Then it never warns

  # pinned: active_project_warns_on_drift_or_low_ftr
  Scenario: an active workspace warns on drift or low FTR
    Given a workspace with recent sessions and open drift
    When its status is decided
    Then it warns
    Given a workspace with recent sessions and a low seven-day FTR
    When its status is decided
    Then it warns

  # pinned: set_project_icon_round_trips
  Scenario: the workspace icon round-trips
    Given a workspace icon is set
    When the workspace is read back
    Then the icon round-trips

  # pinned: project_ftr_and_quality_decode_numeric_metrics
  Scenario: the workspace FTR and quality decode from numeric metrics
    Given numeric metric columns
    When the workspace FTR and quality are read
    Then they decode from the numerics

  # pinned: ftr_getters_read_project_metrics
  Scenario: FTR getters read the workspace metrics view
    Given a workspace with an FTR reading
    When the FTR getters read it
    Then they read the workspace metrics view

  # pinned: build_generalise_message_includes_title_and_bounds_content
  Scenario: the generalise message includes the title and bounds its content
    Given a memory to generalise
    When the generalise message is built
    Then it includes the title and bounds the content

  # pinned: parse_generalise_extracts_text
  Scenario: the generalise reply is parsed into text
    Given a generalise reply, possibly fenced or prose-wrapped
    When it is parsed
    Then the text is extracted

  # pinned: the_shared_contract_demands_a_synthetic_example
  Scenario: both generalise stages demand a synthetic example
    Given the shared generalise contract
    When either stage sends a message
    Then the contract demands a synthetic example

  NO TEST
  Scenario: a multi-repo workspace offers to combine rather than merging silently
    Given a folder that could belong to two workspaces
    When the overview composes
    Then a "combine with {other-workspace}?" banner is shown and nothing is merged without consent
