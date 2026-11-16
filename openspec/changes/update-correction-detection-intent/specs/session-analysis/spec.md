## ADDED Requirements

### Requirement: Corrections by intent
A user turn that follows an assistant turn SHALL be labelled `correction`, `principle` or `neither`
by the intent classifier when a model is available; the per-turn correction flag, first-try-right
and session outcomes SHALL read that label.

#### Scenario: A correction with no key phrase
- **GIVEN** the user turn "Hmm, I meant the other config file" after an assistant edit
- **THEN** it is labelled `correction` and the session is not first-try-right

#### Scenario: A request that contains a key phrase
- **GIVEN** the user turn "add a revert button to the toolbar"
- **THEN** it is labelled `neither`

### Requirement: The label's source is recorded
Each label SHALL record its source: `intent` (the model) or `phrase` (the fallback when no model was
available); metrics SHALL be able to tell them apart.

#### Scenario: No model available
- **GIVEN** the local model is unavailable during analysis
- **THEN** turns are labelled by the phrase list with source `phrase`
- **AND** a later pass with the model relabels them with source `intent`

### Requirement: Measured accuracy
A labelled set of real prompts SHALL be a test of the classifier's precision and recall, with floors
the change sets and the test enforces.

#### Scenario: The labelled set
- **GIVEN** the labelled prompts (including the issue's missed and false-positive examples)
- **WHEN** the classifier runs on them
- **THEN** precision and recall meet their floors
