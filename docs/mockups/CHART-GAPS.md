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
| 1 | `Sparkline` (observatory home, project overview) | Stroke should resolve to zen-sumi's `--color-primary-z*` (vermillion shu) palette. Currently `Sparkline.svelte` derives stroke from `rgb(var(--color-primary-500, 100,116,139))` and fill from `rgba(var(--color-primary-300), 0.25)`. zen-sumi only defines `--color-primary-z0..z9` as OKLCH triplets — there is no `--color-primary-500`, and the format is wrong for `rgb()`. Result today: stroke falls back to slate-gray. | Accepted the fallback for Step 0; Step 10 (`MiniStat`) will revisit. | Standardise `@rokkit/chart` on zen-sumi's `--color-*-z*` OKLCH scale. Same pattern hits `Plot.svelte` (`--color-surface-700`), `FilterHistogram` (`--color-primary`), etc. |
| 2 | `Sparkline` (observatory home) | The previous hand-rolled SVG drew a 2.5 px circle at the last data point as a "you-are-here" marker. Rokkit's `Sparkline` doesn't expose a last-point indicator. | Last-point dot dropped during Step 0. | Add a `lastPoint?: boolean \| { radius?: number; color?: string }` prop to `Sparkline`. |

---

## Examples of likely first entries (verify when implementing)

- **FTR 14-day bar strip** (observatory home, project sidebar) — needs a horizontal baseline rule at a target FTR value plus highlighted last bar. `BarChart` may not expose either.
- **Enso open-arc** (Enso direction screens) — needs a centred numeric label inside the donut and a configurable "open" gap rather than a full ring. `PieChart` with `innerRadius` may handle the shape but not the label.
- **Per-bar opacity gradient** on multi-day strips — older bars dimmer than recent.
- **Animation-in on entry** for charts that update via SSE (scan feed, replay insights).
