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
   * Custom token overrides — light/dark variants emit through the preset
   * so they swap automatically with `[data-mode="dark"]`. `paper-edge` in
   * dark mode needs to be DARKER than `paper`, not lighter — Rokkit's
   * default mapping (sumi-400) would land lighter than the sumi-50 bg and
   * produce a "raised edge" look the design rejects. Override with sumi-800
   * (just above paper bg) so the etched-line hairline reads correctly.
   */
  custom: {
    "paper-edge":  { light: "kami.400", dark: "sumi.800" },
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
