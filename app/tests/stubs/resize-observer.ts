// Vitest setup: polyfill `ResizeObserver` for the jsdom environment.
//
// jsdom does not implement it. @rokkit/chart's self-sizing wrappers (RadarChart
// and friends) construct one to track their container, so merely mounting one
// throws `ResizeObserver is not defined` under jsdom. Real browsers and the app
// build have it, so this stub is test-only.
//
// Deliberately inert — it never fires a callback. Components under test are
// given explicit `width`/`height`, so they lay out from those props rather than
// from an observed box; a stub that invented sizes would make assertions depend
// on a fabricated layout. Note the app's own `ChartCanvas` avoids ResizeObserver
// entirely (scale-free viewBox) for the same reason, so this only matters for
// the library's self-sizing chart wrappers.
//
// Guarded on `window` so the node-environment lib tests (no DOM) are untouched.
if (typeof window !== 'undefined' && typeof window.ResizeObserver !== 'function') {
  class InertResizeObserver implements ResizeObserver {
    observe(): void {}
    unobserve(): void {}
    disconnect(): void {}
  }
  window.ResizeObserver = InertResizeObserver as unknown as typeof ResizeObserver;
  globalThis.ResizeObserver = InertResizeObserver as unknown as typeof ResizeObserver;
}
