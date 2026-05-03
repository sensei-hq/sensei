/**
 * Test fixtures for the DB-missing bootstrap scenario.
 *
 * Stateful mocks: run_bootstrap returns DB failed on first call;
 * after setup_database is invoked, subsequent calls return DB ready.
 * This lets the bootstrap page cycle through fix → re-check → advance.
 */

import { createTauriTest } from '@srsholmes/tauri-playwright';

let setupComplete = false;

export function resetSetupState() {
  setupComplete = false;
}

export const { test, expect } = createTauriTest({
  devUrl: 'http://localhost:5173',
  ipcMocks: {
    run_bootstrap: () => {
      const dbState = setupComplete
        ? { state: 'ready' }
        : { state: 'failed', error: "database 'sensei-dev' does not exist" };
      return {
        components: [
          { name: 'homebrew',   state: { state: 'ready' }, version: '4.0',   detail: null },
          { name: 'postgresql', state: { state: 'ready' }, version: '16',    detail: null },
          { name: 'ollama',     state: { state: 'ready' }, version: '0.3',   detail: null },
          { name: 'sensei',     state: { state: 'ready' }, version: '0.1.0', detail: null },
          { name: 'database',   state: dbState,            version: null,     detail: null },
          { name: 'daemon',     state: { state: 'ready' }, version: '0.1.0', detail: null },
        ],
      };
    },
    setup_database: () => {
      setupComplete = true;
      return null;
    },
    install_prerequisites: () => null,
    start_services: () => null,
    get_platform: () => ({
      platform: 'macos',
      package_manager: 'homebrew',
      prereq_remedy: { title: 'Install via Homebrew', command: 'brew install', url: null },
      pkgmgr_remedy: { title: 'Install Homebrew', command: '/bin/bash -c ...', url: 'https://brew.sh' },
    }),
    detect_hardware: () => ({
      ram_gb: 16, cpu_cores: 8, gpu: 'Apple M2', metal_support: true, recommended_tier: 'recommended',
    }),
  },
  mcpSocket: '/tmp/tauri-playwright.sock',
});
