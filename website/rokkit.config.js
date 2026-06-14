import { sumiPalette } from './sumi-palette.js';

export default {
  skin: {
    surface:   { light: 'sumi', dark: 'sumiDark' },
    ink:       { light: 'sumi', dark: 'sumiDark' },
    primary:   'shu',
    secondary: 'murasaki',
    accent:    'fuji',
    success:   'jade',
    warning:   'amber',
    danger:    'beni',
    error:     'beni',
    info:      'ai',
  },
  palettes: sumiPalette,
  // Per-product accent hues (from docs/mockups/Sensei/hq/site.jsx ACCENTS).
  // Custom tokens — each emits a --<product> var + bg-/text-/border- utilities,
  // flipping light↔dark automatically. Used on the hub's product cards.
  overrides: {
    sensei: { light: 'oklch(0.580 0.150 35)',  dark: 'oklch(0.700 0.150 35)'  },
    dbd:    { light: 'oklch(0.560 0.130 255)', dark: 'oklch(0.700 0.130 255)' },
    rokkit: { light: 'oklch(0.560 0.110 162)', dark: 'oklch(0.730 0.110 162)' },
    kavach: { light: 'oklch(0.520 0.150 305)', dark: 'oklch(0.700 0.150 305)' },
    magpie: { light: 'oklch(0.560 0.110 200)', dark: 'oklch(0.720 0.110 200)' },
    kata:   { light: 'oklch(0.560 0.120 145)', dark: 'oklch(0.720 0.120 145)' },
    burne:  { light: 'oklch(0.580 0.140 50)',  dark: 'oklch(0.730 0.140 50)'  },
    // Retune the reserved paper-edge token to a faint translucent hairline that
    // clearly flips per mode (default skin shade reads too heavy on the cards).
    'paper-edge': { light: 'oklch(0.22 0.012 50 / 0.10)', dark: 'oklch(0.94 0.008 85 / 0.12)' },
  },
  themes: ['rokkit'],
  typography: {
    sans: "'Inter', system-ui, -apple-system, sans-serif",
    mono: "'JetBrains Mono', 'SF Mono', Menlo, monospace",
    heading: "'Fraunces', 'Iowan Old Style', Georgia, serif",
  },
  icons: {},
  switcher: 'manual',
  storageKey: 'sensei-site-theme',
};
