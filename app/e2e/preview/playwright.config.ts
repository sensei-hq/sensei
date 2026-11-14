import { defineConfig } from '@playwright/test';

// The component preview capture: plain Chromium over `vite dev`, no Tauri, no
// daemon data — the preview renders fixtures. Its own port, so a running
// `make app-dev` (5173, strictPort) is left alone. Output goes outside the
// repo (PREVIEW_OUT, default coord/reports/relay-screen): no checked-in PNGs.
export default defineConfig({
  testDir: '.',
  testMatch: /capture\.spec\.ts/,
  timeout: 180_000,
  retries: 0,
  workers: 1,
  reporter: [['list']],
  use: {
    baseURL: 'http://127.0.0.1:5177',
    viewport: { width: 1440, height: 1000 },
    deviceScaleFactor: 2,
  },
  projects: [{ name: 'preview', use: { browserName: 'chromium' } }],
  webServer: {
    command: 'bun run dev -- --port 5177 --strictPort',
    url: 'http://127.0.0.1:5177/preview',
    reuseExistingServer: true,
    timeout: 120_000,
  },
});
