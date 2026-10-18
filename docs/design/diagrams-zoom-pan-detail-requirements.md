# Diagrams / Atlas — Zoom, Pan & Detail Requirements

> **Status:** Requirements gathering — feeds indexer changes + UI implementation
> **Scope:** Atlas canvas, World diagram, Structure diagram, and any rokkit `Graph` surfaces
> **Source analysis:** `app/src/routes/(project)/project/[id]/atlas/*`,
> `app/src/routes/(project)/project/[id]/diagrams/*`,
> `app/node_modules/@rokkit/graph/dist/*`

---

## Problem Statement

**Current state (2026-10-13):**

| Surface | Zoom mechanism | Range | Controls | Detail levels | Pan |
|---------|----------------|-------|----------|---------------|-----|
| **Atlas Canvas** | d3-zoom (ctrl+wheel) | 0.25×–5× (20×) | ❌ None | ❌ None | Drag bg |
| **World / Structure / Cycles / Layers** (rokkit `Graph`) | ctrl+wheel (pinch) | 0.25×–4× (16×) | ❌ None | 4 (full/compact/minimal/dot) | Drag bg |
| **Atlas auto-collapse** | — | >500 nodes → communities | — | 2 levels only | — |

**Pain points reported:**
1. **Scroll-bar-based zoom is "very sensitive"** — d3-zoom wheel zoom reacts to tiny deltas; no smoothing/debounce
3. **No controlled zooming** — no zoom in/out/reset buttons, no presets (fit, 100%, 200%)
4. **No zoom level indicator** — user doesn't know current scale
5. **No controlled focus** — can't programmatically focus a node/community from the sidebar
6. **Detail levels exist but aren't controllable** — rokkit has 4 tiers (full/compact/minimal/dot) but no UI to lock/set them
7. **Indexer has no depth-scope** — queries return full graph; no way to ask for "only L1 modules" or "only top-50 symbols"
8. **World diagram** — `focusPath` works for drill but no breadcrumb UI, no "copy link to this view"

---

## Solution Approaches

### A. Controlled Zoom & Pan (UI Layer)

#### A1. Zoom Toolbar (applies to all Graph surfaces)
```
┌─────────────────────────────────────────────────────────────┐
│  [ - ]  [ 67% ]  [ + ]    [ ⛶ Fit ]  [ 1:1 ]  [ 2× ]  [ 4× ]  │
└─────────────────────────────────────────────────────────────┘
```
- **In/Out buttons** — call `nextZoom('in'/'out')` (rokkit) / `behavior.scaleBy()` (d3)
- **Percentage display** — shows effective scale (rokkit: `scale` derived; d3: `transform.k`)
- **Preset buttons** — Fit (rokkit: `zoom=1` / d3: `behavior.scaleTo(svg, fit)`), 1:1 (`zoom=1`), 2×, 4×
- **Keyboard shortcuts** — `=` / `-` / `0` (reset) / `f` (fit)

#### A2. Wheel Zoom Smoothing
- **Current:** d3-zoom fires on every wheel notch; rokkit uses `ZOOM_STEP=1.25` per notch
- **Fix:** Debounce wheel events (50ms) + accumulate delta → single `nextZoom` call per frame
- **Config:** `wheelZoomSensitivity: 0.5` (user-adjustable in settings)

#### A3. Anchor-Aware Zoom (already in rokkit)
- rokkit's `anchoredScroll` keeps pointer content stationary during pinch
- **Atlas Canvas:** adopt same `anchoredScroll` logic for d3-zoom

#### A4. Programmatic Focus/Zoom API
```typescript
// New methods on GraphState / AtlasView
graph.zoomTo(scale: number, anchor?: {x, y})    // absolute scale
graph.zoomBy(factor: number, anchor?: {x, y})   // relative
graph.zoomToFit(pad?: number)                   // fit with padding
graph.zoomToNode(nodeId: string, scale?: number) // center on node
graph.panBy(dx: number, dy: number)             // programmatic pan
graph.resetView()                               // zoom=1, pan=0
```
- Exposed via `graph` context for sidebar controls, deep links, keyboard shortcuts

---

### B. Detail Levels (LOD) — Control & Lock

#### B1. Explicit Detail Tiers (rokkit already has these)
| Tier | Scale range | What renders | Use case |
|------|-------------|--------------|----------|
| `full` | ≥ 0.6 | Labels, icons, rows, badges, edge arrows | Inspection |
| `compact` | 0.32–0.6 | Labels + icons, no rows, edge dots | Overview |
| `minimal` | 0.14–0.32 | Icons only, no labels, edge lines | Bird's-eye |
| `dot` | < 0.14 | Dots only, no labels, no edges | Map |

