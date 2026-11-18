## ADDED Requirements

### Requirement: The shell owns its height
`ShellScreen` SHALL fill its container (`h-full min-h-0`) so its body is the only scroller, and no
Relay screen SHALL add its own height wrapper for it.

#### Scenario: Relay pages scroll
- **GIVEN** the Relay Now view or a task page with more content than the window
- **WHEN** the user scrolls
- **THEN** the shell body scrolls and the header stays in place
- **AND** `ShellScreen.spec.svelte.ts` pins the height classes

### Requirement: Relay keeps the whole shell in every state
In both windows (`/relay`, `/workspace/<id>/relay`) the Relay screen SHALL draw through
`RelayShell`: the mark, title "Relay" and a context line, the info band per state, and the
resource footer. On a failed read only the info toggle remains in the toolbar and a Notice with
Retry shows. With no crew, Crew is the landing view and the info band is held open. When loaded,
the toolbar shows Now, Queue, Crew (with count badges), Message the crew and Info; the Now view
shows the four stats; the main window shows the schema filters.

#### Scenario: Error state
- **GIVEN** the relay read fails against the real e2e daemon
- **WHEN** either window opens Relay
- **THEN** the header, the whole info band, only the info toggle and the Notice are drawn, and Retry recovers once the read works
- **AND** `app/e2e/tests/relay-conformance.spec.ts` "error" passes

#### Scenario: No crew state
- **GIVEN** a workspace no crew works in
- **WHEN** its Relay page opens in the workspace window
- **THEN** Crew is the landing view, the info band is held open, and the paste lines show
- **AND** `relay-conformance.spec.ts` "no crew" passes

#### Scenario: Loaded state
- **GIVEN** a crew minted on the e2e broker
- **WHEN** either window opens Relay
- **THEN** the toolbar views, stats and (main window) filters are drawn
- **AND** `relay-conformance.spec.ts` "loaded" passes

#### Scenario: Loading state
- **GIVEN** the relay read has not landed
- **WHEN** the shell is built
- **THEN** only the info toggle is in the toolbar and the stats band reads "still reading"
- **AND** `relay-shell.spec.ts` pins it

### Requirement: Unread numbers read "not reported"
A stat or footer value the daemon did not report SHALL read "not reported", never 0.

#### Scenario: Failed read footer
- **GIVEN** the relay read failed
- **WHEN** the footer is drawn
- **THEN** build, disk and swap read "not reported"
