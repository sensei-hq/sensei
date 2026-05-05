import { sumiPalette } from './sumi-palette.js';

export default {
  /**
   * Zen/Sumi OKLCH-inspired palettes — named semantic color scales.
   * All values are sRGB hex; colorSpace defaults to 'rgb'.
   * See sumi-palette.js for full scale definitions.
   */
  palettes: sumiPalette,

  /**
   * Single-skin mode — one fixed colormap for this desktop app.
   * Maps Rokkit semantic roles to the zen/sumi palette names above.
   */
  skin: {
    surface:   'sumi',      // warm grey paper/ink (the base surface scale)
    primary:   'shu',       // vermillion — the one accent (朱)
    secondary: 'murasaki',  // muted purple (紫)
    accent:    'fuji',      // wisteria violet (藤)
    success:   'jade',      // positive green (翠)
    warning:   'amber',     // warm amber (琥珀)
    danger:    'beni',      // deep crimson (紅)
    error:     'beni',      // alias for danger
    info:      'ai',        // indigo blue (藍)
  },

  typography: {
    sans:    "'Inter Variable', 'Inter', system-ui, -apple-system, sans-serif",
    mono:    "'JetBrains Mono', 'SF Mono', Menlo, monospace",
    heading: "'Fraunces', 'Iowan Old Style', Georgia, serif",
  },

  /**
   * Shape — matches the app's current radius usage:
   *   --radius:    6px  ≈ soft.md (0.375rem)
   *   --radius-lg: 10px ≈ soft.lg (0.625rem)
   *   pill/avatar: 100px → soft.full (9999px)
   */
  shape: {
    radius: 'soft',
  },

  // themes: ['zen-sumi']  — uncomment when zen-sumi CSS is published to @rokkit/themes
  icons:      {},
  switcher:   'manual',
  storageKey: 'sensei-desktop-theme',
};