#### B2. Detail Lock Control
```
┌─────────────────────────────────────────────────────────────┐
│ Detail: [ Auto ▼ ]  [ Full ] [ Compact ] [ Minimal ] [ Dot ]  │
└─────────────────────────────────────────────────────────────┘
```
- **Auto** — derives from effective scale (current behavior)
- **Locked tiers** — forces rendering tier regardless of scale
- **Persisted** per diagram type in localStorage

#### B3. Atlas L0/L1/L2 Mapping
| Atlas Level | Maps to | rokkit tier |
|-------------|---------|-------------|
| L0 (communities) | `communities` level | N/A (separate view) |
| L1 (modules/files) | `symbols` capped | `compact` → `minimal` |
| L2 (symbols) | `symbols` full | `full` |

---

### C. Depth/Detail Scoping (Indexer + API Layer)

#### C1. Graph Query Depth Parameter
```typescript
// New query params for /api/graph/nodes, /api/graph/communities, /api/projects/{id}/diagrams/*
interface GraphQueryParams {
  depth?: 'L0' | 'L1' | 'L2' | 'full';  // L0=communities, L1=modules, L2=symbols
  cap?: number;                          // max nodes (default: SYMBOL_CAP=200)
  kinds?: string[];                      // filter by node kind
  scope?: 'communities' | 'symbols';     // which view
  focus?: string;                        // node id to center
  radius?: number;                       // neighbourhood radius (hops)
}
```

#### C2. Indexer Support for Depth-Scoped Queries
| Depth | What indexer returns | Implementation |
|-------|---------------------|----------------|
| `L0` | `communities/info` + aggregate stats | Already exists (`GET /api/graph/communities/info`) |
| `L1` | Module/file nodes + intra-module edges | New: aggregate `nodes` by `module` segment of FQN |
| `L2` | Top-N symbols by degree + internal edges | Already exists (`buildSymbolGraph` with cap) |
| `full` | All nodes + all edges | Current default |

**Indexer changes needed:**
1. **Module aggregation view** — materialized view `sensei.module_graph` (module → symbols, intra-module edges)
2. **Neighbourhood index** — precomputed k-hop neighbourhoods for `radius` queries
3. **Depth-aware watermark** — separate cursors per depth so L0 refresh doesn't invalidate L1 cache

#### C3. API Endpoints (new)
```
GET /api/graph/nodes?depth=L1&cap=100
GET /api/graph/nodes?depth=L2&cap=200&focus=symbol-123&radius=2
GET /api/graph/communities?depth=L0
GET /api/projects/{id}/diagrams/structure?depth=L1&groupBy=module
GET /api/projects/{id}/diagrams/world?depth=L0&groupBy=project
```

---

### D. World / Structure Diagram Enhancements

#### D1. Breadcrumb + Deep Link
```
Home ▸ sensei ▸ crates ▸ senseid ▸ src ▸ tasks
```
- **Component:** `GraphBreadcrumb` using `focusPath`
- **Deep link:** `?focusPath=project,crates,senseid,src,tasks` → restores `focusPath` on load
- **Copy link button** — copies current view URL with all params (groupBy, shadeBy, focusPath, zoom)

#### D2. GroupBy/ShadeBy Presets
```typescript
const WORLD_PRESETS = [
  { id: 'architecture', label: 'Architecture', groupBy: 'project', shadeBy: 'unresolvedShare' },
  { id: 'ownership', label: 'Ownership', groupBy: 'repository', shadeBy: 'nothing' },
  { id: 'quality', label: 'Quality', groupBy: 'kind', shadeBy: 'testShare' },
  { id: 'docs', label: 'Documentation', groupBy: 'project', shadeBy: 'documentedShare' },
];
```

---

### E. Atlas Canvas Specific Fixes

| Issue | Fix |
|-------|-----|
| Wheel zoom too sensitive | Debounce (50ms) + `ZOOM_STEP=1.15` (was implicit) |
| No zoom buttons | Add `AtlasZoomToolbar` component (reuse rokkit pattern) |
| No zoom indicator | Show `%` in toolbar; `transform.k * 100` |
| No reset/fit/presets | Add `zoomToFit()`, `zoomTo(1)`, `zoomTo(2)` |
| No detail tiers | Derive `detail` from effective scale (like rokkit) |
| Pan/zoom conflict | `pointerdown` on bg → pan; `ctrl+wheel` → zoom; click node → select |
| Auto-collapse threshold | Expose `COLLAPSE_THRESHOLD` in toolbar as "Auto-collapse at N nodes" |

