/**
 * The lane's host is detected; the self-report is the fallback (#455; relay-crew spec
 * "The host is checked against the session's environment"). The MCP binary sends
 * `host_detected` at register; the RelayCrew driver plays that over the lane HTTP API.
 * On the Crew view, with no reload:
 *   - a lane with only `host_detected: 'zed'` reads "in Zed";
 *   - a lane reporting `host: 'cursor'` that was detected in Zed reads "in Zed";
 *   - a lane with only `host: 'cursor'` (plain HTTP, nothing detected) reads "in Cursor";
 *   - the coordinator, a terminal lane, shows no tag.
 */
import { test, expect, type TauriPageLike } from '../fixtures';
import { navigateToScreen, seedSetupComplete, DAEMON_URL } from '../helpers';
import { RelayCrew, type RelayLane } from '../lib/relay-crew';
import { cleanSlate } from '../lib/relay-slate';

const STREAM = 20_000;
// Unique per run: names on the shared e2e broker must not collide with an earlier run's.
const TAG = Date.now().toString(36);
const NAMES = { coord: `atlas-${TAG}`, zed: `zed-${TAG}`, both: `both-${TAG}`, cursor: `cursor-${TAG}` };
const crew = new RelayCrew(DAEMON_URL);
const lanes: RelayLane[] = [];
let tauri: TauriPageLike;

/** Each member card's text and its host tag (null when it has none), read in the page. */
async function cards(): Promise<Array<{ text: string; host: string | null }>> {
  const json = (await tauri.evaluate(`JSON.stringify([...document.querySelectorAll('[data-member]')].map((card) => ({
    text: card.textContent.replace(/\\s+/g, ' ').trim(),
    host: card.querySelector('[data-member-host]')?.textContent.replace(/\\s+/g, ' ').trim() ?? null,
  })))`)) as string;
  return JSON.parse(json);
}

/** The host tag of the card naming `name`; undefined while the card is not in yet. */
async function hostOf(name: string): Promise<string | null | undefined> {
  return (await cards()).find((c) => c.text.includes(name))?.host;
}

test.beforeAll(async () => {
  await cleanSlate();
});

test.afterAll(async () => {
  for (const lane of lanes) await lane.unregister().catch(() => {}); // skipcq - afterAll cleanup is best-effort; failures surface in the owning test
});

test('a detected host wins over the reported one, and the report is the fallback', async ({ tauriPage }) => {
  test.setTimeout(180_000);
  tauri = tauriPage;
  await seedSetupComplete(tauriPage);
  await navigateToScreen(tauriPage, '/relay', '[data-screen="relay"] [data-screen-tools]');
  await tauriPage.locator('[data-screen-tools] [aria-label^="Crew"]').click();

  const coord = await crew.register({ role: 'coordinator', family: 'e2e', goal: 'host detection crew', suggested_name: NAMES.coord });
  lanes.push(coord);
  const join = (suggested_name: string, hosts: { host?: string; host_detected?: string }) =>
    crew.register({ family: 'e2e', crew_id: coord.crewId, suggested_name, ...hosts });
  lanes.push(await join(NAMES.zed, { host_detected: 'zed' }));
  lanes.push(await join(NAMES.both, { host: 'cursor', host_detected: 'zed' }));
  lanes.push(await join(NAMES.cursor, { host: 'cursor' }));

  // Every card arrives from the stream, with its tag.
  await expect.poll(() => hostOf(NAMES.zed), { timeout: STREAM }).toBe('in Zed');
  await expect.poll(() => hostOf(NAMES.both), { timeout: STREAM }).toBe('in Zed');
  await expect.poll(() => hostOf(NAMES.cursor), { timeout: STREAM }).toBe('in Cursor');
  // The coordinator runs in a terminal: no tag.
  await expect.poll(() => hostOf(NAMES.coord), { timeout: STREAM }).toBeNull();
});
