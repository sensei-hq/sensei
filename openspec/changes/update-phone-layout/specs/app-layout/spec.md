## ADDED Requirements

### Requirement: Mobile-first layout
Every app and dōjō screen SHALL lay out for phone width (430px) with unprefixed classes and adjust
upward with `sm:` and `md:` variants; components SHALL NOT use `max-*:` variants or layout
`@media`/`<style>` blocks.

#### Scenario: A screen at phone width
- **GIVEN** any main screen (Relay, observatory, a workspace page, settings) at 430px wide
- **WHEN** it renders
- **THEN** the page body does not scroll horizontally
- **AND** no primary text (titles, stat labels, state pills) is cut off; it wraps instead

#### Scenario: The same screen on desktop
- **GIVEN** the same screen at 1440px
- **THEN** its multi-column layout is as before the change

### Requirement: The sidebar is a drawer on phone
The app's sidebar (`Rail`) SHALL be hidden below `md:` and open as a drawer from the screen header,
and SHALL stay beside the content at `md:` and up.

#### Scenario: Navigating on a phone
- **GIVEN** the observatory at 430px wide
- **WHEN** the person opens the menu and picks a section
- **THEN** the drawer opens over the content, the section opens, and the drawer closes

### Requirement: Shell bands fit a phone
The screen shell's info band SHALL start collapsed on phone, its stats SHALL lay out two per row on
phone, and a stat label SHALL wrap rather than truncate.

#### Scenario: Relay on a phone
- **GIVEN** the Relay screen at 430px with the stats band
- **THEN** the body starts within the first viewport height
- **AND** "Completed today" is shown in full

### Requirement: Wide tables scroll inside themselves
A table or fixed-column grid wider than the phone SHALL scroll horizontally inside an
`overflow-x-auto` container, never the page.

#### Scenario: Activity logs on a phone
- **GIVEN** the activity log at 430px
- **THEN** the log grid scrolls sideways within its panel and the page does not

### Requirement: A guard keeps it so
A check SHALL fail when a component adds an unprefixed layout wider than a phone (three or more
grid columns, a fixed column of 300px or more, a fixed width of 300px or more) without a phone
default, counted against today's allowlist, which only shrinks.

#### Scenario: A new desktop-only grid
- **GIVEN** a component adds `grid-cols-4` with no phone default
- **WHEN** the pre-commit hook runs
- **THEN** the check fails and names the file and line
