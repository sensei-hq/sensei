## ADDED Requirements

### Requirement: Uncleared drops redeliver until written

Clears SHALL persist in a pending set until the daemon confirms them:
every refresh SHALL re-attempt every pending id, and a slow PUT SHALL
NOT starve the ids behind it (per-PUT timeout unblocks the flush while
the attempt stays in flight). A clear lost to one failed write is a
lost recurrence memory — redelivery makes the write eventual.

#### Scenario: A clear during an outage lands after recovery

- **WHEN** a condition clears while PUTs fail, then the daemon recovers
- **THEN** a later refresh writes the clear without further user action
  and `seen_clears` increments exactly for the cleared condition

### Requirement: A newer ack wins over a stale clear

`ack()` SHALL cancel its id's pending clear, and a late response from a
superseded PUT SHALL be dropped by generation: a clear that lands after
a re-ack MUST NOT clobber `acked` back to false.

#### Scenario: Ack beats a hung clear

- **WHEN** a clear PUT hangs, the condition recurs, the user acks, and
  then the hung PUT resolves
- **THEN** the disposition stays acked and no spurious clear records
