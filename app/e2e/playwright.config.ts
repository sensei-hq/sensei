import { defineConfig } from '@playwright/test';
import type { TauriFixtures } from '@srsholmes/tauri-playwright';

export default defineConfig<Pick<TauriFixtures, 'mode'>>({
  testDir: './tests',
  timeout: 60_000,
  retries: 0,
  // WKWebView shares one window — parallel workers would race on the same UI.
  workers: 1,
  globalSetup:    './globalSetup.ts',
  globalTeardown: './globalTeardown.ts',
  projects: [
    { name: 'tauri', use: { mode: 'tauri' } },
  ],
  // No webServer — Vite is not used; app is pre-built by globalSetup.
});
