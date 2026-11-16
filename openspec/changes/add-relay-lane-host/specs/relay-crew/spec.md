## ADDED Requirements

### Requirement: A lane names its host editor
Register SHALL accept an optional `host` naming the editor the session runs in; the lane SHALL keep
it and every lane shape SHALL serve it; a lane registered without one has no host (never a guess).

#### Scenario: A lane in Cursor
- **GIVEN** a lane registers with `family: "claude-code"`, `model: "claude-sonnet-4-5"`, `host: "cursor"`
- **WHEN** the person opens the Crew view
- **THEN** its member card shows the tag "in Cursor", titled "Cursor is the editor; the assistant inside it does the work, and its transcripts count for claude-sonnet-4-5."

#### Scenario: A terminal lane
- **GIVEN** a lane registers with no host
- **THEN** its member card shows no host tag
