import { createTauriTest } from '@srsholmes/tauri-playwright';

export const { test, expect } = createTauriTest({
  devUrl: 'tauri://localhost', // required by TauriTestConfig type; unused in tauri socket mode
  mcpSocket: '/tmp/tauri-playwright.sock',
});
