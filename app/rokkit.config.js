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
    // ink-mute / ink-faint retuned for WCAG 2.1 AA (4.5:1) on card surfaces AND to
    // keep a 4-STEP ramp (soft > mute > faint, distinct in BOTH modes). The mockup's
    // own muted inks fail AA on small text (mute 4.0:1, faint 2.07:1), so we keep the
    // darkened values (see docs/spec/2026-08-05-mockup-drift-audit.md F1). Measured:
    //   ink-mute  light 0.470 (6.37:1) / dark 0.760 (7.47:1)
    //   ink-faint light 0.510 (5.37:1) / dark 0.660 (5.16:1)   — distinct from mute both modes.
    // (faint light is 0.510 not 0.545: 0.545 measured 4.62 and axe's stricter calc
    //  tipped it under 4.5 on ~10 screens; 0.510 keeps margin AND stays > mute 0.470.)
    "ink-mute":   { light: "oklch(0.470 0.010 50)", dark: "oklch(0.760 0.009 85)" },
    "ink-faint":  { light: "oklch(0.510 0.010 50)", dark: "oklch(0.660 0.009 85)" },

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

    // ── Status/accent SOFT (tinted callout backgrounds) — ALPHA-COMPOSITE model ──
    // The mockup (WCAG-clean source of truth) defines each `-soft` as a TRANSLUCENT
    // tint of its base — oklch(<base> / 0.10–0.20) — so it composites over whatever
    // paper surface it sits on and is dark-safe BY CONSTRUCTION (no per-mode pale-vs-
    // dark shade juggling). We match that with color-mix over the (mode-correct)
    // named base token, at the mockup alphas (a touch higher in dark). This replaces
    // the earlier solid two-pole shades (.100/.900) that couldn't composite and forced
    // the hardcoded dark workaround. See mockup-drift-audit F2.
    "accent-soft":  { light: "color-mix(in oklch, var(--accent) 14%, transparent)",  dark: "color-mix(in oklch, var(--accent) 20%, transparent)"  },
    "success-soft": { light: "color-mix(in oklch, var(--success) 14%, transparent)", dark: "color-mix(in oklch, var(--success) 20%, transparent)" },
    "warning-soft": { light: "color-mix(in oklch, var(--warning) 15%, transparent)", dark: "color-mix(in oklch, var(--warning) 22%, transparent)" },
    "danger-soft":  { light: "color-mix(in oklch, var(--danger) 12%, transparent)",  dark: "color-mix(in oklch, var(--danger) 18%, transparent)"  },
    "error-soft":   { light: "color-mix(in oklch, var(--error) 12%, transparent)",   dark: "color-mix(in oklch, var(--error) 18%, transparent)"   },
    "info-soft":    { light: "color-mix(in oklch, var(--info) 14%, transparent)",    dark: "color-mix(in oklch, var(--info) 20%, transparent)"    },
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
