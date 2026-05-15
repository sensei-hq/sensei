# Zen-Sumi theme overrides

> Local CSS overrides we apply on top of `@rokkit/themes/zen-sumi` when a Rokkit component doesn't visually match the Sensei mockups out of the box.
> Each entry is a candidate for upstreaming to `@rokkit/themes/zen-sumi` — once stable, raise a PR against Rokkit and remove the local override.

## Rules

1. **Never fork the Rokkit component.** Override via CSS only.
2. Target the same selectors Rokkit's zen-sumi theme uses (`[data-style='zen-sumi']`, `[data-chart-*]`, `[data-plot-element='…']`, the component's class).
3. Keep overrides in one stylesheet per component family (e.g. `app/src/lib/theme/overrides/card.css`) and `@import` them after the Rokkit zen-sumi theme so cascade wins.
4. Every entry needs the **mockup reference** (file + line range) and the **gap** it closes.
5. Once an override matches the mockup, open an upstream PR and delete the local CSS in the same release that the new Rokkit version ships.

## Overrides

| # | Component | Mockup ref | Gap | Local rule (path) | Upstream PR |
|---|---|---|---|---|---|
| _none yet — first entry added when Step 1 lands_ | | | | | |
