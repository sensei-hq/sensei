// Cold-start playwright config — separate from playwright.config.ts so
// the cold-path setup (drop DB, stop services, no daemon) does not
// interfere with the standard E2E suite.
//
// Usage: bun run test:e2e:cold

import { defineConfig } from '@playwright/test';

export default defineConfig({
  testDir: './tests-cold',
  // Resolvers run real brew install/start commands — total time can be
  // 2-3 minutes on a fresh dev box. Per-test budget reflects that.
  timeout: 300_000,
  retries: 0,
  // WKWebView shares one window — parallel workers would race on the same UI.
  workers: 1,
  globalSetup:    './globalSetup-cold.ts',
  globalTeardown: './globalTeardown-cold.ts',
  projects: [
    { name: 'cold-start', use: { mode: 'tauri' } },
  ],
});
