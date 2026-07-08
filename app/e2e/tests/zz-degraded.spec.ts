/**
 * Verification: stop a service so a gate fails, capture the degraded health
 * screen (remedy / installing-progress) to /tmp/sensei-shots/. Restores the
 * service after. Not a regression gate. Uses ollama (safe to bounce).
 */
import { test } from '../fixtures';
import { navigateTo } from '../helpers';
import { execFileSync } from 'child_process';
import { mkdirSync } from 'fs';

const DIR = '/tmp/sensei-shots';
const settle = (ms: number) => new Promise<void>((r) => setTimeout(r, ms));
const brew = (action: string) => {
  try { execFileSync('brew', ['services', action, 'ollama'], { stdio: 'ignore' }); } catch { /* ignore */ }
};

test('capture degraded health with ollama stopped', async ({ tauriPage }) => {
  mkdirSync(DIR, { recursive: true });
  brew('stop');
  try {
    // Re-enter the health page so it re-probes with ollama down.
    await navigateTo(tauriPage, '/health?auto=false');
    await settle(7000);
    await tauriPage.screenshot({ path: `${DIR}/health-degraded.png` });
  } finally {
    brew('start');
  }
});
