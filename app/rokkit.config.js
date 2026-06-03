import { sumiPalette } from "./sumi-palette.js";

export default {
  /**
   * Zen/Sumi palettes — OKLCH bare-component format.
   * colorSpace: 'oklch' is required so Rokkit stores CSS vars as
   * bare L C H triplets, consumed via oklch(var(--color-*) / alpha).
   * See sumi-palette.js for full scale definitions.
   */
  palettes: sumiPalette,
  colorSpace: "oklch",

  /**
   * Dual-surface skin — both `surface` and `ink` pull from the same palette
   * pair (kami in light, sumi in dark). The sumi palette is two-pole: stops
   * 50–400 are warm paper whites (used as text in dark mode), 500–950 are
   * sumi-ink darks (used as backgrounds). The preset's auto-flip in
   * `[data-mode="dark"]` puts text at the light end and bg at the dark end
   * automatically — no manual inversion needed here.
   */
  skin: {
    surface:   { light: "kami", dark: "sumi" },
    ink:       { light: "kami", dark: "sumi" },
    primary:   "shu",       // vermillion — the one accent (朱); skin re-alignment deferred to Phase 2/3
    secondary: "murasaki",  // muted purple (紫)
    accent:    "shu",       // vermillion (朱) — was fuji; aligns skin with accent override
    success:   "hisui",     // jade green (翡翠)
    warning:   "kohaku",    // warm amber (琥珀)
    danger:    "beni",      // deep crimson (紅)
    error:     "beni",      // alias for danger
    info:      "ai",        // indigo blue (藍)
  },

  /**
   * Reserved-name overrides (Rokkit 1.1.1+). The preset emits these per
   * mode so `[data-mode="dark"]` swaps the value automatically — no CSS
   * shim needed at the consumer.
   *
   * `paper-edge` defaults to the surface palette's mid-lightness stop,
   * which on our two-pole sumi (dark) is LIGHTER than the page bg and
   * reads as a "raised edge". Override the dark value to a stop just
   * a touch above paper so the hairline looks etched, not lifted.
   */
  overrides: {
    // ── Surface (paper) ──────────────────────────────────────────
    paper:        { light: "kami.100", dark: "sumi.50"  },
    "paper-soft": { light: "kami.200", dark: "sumi.100" },
    "paper-mute": { light: "kami.300", dark: "sumi.200" },
    "paper-edge": { light: "kami.400", dark: "sumi.100" },  // etched hairline in dark

    // ── Ink (text-zone shades; sumi.600-900 is the two-pole text half) ─
    ink:          { light: "kami.900", dark: "sumi.900" },
    "ink-soft":   { light: "kami.700", dark: "sumi.800" },
    "ink-mute":   { light: "kami.500", dark: "sumi.700" },
    "ink-faint":  { light: "kami.300", dark: "sumi.600" },

    // ── Accent — vermillion (design system: --accent: var(--shu-500)) ─
    // accent-soft is omitted: skin is now `accent: "shu"` so the canonical
    // default resolves to shu.100 / shu.200 — same as the former override.
    // accent override (shade 400 dark shift) is kept for dark-mode legibility.
    accent:        { light: "shu.500", dark: "shu.400" },

    // ── Primary named token — ink-colored CTA (design system: --primary: var(--ink)) ─
    // Named `bg-primary` / `text-primary` = ink color (for ink-on-paper buttons).
    // z-scale `text-primary-z*` still resolves via skin (primary: shu) = vermillion
    // in unmigrated screens. Full skin re-alignment deferred to Phase 2/3.
    primary:      { light: "kami.900", dark: "sumi.900" },
    "on-primary": { light: "kami.100", dark: "sumi.50"  },

    // ── Status — lighten for legibility in dark mode (shade 400 vs 500) ─
    success:      { light: "hisui.500",  dark: "hisui.400"  },
    warning:      { light: "kohaku.500", dark: "kohaku.400" },
    danger:       { light: "beni.500",   dark: "beni.400"   },
    info:         { light: "ai.500",     dark: "ai.400"     },
  },

  typography: {
    sans:    "'Inter Variable', 'Inter', system-ui, -apple-system, sans-serif",
    mono:    "'JetBrains Mono', 'SF Mono', Menlo, monospace",
    display: "'Fraunces', 'Iowan Old Style', Georgia, serif",
    kanji:   "'Yu Mincho', 'Hiragino Mincho ProN', 'Songti SC', serif",
  },

  /**
   * Shape — matches the app's current radius usage:
   *   --radius:    6px  ≈ soft.md (0.375rem)
   *   --radius-lg: 10px ≈ soft.lg (0.625rem)
   *   pill/avatar: 100px → soft.full (9999px)
   */
  shape: {
    radius: "soft",
  },

  // themes: ['zen-sumi']  — uncomment when zen-sumi CSS is published to @rokkit/themes
  icons: {},
  switcher: "manual",
  storageKey: "sensei-desktop-theme",
};
