// Vitest setup: polyfill `window.matchMedia` for the jsdom environment.
//
// jsdom does not implement `window.matchMedia`. @rokkit/chart's barrel eagerly
// loads `AnimatedPlot`, whose `svelte/motion` import constructs a `MediaQuery`
// (a `prefers-reduced-motion` listener) at module load — so merely importing a
// chart component throws `matchMedia is not a function` under jsdom. Real
// browsers and the app build have `matchMedia`, so this stub is test-only.
//
// Guarded on `window` so the node-environment lib tests (no DOM) are untouched.
if (typeof window !== 'undefined' && typeof window.matchMedia !== 'function') {
  window.matchMedia = (query: string): MediaQueryList =>
    ({
      matches: false,
      media: query,
      onchange: null,
      addEventListener: () => {},
      removeEventListener: () => {},
      addListener: () => {},
      removeListener: () => {},
      dispatchEvent: () => false,
    }) as unknown as MediaQueryList;
}
