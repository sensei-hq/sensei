## ADDED Requirements

### Requirement: Lanes named by codename on the task view
The task view SHALL name every lane by its codename, with its code beside it in the facts line
and as the title elsewhere; a lane with no codename SHALL read its code alone.

#### Scenario: The facts line
- **GIVEN** a task held by lane L1 "Diagrams" with codename cobalt-lyra
- **WHEN** the person opens the task view
- **THEN** the facts line reads "Diagrams · L1 · cobalt-lyra"

#### Scenario: History by-lines
- **GIVEN** a history entry reported by L2 with codename coral-spica
- **THEN** the entry's by-line reads "coral-spica" and its title names "L2"

#### Scenario: No codename
- **GIVEN** a lane registered without a codename and none minted
- **THEN** it reads "L3", never a placeholder name

### Requirement: Who handed a task off
The Handed off card SHALL name the lane that handed the task off and, once another lane has
taken it, that lane.

#### Scenario: Handed off and taken over
- **GIVEN** cobalt-lyra handed the task off at 10:42 and coral-spica claimed it
- **WHEN** the person opens the task view
- **THEN** the card reads "Handed off by cobalt-lyra · 10:42 → coral-spica"

#### Scenario: Handed off, not yet taken
- **GIVEN** cobalt-lyra handed the task off and no lane has claimed it
- **THEN** the card reads "Handed off by cobalt-lyra · 10:42" with no taker
