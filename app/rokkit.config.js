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
    // Dark `paper-edge` MUST be lighter than `paper-soft` (sumi.100) to be
    // visible against panel backgrounds. sumi.300 gives Δ 0.110 lightness vs
    // paper-soft — subtle but distinctly etched (NOT the same shade as the bg).
    "paper-edge": { light: "kami.400", dark: "sumi.300" },

    // ── Ink (text-zone shades) ─────────────────────────────────────
    // Light: kami.600-900 — the kami palette comments place text at z6+.
    //   kami.300/400/500 are paper-shades (too light for text contrast).
    // Dark: sumi.600-900 — sumi is two-pole; 600-900 is the text half.
    // Mockup values (Zen-Sumi colors_and_type.css):
    //   ink       light 0.220  ↔ kami.900 ✓ / dark 0.940 ↔ sumi.900 ✓
    //   ink-soft  light 0.380  ↔ kami.700 ✓ / dark 0.780 ↔ sumi.800 ✓
    //   ink-mute  light 0.580  ↔ kami.600 ✓ / dark 0.600 ↔ sumi.700 ✓
    //   ink-faint light 0.750  ↔ kami.500 ✓ / dark 0.420 ↔ sumi.600 ✓
    ink:          { light: "kami.900", dark: "sumi.900" },
    "ink-soft":   { light: "kami.700", dark: "sumi.800" },
    // ink-mute / ink-faint retuned for WCAG 2.1 AA (4.5:1) on card surfaces —
    // measured: old light kami.600 (0.58L) = 4.0:1 and dark sumi.600 (0.42L) =
    // 2.3:1 both FAILED for small labels (eyebrows, captions, timestamps). Raw
    // oklch keeps the ink hue (light ~50 / dark ~85) at contrasts ≥4.5:1 while
    // preserving the soft > mute > faint prominence order. See /tmp/contrast.mjs.
    "ink-mute":   { light: "oklch(0.500 0.010 50)", dark: "oklch(0.740 0.009 85)" },
    "ink-faint":  { light: "oklch(0.500 0.010 50)", dark: "oklch(0.680 0.009 85)" },

    // ── Accent — vermillion (design system: --accent: var(--shu-500)) ─
    // accent-soft is omitted: skin is now `accent: "shu"` so the canonical
    // default resolves to shu.100 / shu.200 — same as the former override.
    // accent override (shade 400 dark shift) is kept for dark-mode legibility.
    // Light value darkened shu.500 (0.58L, 4.27:1) → 0.520L (5.5:1) so vermillion
    // TEXT (stat numbers, arrows, kanji labels) passes WCAG AA. Dark keeps shu.400.
    accent:        { light: "oklch(0.520 0.145 35)", dark: "shu.400" },

    // ── Primary named token — ink-colored CTA (design system: --primary: var(--ink)) ─
    // Named `bg-primary` / `text-primary` = ink color (for ink-on-paper buttons).
    // z-scale `text-primary-z*` still resolves via skin (primary: shu) = vermillion
    // in unmigrated screens. Full skin re-alignment deferred to Phase 2/3.
    primary:      { light: "kami.900", dark: "sumi.900" },
    "on-primary": { light: "kami.100", dark: "sumi.50"  },

    // ── Status (solid/text) ────────────────────────────────────────────────────
    // Light values DARKENED for WCAG AA — the .500 shades read as light text on
    // white and failed 4.5:1 (warning 2.35:1, success 3.26:1, danger 4.55:1). Raw
    // oklch keeps each hue at ~5:1 for status TEXT (counts, verdicts, deltas). Dark
    // keeps the lightened .400 shades (dark mode already passes on the dark surface).
    success:      { light: "oklch(0.510 0.076 160)", dark: "hisui.400"  },
    warning:      { light: "oklch(0.510 0.100 75)",  dark: "kohaku.400" },
    danger:       { light: "oklch(0.520 0.178 25)",  dark: "beni.400"   },
    error:        { light: "oklch(0.520 0.178 25)",  dark: "beni.400"   },
    info:         { light: "oklch(0.520 0.150 254)", dark: "ai.400"     },

    // ── Status/accent SOFT (tinted callout backgrounds) — MUST flip per mode ─────
    // The status palettes are single-pole (50→950), so the preset derives every
    // `-soft` from the pale shade-100 (~0.94L) and emits the SAME value in the dark
    // block — a pale tint that stays pale in dark mode. `text-<status>` (shade 400,
    // ~0.72–0.79L) on it is then near-invisible in dark mode (every pill/card
    // contrast bug traces here). Give each soft a real dark tint (shade 900, ~0.25–
    // 0.38L) so coloured text/icons contrast in BOTH modes. Light keeps shade-100
    // (identical to today — no light-mode regression). This is the SYSTEMIC fix that
    // makes `bg-*-soft text-*` dark-safe everywhere, not per-component.
    "accent-soft":  { light: "shu.100",    dark: "shu.900"    },
    "success-soft": { light: "hisui.100",  dark: "hisui.900"  },
    "warning-soft": { light: "kohaku.100", dark: "kohaku.900" },
    "danger-soft":  { light: "beni.100",   dark: "beni.900"   },
    "error-soft":   { light: "beni.100",   dark: "beni.900"   },
    "info-soft":    { light: "ai.100",     dark: "ai.900"     },
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
