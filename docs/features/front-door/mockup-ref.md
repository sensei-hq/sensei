---
name: front-door — mockup ref
updated: 2026-07-20
---

# front-door — mockup reference

> Optional. Link to the section of the system-wide mockup (docs/mockups/) this
> feature realizes. The mockup is one cohesive artifact — point here, don't fork it.

## Status: pending

The designer's front-door screens — intake, the playbook catalog, and the
recommendation card — are **pending**. The three-tiers/axes approach (the
`lifecycle` / `intent` / `risk` classification + recommend-and-confirm) was
handed to the designer on 2026-07-20; new screen designs are expected but not
yet produced.

Until those screens land in `docs/mockups/Sensei/lib/` and are registered in
`docs/spec/MOCKUP-INDEX.md`, there is nothing to link here. In the meantime,
the surface requirements this feature must satisfy live in:

- [`design.md`](design.md) §2 ("Two surfaces") — the CLI/agent surface and
  the app's `/intake` Observatory screen, including the five UI states
  (`describe` / `loading` / `recommended` / `recorded` / `error`).
- [`../../requirements/front-door.md`](../../requirements/front-door.md) §4
  ("Surfaces & screens") — S1–S3, the screen-by-screen brief for the
  designer (intake screen, recommendation card, playbook catalog).

**Update this file when the screens land** — replace this section with the
actual mockup links and drop the "pending" status.
