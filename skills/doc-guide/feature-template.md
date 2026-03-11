---
id: <module-name>
type: feature
---

# <Module Name>

[One paragraph: what user need or pain point this module addresses, and why it matters. Human audience. No implementation details.]

## Features

### <Feature Name>

[One sentence describing what this feature does from the user's perspective.]

```gherkin
Feature: <Feature Name>

  Scenario: <Happy path — primary use case>
    Given <precondition>
    When <action>
    Then <observable outcome>
    And <secondary outcome if needed>

  Scenario: <Edge case or error case>
    Given <precondition>
    When <action>
    Then <outcome>
```

### <Feature Name>

[One sentence.]

```gherkin
Feature: <Feature Name>

  Scenario: <Description>
    Given <precondition>
    When <action>
    Then <outcome>
```

---

> This is a **feature doc** — what and why, not how.
> Implementation details belong in `docs/design/`.
> Status lives in `docs/traceability.yaml` — do not add a status table here.
