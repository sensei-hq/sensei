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
    primary:   "shu",       // vermillion — the one accent (朱)
    secondary: "murasaki",  // muted purple (紫)
    accent:    "fuji",      // wisteria violet (藤)
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
    "paper-edge": { light: "kami.400", dark: "sumi.100" },
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
