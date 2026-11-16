## ADDED Requirements

### Requirement: A refusal carries the daemon's reason
The app's API client SHALL report a refused call's `message` when the daemon sends one, else its
`error`, else the HTTP status text — the same rule as the relay client, from one shared helper.

#### Scenario: A coded refusal with a reason
- **GIVEN** a route answers 409 `{ "error": "conflict", "message": "the board is connected to another project" }`
- **THEN** the person reads "the board is connected to another project"

#### Scenario: A reason in error alone
- **GIVEN** a route answers 400 `{ "error": "name is required" }`
- **THEN** the person reads "name is required"
