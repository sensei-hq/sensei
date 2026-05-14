/**
 * Zen/Sumi color palette for Rokkit — OKLCH format.
 *
 * Values are bare "L C H" OKLCH components. Requires colorSpace: 'oklch'.
 *
 * Dual-surface design:
 *   kami  — warm paper scale (light mode surface, z0→z9 = lightest→darkest)
 *   sumi  — ink scale (dark mode surface, z-flip: z0=950=dark bg, z9=100=light text)
 *
 * Key anchors:
 *   Surface (kami light / sumi dark, z-flip — text at z6-z9 in both modes):
 *     z1  kami-100 / sumi-900  page bg
 *     z2  kami-200 / sumi-800  card bg
 *     z3  kami-300 / sumi-700  inset
 *     z4  kami-400 / sumi-600  border/separator
 *     z6  kami-600 / sumi-400  faint text
 *     z7  kami-700 / sumi-300  tertiary text
 *     z8  kami-800 / sumi-200  secondary text
 *     z9  kami-900 / sumi-100  primary text
 *
 *   Accent (z6 = semantic anchor, adaptive via flip):
 *     primary-z6  light shu-600   0.580 0.150 35  / dark shu-400   0.700 0.150 35
 *     success-z6  light hisui-600 0.620 0.080 160 / dark hisui-400 0.720 0.090 160
 *     warning-z6  light kohaku-600 ~0.640          / dark kohaku-400 0.790 0.110 75
 */

/** kami — warm neutral, from washi paper (z0) to sumi ink (z9/z10)
 *
 * Text levels start at z6 — mirrors the sumi dark scale so surface-z6/7/8/9
 * carry identical semantics in both modes via Rokkit's z-flip:
 *
 *   z1  kami-100: page bg          | z1  sumi-900: page bg
 *   z2  kami-200: card bg          | z2  sumi-800: card bg
 *   z3  kami-300: inset            | z3  sumi-700: inset
 *   z4  kami-400: border/separator | z4  sumi-600: border/separator
 *   z5  kami-500: mid neutral      | z5  sumi-500: mid neutral
 *   z6  kami-600: faint text       | z6  sumi-400: faint text
 *   z7  kami-700: tertiary text    | z7  sumi-300: tertiary text
 *   z8  kami-800: secondary text   | z8  sumi-200: secondary text
 *   z9  kami-900: primary text     | z9  sumi-100: primary text
 */
const kami = {
  50: "0.985 0.005 85",
  100: "0.975 0.008 85",
  200: "0.955 0.010 85",
  300: "0.920 0.012 85",
  400: "0.850 0.010 70",
  500: "0.750 0.008 50",
  600: "0.580 0.010 50",
  700: "0.380 0.012 50",
  800: "0.280 0.012 50",
  900: "0.220 0.012 50",
  950: "0.170 0.010 50",
};

/** shu — vermillion, primary accent (朱)
 *
 * z6 is the semantic anchor — correct in both modes via Rokkit's flip:
 *   light z6 → shu-600: 0.580 0.150 35  (vivid on light bg)
 *   dark  z6 → shu-400: 0.700 0.150 35  (lighter for dark bg)
 */
const shu = {
  50: "0.970 0.020 35",
  100: "0.940 0.040 35",
  200: "0.880 0.070 35",
  300: "0.800 0.100 35",
  400: "0.700 0.130 35",
  500: "0.580 0.150 35",
  600: "0.500 0.140 35",
  700: "0.420 0.120 35",
  800: "0.350 0.100 35",
  900: "0.280 0.080 35",
  950: "0.220 0.060 35",
};

/** hisui — jade green, success (翡翠)
 *
 * z6 is the semantic anchor — correct in both modes via Rokkit's flip:
 *   light z6 → hisui-600: 0.620 0.080 160  (vivid on light bg)
 *   dark  z6 → hisui-400: 0.720 0.090 160  (lighter for dark bg)
 */
const hisui = {
  50: "0.970 0.015 160",
  100: "0.940 0.030 160",
  200: "0.880 0.050 160",
  300: "0.800 0.065 160",
  400: "0.720 0.075 160",
  500: "0.620 0.080 160",
  600: "0.540 0.075 160",
  700: "0.460 0.065 160",
  800: "0.380 0.055 160",
  900: "0.300 0.045 160",
  950: "0.240 0.035 160",
};

