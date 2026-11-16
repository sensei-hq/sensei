## ADDED Requirements

### Requirement: The journey adds a ticketed item
The run journey SHALL add one backlog item from a seeded tracker ticket by its provider and key,
see its ticket pill in triage, move it to the backlog with +, and include it in the run.

#### Scenario: A seeded Jira ticket
- **GIVEN** the e2e broker has a synced tracker holding ticket SEN-1
- **WHEN** the journey adds SEN-1 to the backlog and creates the run
- **THEN** the triage row shows the ticket pill "Jira SEN-1"
- **AND** the run's planned items include SEN-1
