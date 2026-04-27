/**
 * Zen/Sumi color palette for Rokkit.
 *
 * Maps the sumi aesthetic into Rokkit's 50-950 shade scale.
 * Each color has 11 stops (50, 100, 200, 300, 400, 500, 600, 700, 800, 900, 950).
 *
 * Rokkit's z-scale then aliases these:
 *   z0=50, z1=100, z2=200, ... z9=900, z10=950 (light mode)
 *   z0=950, z1=900, ... z9=100, z10=50 (dark mode)
 *
 * Color mapping:
 *   sumi (surface)  → warm grey with subtle ochre undertone
 *   shu (primary)   → vermillion / terracotta — the one accent
 *   jade (success)  → muted green
 *   amber (warning) → warm amber
 *   beni (danger)   → deep crimson
 *   ai (info)       → indigo blue
 *   murasaki (secondary) → muted purple
 *   fuji (accent)   → wisteria / soft violet
 */

const sumi = {
  50:  '#FAF8F5',
  100: '#F3F0EB',
  200: '#E8E3DC',
  300: '#D6CFC6',
  400: '#B5ADA2',
  500: '#8A8278',
  600: '#6B635A',
  700: '#524B43',
  800: '#3D3730',
  900: '#2B2620',
  950: '#1C1914',
};

const shu = {
  50:  '#FEF3EE',
  100: '#FDE4D8',
  200: '#FBC5AE',
  300: '#F8A07E',
  400: '#F4764E',
  500: '#E8552B',
  600: '#CC3F1A',
  700: '#A83117',
  800: '#872A19',
  900: '#6E2518',
  950: '#3C100A',
};

const jade = {
  50:  '#F0FAF4',
  100: '#DBEFE3',
  200: '#BAE0CB',
  300: '#8CCAA9',
  400: '#5DAF85',
  500: '#3C946A',
  600: '#2C7754',
  700: '#255F45',
  800: '#214C39',
  900: '#1D3F30',
  950: '#0E231B',
};

const amber = {
  50:  '#FFFBEB',
  100: '#FFF3C6',
  200: '#FFE588',
  300: '#FFD24A',
  400: '#FFBF20',
  500: '#F59E07',
  600: '#D97706',
  700: '#B45309',
  800: '#92400E',
  900: '#783510',
  950: '#451A03',
};

const beni = {
  50:  '#FFF5F5',
  100: '#FFE8E8',
  200: '#FFC9C9',
  300: '#FFA3A3',
  400: '#FF6B6B',
  500: '#E84040',
  600: '#C82828',
  700: '#A32020',
  800: '#861D1D',
  900: '#701E1E',
  950: '#3C0A0A',
};

const ai = {
  50:  '#F0F6FE',
  100: '#DDEAFC',
  200: '#C3DBFA',
  300: '#9AC3F6',
  400: '#6AA3EF',
  500: '#4882E8',
  600: '#3366DC',
  700: '#2A52CA',
  800: '#2944A4',
  900: '#273C82',
  950: '#1C264F',
};

const murasaki = {
  50:  '#FAF5FF',
  100: '#F3E8FF',
  200: '#E9D5FF',
  300: '#D8B4FE',
  400: '#C084FC',
  500: '#A855F7',
  600: '#9333EA',
  700: '#7E22CE',
  800: '#6B21A8',
  900: '#581C87',
  950: '#3B0764',
};

const fuji = {
  50:  '#F5F3FF',
  100: '#EDE9FE',
  200: '#DDD6FE',
  300: '#C4B5FD',
  400: '#A78BFA',
  500: '#8B5CF6',
  600: '#7C3AED',
  700: '#6D28D9',
  800: '#5B21B6',
  900: '#4C1D95',
  950: '#2E1065',
};

export const sumiPalette = {
  sumi,
  shu,
  jade,
  amber,
  beni,
  ai,
  murasaki,
  fuji,
};
