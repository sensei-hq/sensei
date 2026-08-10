import { sumiPalette } from '../packages/sumi-palette/index.js';

/**
 * Marketing site — Rokkit named-token config.
 *
 * Shares the one Zen/Sumi OKLCH palette with the desktop app + dōjō console
 * (packages/sumi-palette) so all three surfaces read as one brand. The palette
 * is two-pole, so it needs `colorSpace: 'oklch'` + the explicit paper/ink stop
 * overrides below (the same mapping app/dojo use) — the default skin mapping
 * would pick the wrong surface shades. Site-specific pieces kept: the per-product
 * accent tokens for the hub cards, the translucent `paper-edge`, and the brand
 * icon collection.
 */
export default {
  palettes: sumiPalette,
  colorSpace: 'oklch',

  skin: {
    surface:   { light: 'kami', dark: 'sumi' },
    ink:       { light: 'kami', dark: 'sumi' },
    primary:   'shu',
    secondary: 'murasaki',
    accent:    'shu', // HQ/root brand accent = vermillion (purple is reserved for Kavach)
    success:   'hisui',
    warning:   'kohaku',
    danger:    'beni',
    error:     'beni',
    info:      'ai',
  },

  overrides: {
    // ── Surface (paper) / Ink — explicit stop mapping the two-pole sumi palette
    //    needs; shared verbatim with the desktop app + dōjō console. ──
    paper:        { light: 'kami.100', dark: 'sumi.50'  },
    'paper-soft': { light: 'kami.200', dark: 'sumi.100' },
    'paper-mute': { light: 'kami.300', dark: 'sumi.200' },
    ink:          { light: 'kami.900', dark: 'sumi.900' },
    'ink-soft':   { light: 'kami.700', dark: 'sumi.800' },
    // ink-mute / ink-faint darkened for WCAG AA (4.5:1) — the default kami/sumi
    // stops read too faint for small mono labels. Values match the desktop app's
    // tuned inks (mute ~6.4:1, faint ~5.3:1 in both modes).
    'ink-mute':   { light: 'oklch(0.450 0.010 50)', dark: 'oklch(0.770 0.009 85)' },
    'ink-faint':  { light: 'oklch(0.500 0.010 50)', dark: 'oklch(0.680 0.009 85)' },

    // ── Status — light values darkened so status TEXT passes WCAG AA; dark keeps
    //    the lighter .400 shades. The `-soft` fills are translucent tints
    //    (color-mix) so they composite over the surface and flip per mode instead
    //    of the default solid pale shade that fails contrast in dark mode. ──
    // Primary = the vermillion CTA button fill. Darkened to L0.485 so white
    // (on-primary) button text clears WCAG AA (the skin default shu.500 gave
    // only 4.41:1); kept mode-invariant since the button doesn't flip.
    primary:        { light: 'oklch(0.485 0.150 35)',  dark: 'oklch(0.485 0.150 35)' },
    'on-primary':   { light: 'oklch(0.985 0.005 85)',  dark: 'oklch(0.985 0.005 85)' },
    accent:         { light: 'oklch(0.485 0.150 35)',  dark: 'shu.400' },
    success:        { light: 'oklch(0.475 0.080 160)', dark: 'hisui.400' },
    warning:        { light: 'oklch(0.480 0.102 75)',  dark: 'kohaku.400' },
    danger:         { light: 'oklch(0.490 0.178 25)',  dark: 'beni.400' },
    error:          { light: 'var(--danger)', dark: 'var(--danger)' },
    info:           { light: 'oklch(0.520 0.150 254)', dark: 'ai.400' },
    'accent-soft':  { light: 'color-mix(in oklch, var(--accent) 14%, transparent)',  dark: 'color-mix(in oklch, var(--accent) 20%, transparent)' },
    'success-soft': { light: 'color-mix(in oklch, var(--success) 14%, transparent)', dark: 'color-mix(in oklch, var(--success) 20%, transparent)' },
    'warning-soft': { light: 'color-mix(in oklch, var(--warning) 15%, transparent)', dark: 'color-mix(in oklch, var(--warning) 22%, transparent)' },
    'danger-soft':  { light: 'color-mix(in oklch, var(--danger) 10%, transparent)',  dark: 'color-mix(in oklch, var(--danger) 18%, transparent)' },
    'error-soft':   { light: 'var(--danger-soft)', dark: 'var(--danger-soft)' },
    'info-soft':    { light: 'color-mix(in oklch, var(--info) 14%, transparent)',    dark: 'color-mix(in oklch, var(--info) 20%, transparent)' },

    // ── Per-product accent hues (from docs/mockups/Sensei/hq/site.jsx ACCENTS).
    // Custom tokens — each emits a --<product> var + bg-/text-/border- utilities,
    // flipping light↔dark automatically. Used on the hub's product cards. ──
    // Light values sit at ~L0.50 so the accent passes WCAG AA even as small
    // `text-sm` link text ("Explore …"); the big kanji only need 3:1 and clear it
    // easily. Dark values stay light (on the dark surface).
    sensei:  { light: 'oklch(0.505 0.160 35)',  dark: 'oklch(0.700 0.150 35)'  },
    torii:   { light: 'oklch(0.500 0.155 20)',  dark: 'oklch(0.700 0.140 15)'  },
    seiki:   { light: 'oklch(0.495 0.145 255)', dark: 'oklch(0.700 0.130 255)' },
    gateway: { light: 'oklch(0.500 0.110 80)',  dark: 'oklch(0.720 0.120 85)'  },
    dbd:     { light: 'oklch(0.495 0.145 255)', dark: 'oklch(0.700 0.130 255)' },
    rokkit:  { light: 'oklch(0.495 0.120 162)', dark: 'oklch(0.730 0.110 162)' },
    kavach:  { light: 'oklch(0.485 0.170 305)', dark: 'oklch(0.700 0.150 305)' },
    magpie:  { light: 'oklch(0.500 0.120 210)', dark: 'oklch(0.720 0.110 200)' },
    kata:    { light: 'oklch(0.500 0.130 150)', dark: 'oklch(0.720 0.120 145)' },
    burne:   { light: 'oklch(0.510 0.150 50)',  dark: 'oklch(0.730 0.140 50)'  },

    // Retune the reserved paper-edge token to a faint translucent hairline that
    // clearly flips per mode (default skin shade reads too heavy on the cards).
    'paper-edge': { light: 'oklch(0.22 0.012 50 / 0.10)', dark: 'oklch(0.94 0.008 85 / 0.12)' },
  },

  themes: ['zen-sumi'],
  typography: {
    sans: "'Inter', system-ui, -apple-system, sans-serif",
    mono: "'JetBrains Mono', 'SF Mono', Menlo, monospace",
    heading: "'Fraunces', 'Iowan Old Style', Georgia, serif",
  },
  // Brand/product logos as an Iconify collection — generated by the rokkit CLI
  // (`bun run icons:bundle`) from icons/brand/*.svg, monochromed to currentColor
  // so each mark tints via its per-product token (text-sensei, text-dbd, …) and
  // flips light↔dark. Reference as i-brand:sensei / i-brand:dbd / i-brand:kavach…
  icons: {
    brand: './src/lib/icons/brand.json',
  },
  switcher: 'manual',
  storageKey: 'sensei-site-theme',
};
