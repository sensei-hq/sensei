/**
 * #280 against the REAL daemon: with one assistant's conversation-history
 * reading off, sensei's metrics screen names it and what stops updating; with
 * every assistant read, it says nothing. Turns Zed off for the check and puts it
 * back in `finally`. Captures /tmp/sensei-shots/metrics-history-off.png.
 * Run: `bun run test:inspect -- metrics-history`.
 */
import { expect, test } from '../fixtures';
import { DAEMON_URL, navigateToScreen, shot } from '../helpers';
import { mkdirSync } from 'fs';

const DIR = '/tmp/sensei-shots';
const NOTICE = '[data-component="metrics-history-off"]';

async function setConsent(source: string, consented: boolean) {
  const res = await fetch(`${DAEMON_URL}/api/transcripts/consent/${source}`, {
    method: 'PUT',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ consented }),
  });
  expect(res.ok, `PUT consent ${source}=${consented}`).toBe(true);
}

test('metrics name the assistants whose history is not read', async ({ tauriPage }) => {
  mkdirSync(DIR, { recursive: true });
  const workspaces: { id: string; name: string }[] = await (await fetch(`${DAEMON_URL}/api/workspaces`)).json();
  const sensei = workspaces.find((p) => p.name === 'sensei');
  test.skip(!sensei, 'no sensei workspace on this daemon');
  const before: { source: string; consented: boolean }[] = await (
    await fetch(`${DAEMON_URL}/api/transcripts/consent`)
  ).json();
  const zedWasOn = before.find((s) => s.source === 'zed')?.consented ?? true;

  try {
    await setConsent('zed', false);
    await navigateToScreen(tauriPage, `/workspace/${sensei!.id}/metrics`, '[data-component="workspace-shell"]');
    await expect(tauriPage.locator(NOTICE)).toBeVisible({ timeout: 20_000 });
    const text = await tauriPage.locator(NOTICE).innerText();
    console.log(`\n[inspect] metrics notice: ${text}`);
    await shot(tauriPage, `${DIR}/metrics-history-off.png`);
    expect(text).toContain('Zed');
    expect(text).toContain('its token, cost and session metrics are not updating');

    await setConsent('zed', true);
    const allOn = before.every((s) => s.source === 'zed' || s.consented);
    if (allOn) {
      // Back through Settings, as a person would after switching it on — a load
      // only re-runs on a route change.
      await navigateToScreen(tauriPage, '/settings/assistants', '[data-testid="transcript-consent"]');
      await navigateToScreen(tauriPage, `/workspace/${sensei!.id}/metrics`, '[data-component="workspace-shell"]');
      await new Promise((r) => setTimeout(r, 2000));
      expect(await tauriPage.locator(NOTICE).count(), 'no notice when every assistant is read').toBe(0);
    }
  } finally {
    await setConsent('zed', zedWasOn);
  }
});
