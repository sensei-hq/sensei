# PRD — Sensei: what we are building

The product requirement document for **sensei** — the *what* and the *why*,
grounded in the mockups and the existing docs. If two docs disagree, this tree
wins (precedence below); disagreements are the bug, and get filed.

> **How this tree is organised.** One module = one spec = one roadmap section =
> one `.feature` set. Modules are grouped into five **planes** (Foundation ·
> Insight · Delivery · Team · Platform). Each plane has a landing page, each
> module will get a spec, and scenarios are `.feature` files referenced from
> the specs. Status lives in [`../roadmap/`](../roadmap/), never here.

---

## One paragraph

Sensei is a **helpful observer and mentor of how a developer works with AI
coding assistants** — mentoring that surfaces as insights land. It captures the
sessions, builds a code + activity graph over the developer's real repos, and
turns the resulting signal into things a human can act on: memories that stick,
patterns worth reusing, guards worth adopting, and a clear picture of *when the
pairing worked and when it didn't*. It is not a productivity dashboard — it is a
**retrospective loop for a pair (you + your assistant) that otherwise never gets
one**. The Observatory shows *today's one thing*; the project window shows *what
this project learned*; the Dōjō (SaaS) extends the same loop across a team
without leaking client work.

Full vision and background → [`01-vision.md`](01-vision.md).
Measurable objectives per segment → [`02-objectives.md`](02-objectives.md).

**North-star metric:** **FTR** — first-turn resolution, the fraction of sessions
where the assistant's first attempt landed without a correction. Every module is
judged by one question: *does it make FTR go up, or expose the reason it went
down?*

## Precedence — when two docs disagree

| Question | Authority |
|---|---|
| What are we building, and why? | **`prd/`** (this tree) |
| Is it built? How far? What's next? | [`roadmap/`](../roadmap/README.md) |
| How is it built? | [`architecture/`](../architecture/README.md) |
| What does it look like? | [`mockups/`](../mockups/) |
| What is open right now? | [`backlog.md`](../backlog.md) (GitHub issues) |

If two disagree, the higher row wins; the lower one is the bug.

## Who this is for

The personas in [`03-personas.md`](03-personas.md) — the humans whose goals every
module's "Serves:" line must name. Newly covered this pass: **management /
delivery-lead** and **client / project stakeholder** viewpoints, for the
multi-user, multi-team case.

## The 29 modules, by plane

One module = one PRD spec (in `modules/<plane>/<module>/`) = one roadmap section
(in [`../roadmap/`](../roadmap/) = one `.feature` set. The plane pages are the
reading order; the index is [`modules/README.md`](modules/README.md).

| Plane | Modules | Plane page |
|---|---|---:|
| Foundation — the floor | M1–M6 | [`modules/foundation/`](modules/foundation/README.md) |
| Insight — observation→understanding | M7–M13 | [`modules/insight/`](modules/insight/README.md) |
| Delivery — signal back to human + assistant | M14–M22 | [`modules/delivery/`](modules/delivery/README.md) |
| Team — the Dōjō | T1–T4 | [`modules/team/`](modules/team/README.md) |
| Platform — distribution, web, CI | P1–P3 | [`modules/platform/`](modules/platform/README.md) |

## What is deliberately *not* here

- **Status and gaps** — that is [`../roadmap/`](../roadmap/README.md). A module
  spec describes the target; the roadmap says whether it exists yet.
- **Implementation** — that is [`../architecture/`](../architecture/README.md).
- **Visual ground truth** — that is [`../mockups/`](../mockups/), untouched.
- **Design system and cross-cutting UX** — [`../design/`](../design/README.md).

## Read next

1. [`01-vision.md`](01-vision.md) — the why, the loop, the business model, the six themes.
2. [`02-objectives.md`](02-objectives.md) — the measurable WHAT per segment.
3. [`03-personas.md`](03-personas.md) — who each module serves.
4. [`modules/README.md`](modules/README.md) — the module index + spec template.

## Status of this tree

| Piece | State |
|---|---|
| `prd/README.md` (this file) | ✅ living front door |
| `01-vision.md`, `02-objectives.md` | ✅ moved in verbatim (2026-10-13), links re-based |
| `03-personas.md` | ✅ authored (2026-10-13) |
| `modules/<plane>/README.md` | ✅ plane charters + module lists |
| `modules/<module>/` specs | 🚧 Phase 2 — written in priority order |
| `scenarios/*.feature` | 🚧 with each module spec |
| `assets/diagrams/*.svg` | 🚧 Phase 4 — register in [`assets/diagrams/README.md`](assets/diagrams/README.md) |

Migration authority: [`../reorg.md`](../reorg.md).