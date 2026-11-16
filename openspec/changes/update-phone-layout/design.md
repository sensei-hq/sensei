## Context
Survey 2026-11-16 (~40 findings). Root causes are shared (Rail 220px; rokkit ScreenShell
desktop-first). dōjō is already mobile-first (NavPane drawer, TabBar). Relay components are mostly
mobile-first.

## Decisions
- Fix in rokkit what is rokkit's (ScreenShell, StatCard), per the one-shell ruling.
- The Rail drawer follows the dōjō NavPane pattern (fixed + translate below md:, static at md:).
- A guard with a shrinking allowlist lands first, so the work cannot regress while in flight.
- Phone = 430px design width; breakpoints are UnoCSS defaults (`sm` 640, `md` 768).

## Open Questions
- Q1: Is the phone target the desktop app (Tauri window resized) and the dōjō in a phone browser,
  or also a future mobile shell? (Affects whether the Rail drawer is enough.)
- Q2: Order: shells and Relay first, then the rest screen by screen — or all in one change?
