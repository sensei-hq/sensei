import { sumiPalette } from './sumi-palette.js';

export default {
  colors: {
    surface:   'sumi',       // paper/ink greys
    primary:   'shu',        // vermillion accent
    secondary: 'murasaki',   // muted purple
    accent:    'fuji',       // wisteria violet
    success:   'jade',       // positive green
    warning:   'amber',      // warm amber
    danger:    'beni',       // crimson
    error:     'beni',       // crimson (alias)
    info:      'ai',         // indigo blue
  },
  palettes: sumiPalette,
  themes: ['rokkit'],
  typography: {
    sans: "'Inter Variable', 'Inter', system-ui, -apple-system, sans-serif",
    mono: "'JetBrains Mono', 'SF Mono', Menlo, monospace",
    heading: "'Fraunces', 'Iowan Old Style', Georgia, serif",
  },
  icons: {},
  switcher: 'manual',
  storageKey: 'sensei-desktop-theme',
};
