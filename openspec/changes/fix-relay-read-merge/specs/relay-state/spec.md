## ADDED Requirements

### Requirement: A read never undoes newer stream rows
When a Now or queue read lands, the state SHALL keep every row the stream applied after that read
started (adds and updates), and SHALL keep removed every row the stream removed after it started.

#### Scenario: A lane registers during a read
- **GIVEN** a Now read is out with two lanes in its snapshot
- **WHEN** a third lane's add arrives on the stream, then the read lands
- **THEN** the three lanes are shown at every moment after the add — no blink

#### Scenario: A lane leaves during a read
- **GIVEN** a Now read is out whose snapshot still has lane L2
- **WHEN** L2's remove arrives on the stream, then the read lands
- **THEN** L2 is not shown
