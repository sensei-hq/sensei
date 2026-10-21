Feature: Dōjō governance authoring — author at a scope, compose down the ladder

  # gap: dojo-authoring-surface (epic #91)
  NO TEST
  Scenario: the scope rail groups the dōjō ladder and marks which kinds are paid
    Given a maintainer with their own rules in sensei and a company Dōjō
    When they open the governance authoring surface
    Then scopes are grouped Personal · Dōjō · Client · Workspace · Repository
    And there is no Team group and no Stack group (D25, D29)
    And Dōjō · Client are marked paid while Personal reads free

  # gap: dojo-authoring-surface (epic #91)
  NO TEST
  Scenario: Company vs Client is derived from membership, not chosen
    Given a user whose employer owns one Dōjō and who is engaged with another
    When the scope rail renders both
    Then the employer Dōjō is captioned "your employer"
    And the engaged Dōjō is captioned "engagement · not your employer"

  # gap: dojo-authoring-surface (epic #91)
  NO TEST
  Scenario: a stack applied at a level applies from that level down (D29)
    Given a workspace under the dōjō, with the React stack's rules and packs applied at the workspace
    When the inherited set of a repository in that workspace is composed
    Then the dōjō contributes
    And the React stack's rules contribute, attributed to the workspace

  # gap: dojo-authoring-surface (epic #91)
  NO TEST
  Scenario: a repository rule is the most specific level of the dōjō ladder
    Given a rule authored at the dōjō, at a workspace, and at one of its repositories
    When the rules for that repository are composed
    Then the repository's rule wins over the workspace's, which wins over the dōjō's

  # gap: dojo-authoring-surface (epic #91)
  NO TEST
  Scenario: the stance dials show the consequence of the selected stop
    Given a scope with Sharing set to Opt-in
    When the stance renders
    Then Sharing reads "Lessons queue in ready-to-share; you confirm each batch."

  # gap: dojo-authoring-surface (epic #91)
  NO TEST
  Scenario: a scope never authored renders the default stance
    Given a scope with no authored governance
    When the stance renders
    Then autonomy reads Balanced, sharing reads Opt-in, and review reads 2 approvals

  # gap: dojo-authoring-surface (epic #91)
  NO TEST
  Scenario: a more specific scope's stance beats the default and lower rungs
    Given a default stance and a stance authored on a tighter scope
    When the stance resolves for the tighter scope
    Then the tighter scope's stance is used

  # gap: dojo-authoring-surface (epic #91)
  NO TEST
  Scenario: a strict stance fails closed on every dial
    Given a scope marked strict
    When autonomy is evaluated
    Then every autonomous action is refused unless explicitly permitted
    The authoring surface has no strict control yet — see Open gaps.

  # gap: dojo-authoring-surface (epic #91)
  NO TEST
  Scenario: anonymization is not an authorable stop
    Given a maintainer authoring at a client scope
    When the stance renders
    Then client anonymization is shown as always on with no stop to change

  # gap: dojo-authoring-surface (epic #91)
  NO TEST
  Scenario: each artifact section counts what is here and what is inherited
    Given a scope with two authored rules and five inherited rules
    When the rules section renders
    Then its header reads "2 here · 5 inherited"
    And no empty copy is shown while anything is present

  # gap: dojo-authoring-surface (epic #91)
  NO TEST
  Scenario: rule kinds render their own glyph and type chip
    Given a scope that authored a guard, a principle, a pattern, and an anti-pattern
    When the rules section renders
    Then each row carries its glyph and a chip reading its type

  # gap: dojo-authoring-surface (epic #91)
  NO TEST
  Scenario: learnings are authored at workspace scope only
    Given a workspace with recorded learnings and the dōjō above it with none
    When both scopes render
    Then the workspace shows Memory & learnings with each learning's tag
    And the dōjō shows no memory section

  # gap: dojo-authoring-surface (epic #91)
  NO TEST
  Scenario: an inherited item is dimmed and attributed to its source scope
    Given a workspace whose dōjō authored a pattern at the dōjō level
    When the rules section renders
    Then the pattern is dimmed and attributed to the dōjō

  # gap: dojo-authoring-surface (epic #91)
  NO TEST
  Scenario: a local rule that shadows an inherited one is labelled as an override
    Given a workspace whose local rule repeats a company rule's text
    When the rules section renders
    Then the local rule reads "overrides ↑ Company"
    And the inherited rule is struck through and reads "overridden here"

  # gap: dojo-authoring-surface (epic #91)
  NO TEST
  Scenario: a collision is surfaced rather than hidden
    Given a scope where a local rule shadows an inherited one
    When the rules section renders
    Then both rules are still listed

  # gap: dojo-authoring-surface (epic #91)
  NO TEST
  Scenario: composed counts are deduplicated across the ladder
    Given the same rule text authored at two levels of the chain
    When the inheritance summary is counted
    Then it is counted once

  # gap: dojo-authoring-surface (epic #91)
  NO TEST
  Scenario: the inheritance summary names every scope that contributes
    Given a repository in a workspace under the dōjō, with a pack adopted at the workspace
    When the summary renders
    Then it reads "this scope plus" and names each contributing scope

  # gap: dojo-authoring-surface (epic #91)
  NO TEST
  Scenario: previewing onboarding breaks the composition down per scope
    Given a workspace with contributions from three scopes
    When preview onboarding is opened
    Then each contributing scope lists its rules, skills, agents, commands, and learnings
    And the preview states that a more specific scope's rule wins on collision

  # gap: dojo-authoring-surface (epic #91)
  NO TEST
  Scenario: the constitution library is reachable from the authoring surface
    Given a maintainer starting from an empty scope
    When they follow "Browse library"
    Then they can pull proven principles, patterns, compliance controls, and stack reviewers into this scope

  # gap: dojo-authoring-surface (epic #91)
  NO TEST
  Scenario: accepting a playbook proposal makes it a rule at this scope
    Given a proposal to prefer change-flow over get-stuff-done on stable enhancements
    When the maintainer accepts it
    Then it is marked as a rule at that scope
    And it cascades to the scopes below

  # gap: dojo-authoring-surface (epic #91)
  NO TEST
  Scenario: a dismissed proposal can be undone
    Given a proposal the maintainer dismissed
    When they choose undo
    Then it returns to pending

  # gap: dojo-authoring-surface (epic #91)
  NO TEST
  Scenario: an accepted proposal leaves the pending queue
    Given a scope with three pending playbook proposals
    When one is accepted
    Then the section count drops to the two still pending

  # resolved: T-2
  NO TEST
  Scenario: only a maintainer can author on the ladder
    Given a user with contributor role in the org
    When they open the governance authoring surface
    Then they can read the composed constitution but cannot author
    The Govern group is gated at maintainer (navForOrg); the authoring surface itself is unbuilt (#91).
