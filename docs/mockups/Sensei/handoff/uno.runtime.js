/* ═══════════════════════════════════════════════════════════════════
   ZEN-SUMI · UnoCSS runtime
   ═══════════════════════════════════════════════════════════════════
   The mock and the app now share ONE utility engine. Load after
   tokens.css, on every page:

     <link rel="stylesheet" href="lib/tokens.css"/>
     <script src="lib/unocss.js"></script>

   tokens.css stays the source of truth: it owns the CSS variables, the
   reset, the named type roles (zs-h1 · zs-body · zs-meta) and the
   components (zs-btn · zs-card · zs-input · zs-badge · zs-dot).
   UnoCSS owns every utility.

   The theme below is REPLACED, not extended — Uno's default palette,
   type scale, radii and shadows are dropped and only the system's
   values are registered. So `bg-red-500`, `text-4.5xl` and
   `rounded-2xl` generate nothing at all: a dev agent that reaches past
   the system gets no CSS rather than an off-brand colour. That is the
   whole point of this file.

   The build-time equivalent for the app lives in handoff/uno.config.ts
   and MUST stay in sync with this — same keys, same token references.

   Parity with the app: it styles with UnoCSS via @rokkit/unocss's
   presetRokkit, which already resolves bg-{token} to var(--{token}) and
   inherits Uno's 4px spacing and sm/md/lg/xl breakpoints. Everything
   here is chosen to match that.
   ═══════════════════════════════════════════════════════════════════ */

(function () {
  if (window.__zsUno) return;
  window.__zsUno = true;

  // Every colour the system has, and nothing else. Values point at the
  // tokens, so dark mode keeps flipping for free.
  var COLORS = {
    transparent: 'transparent',
    current: 'currentColor',
    inherit: 'inherit',

    paper: 'var(--paper)',
    'paper-soft': 'var(--paper-soft)',
    'paper-mute': 'var(--paper-mute)',
    'paper-edge': 'var(--paper-edge)',

    ink: 'var(--ink)',
    'ink-soft': 'var(--ink-soft)',
    'ink-mute': 'var(--ink-mute)',
    'ink-faint': 'var(--ink-faint)',

    accent: 'var(--accent)',
    'accent-soft': 'var(--accent-soft)',
    'accent-edge': 'var(--accent-edge)',

    success: 'var(--success)',
    'success-soft': 'var(--success-soft)',
    'success-edge': 'var(--success-edge)',
    warning: 'var(--warning)',
    'warning-soft': 'var(--warning-soft)',
    'warning-edge': 'var(--warning-edge)',
    danger: 'var(--danger)',
    'danger-soft': 'var(--danger-soft)',
    'danger-edge': 'var(--danger-edge)',

    primary: 'var(--primary)',
    'on-primary': 'var(--on-primary)',
    'on-primary-soft': 'var(--on-primary-soft)',
    'on-primary-mute': 'var(--on-primary-mute)',
    'on-primary-faint': 'var(--on-primary-faint)',

    /* deprecated numbered aliases — older screens only */
    'paper-2': 'var(--paper-2)',
    'paper-3': 'var(--paper-3)',
    'ink-2': 'var(--ink-2)',
    'ink-3': 'var(--ink-3)',
    'ink-4': 'var(--ink-4)'
  };

  // The 8 stops. Floor is xs (11px); there is nothing below it.
  // Tuple form [size, lineHeight]: the app ships these as tuples, so a bare
  // `text-sm` must set BOTH here too — otherwise the mock inherits the
  // scope's 1.6 ratio and vertical rhythm drifts against the app.
  var FONT_SIZE = {
    xs:    ['var(--text-xs)',   '1.4'],
    sm:    ['var(--text-sm)',   '1.5'],
    base:  ['var(--text-base)', '1.6'],
    lg:    ['var(--text-lg)',   '1.5'],
    xl:    ['var(--text-xl)',   '1.2'],
    '2xl': ['var(--text-2xl)',  '1.2'],
    '3xl': ['var(--text-3xl)',  '1.2'],
    '4xl': ['var(--text-4xl)',  '1.05']
  };

  window.__unocss = {
    // Replace whole theme sections rather than merging into Uno's
    // defaults — merging would leave bg-red-500 alive.
    //
    // NOT overridden: spacing and breakpoints. Uno's defaults already
    // ARE the system's scale (4px x n; sm 640 / md 768 / lg 1024 /
    // xl 1280), so re-declaring them would be a second copy to keep in
    // sync for no gain — and would drop the fractional stops (p-0.5 =
    // 2px) that hairline-gap controls need. The --space-N tokens are
    // the same values under a name, for inline geometry only.
    extendTheme: function (theme) {
      theme.colors = COLORS;
      theme.textColor = COLORS;
      theme.backgroundColor = COLORS;
      theme.borderColor = COLORS;
      theme.fontSize = FONT_SIZE;
      theme.fontFamily = {
        display: 'var(--font-display)',
        ui: 'var(--font-ui)',
        mono: 'var(--font-mono)',
        kanji: 'var(--font-kanji)'
      };
      theme.lineHeight = {
        tight: 'var(--leading-tight)', snug: 'var(--leading-snug)',
        normal: 'var(--leading-normal)', loose: 'var(--leading-loose)'
      };
      theme.letterSpacing = {
        tight: 'var(--tracking-tight)', normal: 'var(--tracking-normal)',
        wide: 'var(--tracking-wide)'
      };
      theme.borderRadius = {
        DEFAULT: 'var(--radius)', sm: 'var(--radius-sm)',
        lg: 'var(--radius-lg)', full: 'var(--radius-full)'
      };
      theme.boxShadow = {
        DEFAULT: 'var(--shadow)', sm: 'var(--shadow-sm)', lg: 'var(--shadow-lg)'
      };
      theme.fontWeight = { light: '300', normal: '400', medium: '500', semibold: '600' };
      theme.duration = { fast: 'var(--dur-fast)', DEFAULT: 'var(--dur)', slow: 'var(--dur-slow)' };
      theme.easing = { zen: 'var(--ease)', DEFAULT: 'var(--ease)' };
      return theme;
    },

    // No shortcuts. The app ships none, and its screens already write
    // `border border-paper-edge` directly — so the two vocabularies match
    // with zero config on either side. A recipe belongs to a component
    // (<Card>, <Row>, <Button>), not to a class alias.


    // tokens.css owns the reset; Uno must not ship a second one.
    preflights: []
  };

  var s = document.createElement('script');
  s.src = 'https://cdn.jsdelivr.net/npm/@unocss/runtime';
  document.head.appendChild(s);
})();
