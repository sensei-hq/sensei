Feature: Workspace viewer — a granted, read-only window on one workspace's outcomes

  # pinned: database/tests/dojo/workspace_viewers.sql
  Scenario: someone is in the dōjō before they are granted a workspace
    Given a person who is neither a member of the dōjō nor invited into it as a viewer
    When an admin grants them a workspace
    Then the grant is refused with "invite them to this dōjō as a viewer first"

  # pinned: database/tests/dojo/workspace_viewers.sql
  Scenario: a viewer invited into the dōjō is not a member and reads no repository
    Given a person who accepted a viewer invite
    And an admin granted them one workspace
    Then they read that workspace and no other
    And they are not a member of the dōjō
    And they read none of its repositories

  # pinned: dojo/src/lib/server/viewers-data.spec.ts
  Scenario: only an admin grants or revokes
    Given a maintainer of the dōjō
    When they grant or revoke a workspace view
    Then it is refused

  # pinned: database/tests/dojo/teams_groupings.sql
  Scenario: a grant to a team is a grant to the people on it
    Given a team of two members
    When an admin grants the team a workspace
    Then both people hold the view
    And deleting the team leaves both grants in place

  # gap: T6 viewer shell
  NO TEST
  Scenario: a viewer with one grant lands on that workspace; with two, on a home listing both
    Given a viewer granted one workspace
    When they sign in
    Then they land on that workspace's overview
    This is unbuilt — the viewer lands on their workspaces list in the dōjō shell.

  # gap: T6 viewer reads
  NO TEST
  Scenario: no viewer response carries a person, a path, a branch or a ticket key
    Given a viewer's workspace overview
    Then no field names a person, a file path, a branch or a ticket key
    This is unbuilt.

  # gap: T6 viewer reads
  NO TEST
  Scenario: an unmeasured key says so, and a thin one is flagged
    Given a workspace with no CI connection
    Then every DORA key reads "not measured"
    And a key over fewer than 5 deploys reads "low sample"
    This is unbuilt — T5 has no deployments yet.

  # gap: T6 viewer reads
  NO TEST
  Scenario: a partial index is not evaluated
    Given a repository at 62% indexed
    Then coverage and complexity read "not measured" and do not count toward the headline
    This is unbuilt.

  # gap: T6 export
  NO TEST
  Scenario: someone outside the organization cannot export
    Given a viewer invited as a client
    Then the overview offers no export
    This is unbuilt.
