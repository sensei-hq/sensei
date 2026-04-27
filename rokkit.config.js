import { sumiPalette } from './sumi-palette.js';

export default {
  colors: {
    surface:   'sumi',
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
