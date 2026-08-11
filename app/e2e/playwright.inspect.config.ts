import { defineConfig } from '@playwright/test';

// Read-only "inspect" harness: launches the built app against the REAL `sensei`
// DB (via the running brew daemon) so live project data can be screenshotted /
// inspected. Run: `bun run test:inspect` (needs `make app-e2e-build` + a running
// daemon). Distinct from playwright.config.ts, which isolates to `sensei_e2e`.
export default defineConfig({
  testDir: './tests-inspect',
  timeout: 120_000,
  retries: 0,
  workers: 1,
  globalSetup: './globalSetup-inspect.ts',
  globalTeardown: './globalTeardown-inspect.ts',
  projects: [{ name: 'inspect', use: { mode: 'tauri' } }],
});
