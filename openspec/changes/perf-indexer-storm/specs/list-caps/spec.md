## ADDED Requirements

### Requirement: Operations lists render capped with honest counts

The alerts, failures, queue, and history lists SHALL render at most 200
rows each; beyond that an overflow row SHALL state "showing 200 of N".
Counts and totals always reflect the full set, never the rendered
window.

#### Scenario: A 240-alert storm stays readable

- **WHEN** 240 alerts stand and the queue holds 30k tasks
- **THEN** each list shows its first 200 rows plus its overflow row, the
  page stays interactive, and the counts read 240 / 30k, not 200
