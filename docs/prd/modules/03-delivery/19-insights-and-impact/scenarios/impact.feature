Feature: Insights & Impact — the impact endpoint and the unbuilt tables
 
   # NO TEST: get_project_impact_endpoint
   Scenario: the project impact endpoint returns the verdict, trend and source memory
    Given a project with FTR history and a verdict
    When the impact endpoint is called
    Then it returns the verdict, the trend markers and the source memory link

  # NO TEST: list_impact_verdicts
  Scenario: listing impact verdicts returns the project's verdicts
    Given a project with several impact verdicts
    When the list endpoint is called
    Then it returns all verdicts

  # NO TEST: create_impact_verdict
  Scenario: creating an impact verdict stores it
    Given a project
    When an impact verdict is created
    Then it is stored and can be listed

  # NO TEST: decide_impact_verdict
  Scenario: deciding an impact verdict updates its state
    Given a pending impact verdict
    When it is decided
    Then its state updates

  # NO TEST: tool_insights_endpoint
  Scenario: the observatory tool-insights endpoint returns tool-level insights
    Given tool usage and verdict data
    When the tool-insights endpoint is called
    Then it returns the curated insights

  NO TEST
  Scenario: the insights table stores the synthesised narration
    Given the insights table exists
    When a narration is synthesised
    Then it is stored with its cache key
    This is unbuilt — no `insights` table in DDL.

  NO TEST
  Scenario: applied_recommendations records what was applied and when
    Given the applied_recommendations table exists
    When a recommendation is applied
    Then it is recorded with the applied timestamp
    This is unbuilt — no `applied_recommendations` table in DDL.

  NO TEST
  Scenario: impact_regressions records FTR regressions tied to a change
    Given the impact_regressions table exists
    When an FTR regression is detected
    Then it is recorded with the change context
    This is unbuilt — no `impact_regressions` table in DDL.

  NO TEST
  Scenario: GET /api/impact returns the cross-project impact summary
    Given the impact endpoint is built
    When it is called
    Then it returns the cross-project summary
    This is unbuilt — no `GET /api/impact` route.

  # gap: C-8
  NO TEST
  Scenario: the full-screen impact alert surfaces a critical regression
    Given a critical FTR regression
    When the alert fires
    Then the full-screen alert renders with the revision
    This is the C-8 gap — impact-alert full-screen unbuilt.