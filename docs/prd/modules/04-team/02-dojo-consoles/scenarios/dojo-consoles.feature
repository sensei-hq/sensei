Feature: Dōjō Consoles — the built personal screens

  # pinned: a_bound_membership_presents_its_persona_session
  Scenario: a bound membership presents its persona session
    Given a membership bound to a persona
    When the membership is resolved
    Then the persona session is presented

  # pinned: an_unbound_membership_presents_nothing
  Scenario: an unbound membership presents nothing
    Given an unbound membership
    When it is resolved
    Then it presents nothing

  # pinned: a_bound_membership_whose_persona_signed_out_presents_nothing
  Scenario: a bound membership whose persona signed out presents nothing
    Given a membership bound to a persona that is signed out
    When it is resolved
    Then it presents nothing

  # pinned: the_tenant_base_url_is_two_real_path_segments_under_the_registry
  Scenario: the tenant base URL is two real path segments under the registry
    Given a tenant key
    When the base URL is derived
    Then it is two real path segments under the registry

  # pinned: base_url_trims_trailing_slash
  Scenario: the base URL trims trailing slash
    Given a base URL with a trailing slash
    When it is normalized
    Then the trailing slash is trimmed

  # pinned: artifacts_url_is_the_workers_two_segment_tenant_route
  Scenario: the artifacts URL is the Worker's two-segment tenant route
    Given a tenant key
    When the artifacts URL is built
    Then it is the Worker's two-segment tenant route

  # pinned: encode_path_segment_encodes_reserved_but_not_unreserved
  Scenario: encode path segment encodes reserved but not unreserved characters
    Given a path segment with reserved and unreserved characters
    When it is encoded
    Then reserved are encoded and unreserved are not

  # pinned: client_error_retryability
  Scenario: client errors are classified for retryability
    Given a client error
    When its retryability is checked
    Then it is classified correctly

  # pinned: relay_url_encodes_tenant_as_path_segments
  Scenario: the relay URL encodes the tenant as path segments
    Given a tenant key and a relay suffix
    When the relay URL is built
    Then the tenant is encoded as path segments

  NO TEST
  Scenario: the personal landing renders the work-first view
    Given a signed-in user (with or without memberships)
    When the landing renders
    Then "your work" (workspaces, needs-you, relay) is primary and "My Dōjōs" is secondary

  NO TEST
  Scenario: the personal Relay screen shows live runs
    Given a user with active runs
    When they open /you/relay
    Then live runs are shown with status, progress, and gates

  NO TEST
  Scenario: the personal Library shows rule packs
    Given rule packs exist
    When the user opens /library
    Then rule packs are shown with level and non-negotiable badge

  NO TEST
  Scenario: the personal Preview shows a workspace's effective constitution
    Given a workspace with a resolved constitution
    When the user opens /preview
    Then it shows which ladder levels contributed, conflicts settled, and locks

  NO TEST
  Scenario: the Me screen shows the user's own rules from the library
    Given a user with personal rules
    When they open /me
    Then their personal constitution is shown

  NO TEST
  Scenario: the org-switcher popover shows Relay · you — all Dōjōs + memberships + Create/Join
    Given a user with multiple memberships
    When they open the org-switcher
    Then it shows the pinned Relay row, one row per membership, Your Dōjōs, and Create/Join