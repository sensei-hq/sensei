declare global {
  namespace App {
    // No server-side locals — static site
  }

  // Injected by Vite `define` (vite.config.ts) — app version from package.json.
  const __APP_VERSION__: string;
}

// @rokkit/states ships JS source; its package `types` points at an unpublished
// dist/, so it resolves untyped here. Shim until rokkit ships declarations.
declare module '@rokkit/states';

// @rokkit/ui's ButtonProps lacks `rel`; `rel="noopener"` is required on
// target="_blank" links. Augment until rokkit adds it upstream.
declare module '@rokkit/ui' {
  interface ButtonProps {
    rel?: string;
  }
}

export {};
