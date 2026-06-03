# Sensei Observatory · UI kit

The desktop app — a Tauri window with a thin status chrome, a left rail of projects and sections, and a main view that updates per-section.

This kit recreates the daily "morning briefing" home view as it appears in the source prototype (`sensei_src/lib/observatory.jsx`), rebuilt against the rationalized Zen-Sumi system: the type scale has 8 sizes (not 30), spacing snaps to 9 stops, and every color flows from semantic CSS variables.

## Files

- `index.html` — interactive observatory (run it)
- `Chrome.jsx` — `<Chrome/>`: 38px Tauri chrome with traffic lights + centered title
- `Sidebar.jsx` — `<Sidebar/>`: nav sections + active projects + dormant projects
- `Hero.jsx` — `<HeroKoan/>`: the signature daily teaching block — kanji + koan + body + action
- `FtrPanel.jsx` — `<FtrPanel/>`: the 14-day bar strip with current rate
- `InsightRow.jsx` — `<InsightRow/>`: secondary observations
- `SessionRow.jsx` — `<SessionRow/>`: recent-sessions list row
- `LearnedRow.jsx` — `<LearnedRow/>`: "system has learned" adopted-teachings row

## Components used from the design system

Pure CSS classes from `colors_and_type.css`:
- `.zs`, `.zs-h1`, `.zs-h2`, `.zs-eyebrow`, `.zs-kanji`, `.zs-mono`, `.zs-body`, `.zs-body-sm`
- `.zs-card`, `.zs-btn-primary`, `.zs-btn-secondary`, `.zs-badge*`, `.zs-dot*`, `.zs-chrome`, `.zs-input`
- Utility classes for layout (flex, gap, padding, etc.)

No raw font sizes, no raw colors, no raw spacing.
