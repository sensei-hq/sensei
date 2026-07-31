// ═══════════════════════════════════════════════════════════════════
// Zen-Sumi · font manifest
// ═══════════════════════════════════════════════════════════════════
// The single machine-readable statement of what typography this design
// ships — a HANDOFF ARTIFACT, not a runtime dependency. Nothing in the
// mock loads it; the pages load fonts.css, which is what actually
// declares the faces.
//
// Three places must agree, and this is the one to change first:
//   1. this manifest
//   2. fonts.css        — the @font-face blocks that load them
//   3. lib/unocss.js    — theme.fontWeight, which makes them writable
// A weight in fewer than three places is a bug.

(function () {
  var FONTS = [
    {
      token: '--font-ui',
      utility: 'font-ui',
      family: 'Inter',
      pkg: '@fontsource/inter',
      version: '5',
      role: 'Body text and chrome. The default for everything unscoped.',
      weights: [300, 400, 500, 600],
      styles: ['normal', 'italic'],
      // italic is shipped at 400 only
      italicWeights: [400],
      subset: 'latin',
      variable: false,
      files: [
        'fonts/google/inter/files/inter-latin-300-normal.woff2',
        'fonts/google/inter/files/inter-latin-400-normal.woff2',
        'fonts/google/inter/files/inter-latin-400-italic.woff2',
        'fonts/google/inter/files/inter-latin-500-normal.woff2',
        'fonts/google/inter/files/inter-latin-600-normal.woff2'
      ],
      notes: 'Weight 700 is vendored but deliberately not loaded — the scale stops at 600.'
    },
    {
      token: '--font-display',
      utility: 'font-display',
      family: 'Fraunces',
      pkg: '@fontsource-variable/fraunces',
      version: '5',
      role: 'Headings and hero type. Always tight tracking.',
      weights: [300, 400, 500, 600],   // one variable file spans the range
      styles: ['normal', 'italic'],
      italicWeights: [300, 400, 500, 600],
      subset: 'latin',
      variable: true,
      axes: ['opsz'],
      files: [
        'fonts/variable/fraunces/files/fraunces-latin-opsz-normal.woff2',
        'fonts/variable/fraunces/files/fraunces-latin-opsz-italic.woff2'
      ],
      notes: 'Variable: a single request covers 300–600. Uses font-feature-settings "ss01" via the .display role.'
    },
    {
      token: '--font-mono',
      utility: 'font-mono',
      family: 'JetBrains Mono',
      pkg: '@fontsource/jetbrains-mono',
      version: '5',
      role: 'Numbers, ids, paths, keyboard hints, versions. Never paragraphs.',
      weights: [400, 500],
      styles: ['normal'],
      italicWeights: [],
      subset: 'latin',
      variable: false,
      files: [
        'fonts/google/jetbrains-mono/files/jetbrains-mono-latin-400-normal.woff2',
        'fonts/google/jetbrains-mono/files/jetbrains-mono-latin-500-normal.woff2'
      ],
      notes: 'Tabular numerals are switched on by the .mono type role, not by the face.'
    },
    {
      token: '--font-kanji',
      utility: 'font-kanji',
      family: 'Shippori Mincho',
      pkg: '@fontsource/shippori-mincho',
      version: '5',
      role: 'The small functional kanji marks. Never below ~14px — brush detail dies.',
      weights: [400],
      styles: ['normal'],
      italicWeights: [],
      subset: 'japanese',
      variable: false,
      files: [],                 // ← not vendored in the mock
      vendored: false,
      notes: 'NOT loaded in the mock: falls back to the OS Mincho stack (Yu Mincho / Hiragino Mincho ProN / Songti SC). The app should install this package — it is the intended face. This is the one known typography gap between mock and app.'
    }
  ];

  // The complete set of weights any utility may use. Mirrors
  // theme.fontWeight in lib/unocss.js.
  var SHIPPED_WEIGHTS = { light: 300, normal: 400, medium: 500, semibold: 600 };

  // Deliberately no runtime "are the fonts loaded?" helper.
  // document.fonts.check() answers "is this face available for text on
  // the page", not "did this file load" — a browser never fetches a
  // declared face until something renders in it, so any such assert
  // reports every unused weight as missing. To verify a src path is
  // right, render a sample in each weight and compare metrics, or check
  // the network panel.

  var API = { fonts: FONTS, shippedWeights: SHIPPED_WEIGHTS };
  if (typeof window !== 'undefined') window.ZS_FONTS = API;
  if (typeof module !== 'undefined' && module.exports) module.exports = API;
})();
