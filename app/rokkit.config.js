import { zenSumi } from '@rokkit/unocss/palettes';

export default {
  // rokkit's zen-sumi family — palettes and skin imported, never copied
  // (user 2026-10-28). The app adds tokens; it overrides none of rokkit's
  // (src/lib/rokkit-defaults.spec.ts).
  palettes: zenSumi.palettes,
  colorSpace: "oklch",
  skin: zenSumi.skin,

  overrides: {
    // Modal/dialog backdrop scrim — a dark veil in both modes.
    "scrim":        { light: "oklch(0.150 0.008 50 / 0.45)", dark: "oklch(0.120 0.008 50 / 0.55)" },
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
