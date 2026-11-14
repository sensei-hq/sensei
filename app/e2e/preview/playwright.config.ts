import { defineConfig } from '@playwright/test';

// The component preview rig — the capture matrix and the scroll test: plain Chromium over `vite dev`, no Tauri, no
// daemon data — the preview renders fixtures. Its own port, so a running
// `make app-dev` (5173, strictPort) is left alone. Output goes outside the
// repo (PREVIEW_OUT, default coord/reports/relay-screen): no checked-in PNGs.
export default defineConfig({
  testDir: '.',
  timeout: 180_000,
  retries: 0,
  workers: 1,
  reporter: [['list']],
  use: {
    // localhost, not 127.0.0.1: vite may bind IPv6 only.
    baseURL: 'http://localhost:5177',
    viewport: { width: 1440, height: 1000 },
    deviceScaleFactor: 2,
  },
  projects: [
    { name: 'preview', testMatch: /capture\.spec\.ts/, use: { browserName: 'chromium' } },
    { name: 'scroll', testMatch: /scroll\.spec\.ts/, use: { browserName: 'chromium' } },
  ],
  webServer: {
    command: 'bun run dev -- --port 5177 --strictPort',
    url: 'http://localhost:5177/preview',
    reuseExistingServer: true,
    timeout: 120_000,
  },
});
