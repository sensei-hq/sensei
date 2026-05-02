/**
 * Tauri Playwright test fixtures.
 *
 * Browser mode: headless Chromium, mocked Tauri IPC — fast, for CI.
 * Tauri mode: real webview + real daemon — full E2E, for local.
 */

import { createTauriTest } from '@srsholmes/tauri-playwright';

export const { test, expect } = createTauriTest({
  devUrl: 'http://localhost:5173',
  ipcMocks: {
    // Bootstrap commands — return "all ready" for tests
    run_bootstrap: () => ({
      components: [
        { name: 'homebrew', state: { state: 'ready' }, version: '4.0', detail: null },
        { name: 'postgresql', state: { state: 'ready' }, version: '16', detail: null },
        { name: 'ollama', state: { state: 'ready' }, version: '0.3', detail: null },
        { name: 'sensei', state: { state: 'ready' }, version: '0.1', detail: null },
        { name: 'database', state: { state: 'ready' }, version: null, detail: null },
        { name: 'daemon', state: { state: 'ready' }, version: '0.1', detail: null },
      ],
    }),
    get_platform: () => ({
      platform: 'macos',
      package_manager: 'homebrew',
      prereq_remedy: { title: 'Install via Homebrew', command: 'brew install', url: null },
      pkgmgr_remedy: { title: 'Install Homebrew', command: '/bin/bash -c ...', url: 'https://brew.sh' },
    }),
    detect_hardware: () => ({
      ram_gb: 16, cpu_cores: 8, gpu: 'Apple M2', metal_support: true,
      recommended_tier: 'recommended',
    }),
  },
  mcpSocket: '/tmp/tauri-playwright.sock',
});
