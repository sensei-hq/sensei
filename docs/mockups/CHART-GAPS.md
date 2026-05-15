# Chart feature gaps

> Running list of capabilities the Sensei UI needs from `@rokkit/chart` that aren't supported yet.
> When you spot one while implementing a chart, add a row here. Each row should make the **need**, **the workaround we're using right now**, and **the proposed upstream change** clear enough for a Rokkit PR.

For each gap, prefer this order:

1. **Compose locally with `PlotChart`** — its declarative geom API (Bar / Line / Area / Point / Arc / Box / Violin / Heatmap / Candlestick / Waterfall / Hexbin / Ribbon) covers most cases without forking.
2. **Raise upstream** — open an issue or PR on `@rokkit/chart` so the feature lands once.
3. **Never** roll a parallel chart implementation inside the app.

---

## Gaps

| # | Chart / route | Need | Current workaround | Upstream proposal |
|---|---|---|---|---|
| 1 | `Sparkline` (observatory home, project overview) | Stroke should resolve to zen-sumi's `--color-primary-z*` (vermillion shu) palette. Currently `Sparkline.svelte` derives stroke from `rgb(var(--color-primary-500, 100,116,139))` and fill from `rgba(var(--color-primary-300), 0.25)`. zen-sumi only defines `--color-primary-z0..z9` as OKLCH triplets — there is no `--color-primary-500`, and the format is wrong for `rgb()`. Result today: stroke falls back to slate-gray. | Blocked on gap #3 (can't adopt until package builds). Step 10 (`MiniStat`) will revisit once gaps #1+#3 are upstream. | Standardise `@rokkit/chart` on zen-sumi's `--color-*-z*` OKLCH scale. Same pattern hits `Plot.svelte` (`--color-surface-700`), `FilterHistogram` (`--color-primary`), etc. |
| 2 | `Sparkline` (observatory home) | The previous hand-rolled SVG drew a 2.5 px circle at the last data point as a "you-are-here" marker. Rokkit's `Sparkline` doesn't expose a last-point indicator. | Hand-rolled SVG kept for now (see gap #3). | Add a `lastPoint?: boolean \| { radius?: number; color?: string }` prop to `Sparkline`. |
| 3 | `@rokkit/chart@1.0.5` package (blocker) | `src/lib/brewing/colors.js` and `src/lib/plot/preset.js` both `import masterPalette from '../palette.json'`, but `palette.json` is **not published** in the npm tarball. Any `import { Sparkline } from '@rokkit/chart'` (which barrels through `index.js → PlotState.svelte.js → brewing/colors.js`) fails the production Vite/Rolldown build with `[UNRESOLVED_IMPORT] '../palette.json'`. `bun run check` and `vitest` pass (lazy resolution) but `bun run build` does not. | Reverted to the hand-rolled `sparklinePath` (now extracted to `$lib/sparkline.ts`) until upstream ships a fix. `@rokkit/chart` stays in `dependencies` so adoption resumes the moment the package is fixed. | **Publish `src/palette.json`** in the next `@rokkit/chart` release (it's just missing from `package.json#files` / the build output). Either include the file in the published tarball, or inline the master palette into `brewing/colors.js` and `plot/preset.js` directly. |

---

## Examples of likely first entries (verify when implementing)

- **FTR 14-day bar strip** (observatory home, project sidebar) — needs a horizontal baseline rule at a target FTR value plus highlighted last bar. `BarChart` may not expose either.
- **Enso open-arc** (Enso direction screens) — needs a centred numeric label inside the donut and a configurable "open" gap rather than a full ring. `PieChart` with `innerRadius` may handle the shape but not the label.
- **Per-bar opacity gradient** on multi-day strips — older bars dimmer than recent.
- **Animation-in on entry** for charts that update via SSE (scan feed, replay insights).