/** kohaku — amber, warning (琥珀) */
const kohaku = {
  50: "0.980 0.020 75",
  100: "0.950 0.040 75",
  200: "0.900 0.070 75",
  300: "0.850 0.095 75",
  400: "0.790 0.110 75",
  500: "0.720 0.120 75",
  600: "0.640 0.110 75",
  700: "0.560 0.095 75",
  800: "0.470 0.080 75",
  900: "0.380 0.065 75",
  950: "0.300 0.050 75",
};

/**
 * sumi — dark-mode surface scale.
 *
 * Two-pole design: light end (50–400) = warm paper whites for dark-mode text,
 * dark end (500–950) = sumi-ink tones for dark-mode backgrounds.
 * The z-flip in base.css maps z0→950 and z9→100.
 *
 * Shifted so z1 dark = page bg (mirrors kami: z1 light = page bg):
 *
 *   z0 dark (body)   ← sumi-950: 0.130 0.008 50  (deepest bg, body)
 *   z1 dark (page)   ← sumi-900: 0.170 0.010 50  (page bg = --paper dark)
 *   z2 dark (card)   ← sumi-800: 0.210 0.012 50  (--paper-2 dark)
 *   z3 dark (inset)  ← sumi-700: 0.250 0.012 50  (--paper-3 dark)
 *   z4 dark (border) ← sumi-600: 0.320 0.012 50  (separator)
 *   z5 dark (mid)    ← sumi-500: 0.380 0.010 60  (neutral midpoint, no flip)
 *   z6 dark (faint)  ← sumi-400: 0.420 0.012 85  (--sumi-4 dark)
 *   z9 dark (text)   ← sumi-100: 0.940 0.008 85  (--sumi dark)
 */
const sumi = {
  50: "0.975 0.008 85",
  100: "0.940 0.008 85",
  200: "0.780 0.008 85",
  300: "0.600 0.010 85",
  400: "0.420 0.012 85",
  500: "0.570 0.010 50",
  600: "0.420 0.010 50",
  700: "0.320 0.012 50",
  800: "0.250 0.012 50",
  900: "0.210 0.012 50",
  950: "0.170 0.010 50",
};

/** beni — deep crimson, danger/error (紅) */
const beni = {
  50: "0.980 0.010 18",
  100: "0.955 0.025 20",
  200: "0.910 0.055 22",
  300: "0.850 0.100 24",
  400: "0.740 0.155 26",
  500: "0.570 0.185 27",
  600: "0.500 0.175 25",
  700: "0.420 0.155 23",
  800: "0.330 0.120 20",
  900: "0.250 0.085 18",
  950: "0.170 0.060 18",
};

/** ai — indigo blue, info (藍) */
const ai = {
  50: "0.970 0.015 250",
  100: "0.945 0.030 251",
  200: "0.905 0.060 252",
  300: "0.845 0.100 253",
  400: "0.750 0.135 254",
  500: "0.590 0.160 254",
  600: "0.510 0.155 254",
  700: "0.430 0.140 254",
  800: "0.330 0.110 254",
  900: "0.250 0.080 254",
  950: "0.180 0.065 255",
};

/** murasaki — muted purple, secondary (紫) */
const murasaki = {
  50: "0.970 0.020 300",
  100: "0.945 0.045 300",
  200: "0.905 0.085 301",
  300: "0.845 0.135 302",
  400: "0.735 0.185 302",
  500: "0.560 0.225 303",
  600: "0.480 0.210 303",
  700: "0.400 0.185 304",
  800: "0.305 0.145 304",
  900: "0.225 0.105 305",
  950: "0.180 0.085 305",
};

/** fuji — wisteria violet, accent (藤) */
const fuji = {
  50: "0.972 0.018 296",
  100: "0.948 0.038 297",
  200: "0.910 0.072 298",
  300: "0.850 0.120 299",
  400: "0.750 0.170 300",
  500: "0.575 0.205 300",
  600: "0.495 0.195 300",
  700: "0.415 0.175 300",
  800: "0.315 0.138 300",
  900: "0.235 0.105 300",
  950: "0.170 0.090 300",
};

export const sumiPalette = {
  kami,
  sumi,
  shu,
  hisui,
  kohaku,
  beni,
  ai,
  murasaki,
  fuji,
};
