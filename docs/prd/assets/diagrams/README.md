# PRD diagrams — SVG register

Self-contained SVGs, authored in **Phase 4** of the reorg
([`../../../reorg.md`](../../../reorg.md) §7). The mockups' own `.html` diagrams are
D3/data-bound or template-string SVG — none are standalone except the glyphs;
these are authored fresh under `prd/assets/diagrams/` as `dNN-<slug>.svg`.

| # | Diagram | Source | Status |
|---|---|---|---|
| D1 | The loop (capture → graph → analyze → learn → deliver → FTR) | `01-vision.md` loop | 🚧 Phase 4 |
| D2 | Journey — four segments + Dōjō | Journey Map | 🚧 Phase 4 |
| D3 | Dōjō journey — roles and stages | Dōjō Journey Map | 🚧 Phase 4 |
| D4 | Business model — free where public or personal | `01-vision.md` | 🚧 Phase 4 |
| D5 | Data model (schemas + key tables) | `../database/` / `design.dbml` | 🚧 Phase 4 |
| D6 | Metrics master-detail | reorg §9.6 | 🚧 Phase 4 |
| D7 | Metric gating funnel (all metrics → gate-eligible → sprint gates) | reorg §9.6 | 🚧 Phase 4 |
| D8 | Graph level-of-detail — L0 birds-eye | reorg §9.7 | 🚧 Phase 4 |
| D9 | Graph level-of-detail — L1 group | reorg §9.7 | 🚧 Phase 4 |
| D10 | Graph level-of-detail — L2 detail | reorg §9.7 | 🚧 Phase 4 |
| D11 | T3 planning — tracker sync + gates | reorg §9 | 🚧 Phase 4 |
| D12 | Gate vs improvement split | reorg §9.3 | 🚧 Phase 4 |
| D13 | Own-baseline ratchet | reorg §9.4 | 🚧 Phase 4 |
| D14 | Platform matrix — macOS / Linux / Windows adapter model | reorg §10 | 🚧 Phase 4 |
| D15 | Module map — 29 modules across 5 planes | `../modules/README.md` | 🚧 Phase 4 |

Rules (reorg §7.3): self-contained (no external CSS, no D3, no JS); monochrome-
with-one-accent, matching the design system; every level of the LOD diagrams is
readable at its own scale — never a 1000%-zoom render.