---

### F. Deep Link & State Persistence

#### F1. URL State Sync
```
?view=atlas&level=symbols&zoom=1.5&focus=symbol-42&pan=x,y
?view=world&groupBy=project&shadeBy=unresolvedShare&focusPath=project,crates,senseid
?view=structure&depth=L1&groupBy=module
?view=layers&focusPath=module,senseid,graph
```

#### F2. Persisted Preferences
```typescript
interface DiagramPrefs {
  zoom: { sensitivity: 0.5; defaultLevel: 'auto' | 'full' | 'compact' ... };
  atlas: { collapseThreshold: 500; defaultView: 'communities' | 'symbols' };
  world: { defaultGroupBy: 'project'; defaultShadeBy: 'unresolvedShare' };
}
```

---

## Indexer Changes Required (Summary)

| # | Change | Module | Effort | Blocks |
|---|--------|--------|--------|--------|
| 1 | `module_graph` materialized view | M4 Indexer | M | L1 queries, module-level metrics |
| 2 | `neighbourhood_index` (k-hop) | M4 Indexer | L | L2 radius queries, impact analysis |
| 3 | `depth` param on graph endpoints | M14/M17 API | S | All depth-scoped UIs |
| 4 | `focus` + `radius` params | M14/M17 API | S | Neighbourhood view, deep links |
| 5 | Separate watermarks per depth | M4 Scheduler | S | Independent L0/L1 refresh |

---

## Implementation Phases

### Phase 1 — Controlled Zoom (UI only, no indexer changes)
- [ ] ZoomToolbar component (shared)
- [ ] Wheel debounce + sensitivity setting
- [ ] Zoom in/out/reset/fit buttons
- [ ] Zoom % display
- [ ] Keyboard shortcuts
- [ ] Apply to AtlasCanvas + rokkit Graph (via `graph.zoomBy()`)

### Phase 2 — Detail Levels & Lock
- [ ] Expose `detail` tier in rokkit Graph (already derived)
- [ ] DetailLock dropdown (Auto/Full/Compact/Minimal/Dot)
- [ ] Persist per-diagram in localStorage
- [ ] Atlas: map L0/L1/L2 to detail tiers

### Phase 3 — Depth Scoping (Indexer + API)
- [ ] `depth` param on `/api/graph/nodes` + `/api/graph/communities`
- [ ] `module_graph` view for L1
- [ ] `focus` + `radius` params for neighbourhood
- [ ] Update Atlas loader to request `depth=L1` by default

### Phase 4 — World/Structure Enhancements
- [ ] Breadcrumb component for World/Structure
- [ ] Deep link copy button
- [ ] Preset selector for groupBy/shadeBy
- [ ] Copy view link button

### Phase 5 — Atlas-Specific
- [ ] ZoomToolbar on Atlas page
- [ ] Zoom % display
- [ ] Fit/1:1/2×/4× presets
- [ ] Wheel sensitivity setting
- [ ] Auto-collapse threshold control

---

## Acceptance Criteria

| Scenario | Pass condition |
|----------|----------------|
| User clicks "+" | Zoom increases by one step (1.25×), anchor at viewport center |
| User presses "=" | Zoom in one step |
| User presses "0" | Zoom resets to 1.0, pan resets |
| User presses "f" | Zoom to fit with 28px padding |
| User locks "Compact" | Labels+icons render even at 0.8× scale; no rows |
| User requests `depth=L1` | API returns ≤500 module nodes, no symbols |
| User copies World link | URL contains `groupBy`, `shadeBy`, `focusPath`, `zoom` |
| User pastes link | View restores exact groupBy, shadeBy, focusPath, zoom, pan |
| Wheel zoom on Atlas | Single notch → one zoom step (no multi-step jumps) |
| Pinch zoom on trackpad | Smooth, anchored at pointer, no jump |

---

## Related Documents

- `docs/design/transcript-adapter-pattern.md` — adapter pattern (not directly related)
- `docs/prd/modules/03-delivery/17-atlas/README.md` — Atlas module spec
- `docs/prd/modules/01-foundation/04-code-graph/README.md` — Indexer spec
- `docs/roadmap/06-gaps.md` — G-1 (L0/L1/L2), F-4 (resolution)
- `app/node_modules/@rokkit/graph/dist/*` — rokkit Graph implementation reference