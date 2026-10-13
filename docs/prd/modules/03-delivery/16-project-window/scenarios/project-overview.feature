Feature: Project Window — the Overview assembler, membership and the warn rule

  # pinned: kanji_from_a_kanji_icon
  Scenario: a kanji icon is read from jsonb
    Given a project whose icon is a kanji value
    When the header is composed
    Then the kanji is used

  # pinned: kanji_falls_back_for_image_or_missing_or_blank
  Scenario: the header falls back when the icon is not a kanji
    Given a project with an image icon, a missing icon or a blank kanji
    When the header is composed
    Then the default kanji is used

  # pinned: dormant_project_never_warns
  Scenario: a dormant project never warns
    Given a project with no recent sessions
    When its status is decided
    Then it never warns

  # pinned: active_project_warns_on_drift_or_low_ftr
  Scenario: an active project warns on drift or low FTR
    Given a project with recent sessions and open drift
    When its status is decided
    Then it warns
    Given a project with recent sessions and a low seven-day FTR
    When its status is decided
    Then it warns

  # pinned: set_project_icon_round_trips
  Scenario: the project icon round-trips
    Given a project icon is set
    When the project is read back
    Then the icon round-trips

  # pinned: project_ftr_and_quality_decode_numeric_metrics
  Scenario: the project FTR and quality decode from numeric metrics
    Given numeric metric columns
    When the project FTR and quality are read
    Then they decode from the numerics

  # pinned: ftr_getters_read_project_metrics
  Scenario: FTR getters read the project metrics view
    Given a project with an FTR reading
    When the FTR getters read it
    Then they read the project metrics view

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
  Scenario: a multi-repo project offers to combine rather than merging silently
    Given a folder that could belong to two projects
    When the overview composes
    Then a "combine with {other-project}?" banner is shown and nothing is merged without consent
