// ═══════════════════════════════════════════════════════════════════
// Zen-Sumi · UnoCSS theme — REFERENCE for the app's config
// ═══════════════════════════════════════════════════════════════════
// These are the literals the app's dojo/uno.config.js + app/uno.config.js
// already ship. This file exists so mock ↔ app can be diffed value-for-value.
//
// DO NOT "merge" it into the app. The app hardcodes these on purpose:
// --text-*, --shadow, --leading-*, --tracking-* and --dur* are NOT defined
// in the app's token layer, so a var() reference there resolves to nothing
// and the type scale, shadows and line-heights collapse silently. Only
// --font-*, --radius-* and --space-* exist app-side.
//
// presetRokkit already gives bg/text/border-{token}, Uno's 4px spacing and
// the sm/md/lg/xl breakpoints — all of which the mock matches. Only the
// scale below (type, radii, shadow, families, tracking/leading, motion) is
// declared.

import { defineConfig } from 'unocss';
import { presetRokkit } from '@rokkit/unocss';

export default defineConfig({
  presets: [presetRokkit()],

  theme: {
    // Type · 8 stops, floor xs (11). Tuple form [size, lineHeight] — the
    // line-height is bundled so `text-sm` alone matches the app exactly,
    // with no stray leading-* needed.
    fontSize: {
      xs:    ['11px', '1.4'],
      sm:    ['13px', '1.5'],
      base:  ['15px', '1.6'],
      lg:    ['17px', '1.5'],
      xl:    ['22px', '1.2'],
      '2xl': ['28px', '1.2'],
      '3xl': ['40px', '1.2'],
      '4xl': ['56px', '1.05'],
    },

    // Families resolve to --font-* tokens, which DO exist app-side.
    fontFamily: {
      display: 'var(--font-display)', // Fraunces — headings
      ui:      'var(--font-ui)',      // Inter — body + chrome
      mono:    'var(--font-mono)',    // JetBrains — numbers, ids, paths
      kanji:   'var(--font-kanji)',   // Mincho — the functional marks
    },

    // Hardcoded — the app has no --leading-* / --tracking-* tokens.
    lineHeight:    { tight: '1.2', snug: '1.4', normal: '1.6', loose: '1.75' },
    letterSpacing: { tight: '-0.02em', normal: '0', wide: '0.18em' },

    // Radii — hardcoded to match the app. `rounded-full` is for pills, dots,
    // avatars and toggle tracks ONLY; never a wide card. Cards are -lg.
    borderRadius: { sm: '4px', DEFAULT: '6px', lg: '10px', full: '9999px' },

    // Motion — one curve, three durations. Hardcoded (no --dur* app-side).
    transitionDuration:       { fast: '120ms', DEFAULT: '180ms', slow: '280ms' },
    transitionTimingFunction: { DEFAULT: 'cubic-bezier(0.2, 0.6, 0.2, 1)' },

    // Elevation — modals, popovers and command palettes ONLY; cards and
    // buttons never shadow, the system separates with hairlines. The one
    // place a var-ref stays: the requirement is visual parity (soft,
    // ink-tinted), not an identical expression.
    boxShadow: {
      sm:      'var(--shadow-sm)',
      DEFAULT: 'var(--shadow)',
      lg:      'var(--shadow-lg)',
    },

    // Weights — the app doesn't restrict these yet, so font-bold (700) is
    // writable in-app but not in the mock. Values match Uno's defaults, so
    // this block only matters if the app wants the 4-weight guarantee
    // enforced (app-side follow-up C).
    fontWeight: { light: '300', normal: '400', medium: '500', semibold: '600' },

    // Spacing + breakpoints — deliberately NOT declared. Uno's defaults
    // already ARE this system's scale (4px × n; sm 640 / md 768 / lg 1024 /
    // xl 1280) and presetRokkit inherits them. Re-declaring would drop the
    // fractional stops (p-0.5 = 2px) that hairline-gap controls need.
  },

  // ─── No shortcuts, by design ──────────────────────────────────────
  // Answering the "pick one side" question on border-1px: the mock now
  // emits raw `border border-paper-edge`, the same thing the app's shipped
  // screens already write. So there is nothing for the app team to add —
  // no border-1px, no surface-ink, no surface-accent (app-side follow-up B
  // can be dropped).
  //
  // Recipes belong to components, not class aliases: a card is <Card>, a
  // row is <Row>, a button is <Button>. A `card` shortcut alongside <Card>
  // would be a second way to say the same thing, and the two would drift.
  //
  // The one thing that must exist somewhere is the reset both engines
  // assume: `*, ::before, ::after { box-sizing: border-box; border-width: 0;
  // border-style: solid }`. Without border-style, every `border` utility
  // silently collapses to nothing. It lives in tokens.css @layer zs-base.

  // Required ONLY if the app adopts tokens.css's @layer zs-components (it
  // doesn't today — it has its own tokens.css with no component layer).
  // The mock keeps it so utilities outrank component defaults.
  outputToCssLayers: { cssLayerName: () => 'uno-utilities' },
});
