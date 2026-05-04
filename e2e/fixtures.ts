import { createTauriTest } from '@srsholmes/tauri-playwright';

export const { test, expect } = createTauriTest({
  mcpSocket: '/tmp/tauri-playwright.sock',
});
