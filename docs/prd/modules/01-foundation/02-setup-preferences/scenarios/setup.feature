Feature: Setup & Preferences — the free-navigation settings surface and the one first-run gate

  # pinned: collective_preferences_defaults_and_upsert_roundtrip
  Scenario: assisted preferences start from defaults and round-trip an upsert
    Given a fresh preferences store
    When a team default is upserted
    Then the value round-trips and the store still serves defaults for untouched keys

  # pinned: collective_preferences_put_then_get_roundtrip
  Scenario: a saved preference is returned intact
    Given a preference was written
    When it is read back
    Then the stored value equals the written value

  # pinned: collective_preferences_put_rejects_invalid_destination
  Scenario: an invalid preference destination is rejected
    Given an unknown or malformed destination
    When a preference is put to it
    Then the write is rejected without corrupting the store

  # pinned: assistants_health_endpoint_returns_status_and_adapters
  Scenario: the Assistants pane lists every installed family with its health
    Given one or more assistant adapters are registered
    When GET /api/assistants/health is called
    Then each family returns a status and adapter detail

  Scenario: every preference section is reachable
    Given the app shell is open at /preferences
    Then GET /api/preferences exposes assistants, assignments, folders, inference, instruments, libraries, profile, projects

  Scenario: editing a value applies live with no Save/Cancel
    Given a preference pane is open
    When a value is edited
    Then the change is written immediately
    And an override exposes a one-click restore-default

  Scenario: search finds a value by its label or its description
    Given the global search field at /preferences
    When a term matching a value's label or description is typed
    Then the matching row is shown and deep-links to its pane row

  Scenario: the first-run gate requires at least one validated root
    Given the first-run scan screen
    When no root has been added or validated
    Then the Begin action is disabled
    When a paste-path root that does not exist is submitted
    Then the root is rejected with a validation error

  Scenario: the kanji ⇄ solar icon-style preference persists
    Given the silhouette toggle from decision 10
    When the icon style is switched
    Then the choice persists across restart

  # gap: M2
  Scenario: review cues surface tuning suggestions with a deep link
    Given sensei has data suggesting a tuning (e.g. an assistant family without a default persona)
    When the cues endpoint is polled
    Then each cue links to the specific row it concerns