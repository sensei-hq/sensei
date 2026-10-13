import { createTauriTest, type TauriFixtures } from '@srsholmes/tauri-playwright';

/** The page a test's `tauriPage` fixture hands out — what a helper takes. */
export type TauriPageLike = TauriFixtures['tauriPage'];

export const { test, expect } = createTauriTest({
  devUrl: 'tauri://localhost', // required by TauriTestConfig type; unused in tauri socket mode
  mcpSocket: '/tmp/tauri-playwright.sock',
});
