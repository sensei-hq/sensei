## ADDED Requirements

### Requirement: The journey adds a ticketed item
The run journey SHALL add one backlog item from a seeded tracker ticket by its provider and key
(the person route: no screen adds a ticket until the tracker pull), see its ticket pill on its
backlog row, and include it in the run with the manual items.

#### Scenario: A seeded Jira ticket
- **GIVEN** the e2e broker has a synced tracker holding ticket E2E-<run tag>
- **WHEN** the journey adds it to the backlog by key, picks it with the two manual items, and creates the run
- **THEN** its backlog row shows the ticket pill with its key
- **AND** it leaves the backlog as one of the run's three items
