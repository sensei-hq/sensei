Feature: MCP work delivery — the ticket fetch, targets at start of work, and the missed-target handoff

  # gap: work-delivery-tools
  NO TEST
  Scenario: the assistant fetches a ticket by key and the fetch marks the start of work
    Given a synced tracker with the ticket AUTH-412
    When the assistant calls get_ticket with key AUTH-412 in session S
    Then it receives the key, title, description, acceptance criteria, status, sprint and assignee
    And the response is the same shape for Jira, ADO and GitHub Issues
    And session S is linked to AUTH-412 and the start of work is recorded (D63)
    Designed, not built; the tool name is a proposal.

  # gap: work-delivery-tools
  NO TEST
  Scenario: a second fetch in the same session records nothing new
    Given session S already started work on AUTH-412
    When the assistant calls get_ticket with key AUTH-412 again
    Then the ticket is returned and no second start of work is recorded

  # gap: work-delivery-tools
  NO TEST
  Scenario: an unknown key is not found
    Given no synced ticket has the key AUTH-9999
    When the assistant calls get_ticket with key AUTH-9999
    Then the call fails with not-found and no session link is written

  # gap: work-delivery-tools
  NO TEST
  Scenario: the ticket's rules and targets arrive with it
    Given AUTH-412 is a migration task in a repository with a Repository-rung coverage target
    When the assistant calls get_ticket with key AUTH-412
    Then the response carries the ticket's rules and its resolved targets (D62)
    And each target names its threshold, its mode and the ladder rung it came from (D48)

  # gap: work-delivery-tools
  NO TEST
  Scenario: the assistant asks which targets a change missed and what to fix
    Given a change that drops coverage on auth/** below a blocking target
    When the assistant calls get_missed_targets
    Then the target is returned as "target missed (blocking)" with its reading and threshold
    And it names what to fix: the files behind the miss (D61)

  # gap: work-delivery-tools
  NO TEST
  Scenario: a target with no reading is not measured, never passed
    Given a repository still indexing
    When the assistant calls get_missed_targets
    Then its index-derived targets read "not measured" and none is reported as met

  # gap: work-delivery-tools
  NO TEST
  Scenario: key matching stays the fallback
    Given an agent that read AUTH-412 with gh issue view and never called get_ticket
    When its branch carries AUTH-412
    Then the session is linked to AUTH-412 by key, as today
