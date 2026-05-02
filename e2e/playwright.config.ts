import { defineConfig, devices } from '@playwright/test';

export default defineConfig({
  testDir: './tests',
  timeout: 30_000,
  retries: 0,
  // Tauri tests share a single WebView — parallel workers cause navigation
  // races (one worker's goto() cancels the other's). One worker = safe.
  workers: 1,
  projects: [
    {
      name: 'browser',
      use: { ...devices['Desktop Chrome'] },
    },
    {
      name: 'tauri',
      use: { mode: 'tauri' },
    },
  ],
  webServer: {
    command: 'npx vite dev',
    port: 5173,
    reuseExistingServer: true,
  },
});
