/**
 * Tauri E2E test config — PLACEHOLDER
 *
 * tauri-driver is not yet supported on macOS 26 (Tahoe).
 * See bootstrap-integration.rs for Rust-level sidecar tests.
 *
 * When tauri-driver adds macOS 26 support, re-enable this config.
 * Track: https://github.com/tauri-apps/tauri/issues (tauri-driver)
 */

export const config: WebdriverIO.Config = {
  runner: 'local',
  specs: [],
  capabilities: [],
  logLevel: 'warn',
  framework: 'mocha',
  mochaOpts: { ui: 'bdd', timeout: 60_000 },
};
