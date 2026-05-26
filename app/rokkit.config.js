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
   * Dual-surface skin:
   *   light → kami  (warm washi paper tones, z0=lightest surface)
   *   dark  → sumi  (ink tones, z-flip: z0=darkest bg, z9=lightest text)
   */
  skin: {
    surface: { light: "kami", dark: "sumi" },
    paper: { light: "kami", dark: "sumi" }, // alias of surface — bg-paper-z0 = page, z1 = card, …
    ink: { light: "sumi", dark: "kami" },
    primary: "shu", // vermillion — the one accent (朱)
    secondary: "murasaki", // muted purple (紫)
    accent: "fuji", // wisteria violet (藤)
    success: "hisui", // jade green (翡翠)
    warning: "kohaku", // warm amber (琥珀)
    danger: "beni", // deep crimson (紅)
    error: "beni", // alias for danger
    info: "ai", // indigo blue (藍)
  },

  typography: {
    sans: "'Inter Variable', 'Inter', system-ui, -apple-system, sans-serif",
    mono: "'JetBrains Mono', 'SF Mono', Menlo, monospace",
    heading: "'Fraunces', 'Iowan Old Style', Georgia, serif",
    kanji: "'Yu Mincho', 'Hiragino Mincho ProN', 'Songti SC', serif",
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
