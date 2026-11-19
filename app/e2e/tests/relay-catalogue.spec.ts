/**
 * AddMember picks from what this machine has seen (#455; relay-crew spec "AddMember picks
 * from what this machine has seen"). The e2e database holds a Claude Code session and a
 * Cursor-hosted session with its model, and Cursor's install row says sensei is set up in
 * the isolated home's `.cursor/mcp.json`. On the Crew view, with no reload:
 *   - Assistants lists Claude Code with its sessions this week;
 *   - Editors lists Cursor with the model and the config path;
 *   - picking Cursor words the note for Cursor's agent chat and names the config;
 *   - picking Claude Code words it for a Claude Code session;
 *   - the paste line never changes.
 * Only what the spec seeded is asserted; nothing is asserted absent (the probe sees
 * /Applications, which the isolated home does not hide).
 */
import { test, expect, type TauriPageLike } from '../fixtures';
import { navigateToScreen, seedSetupComplete, DAEMON_URL } from '../helpers';
import { E2E_HOME, seedSenseiMcp } from '../isolated-home';
import { CATALOGUE_MODEL, seedCatalogue } from '../lib/seed-catalogue';

const STREAM = 20_000;
let tauri: TauriPageLike;
let claudeSessions = 0;

const item = (id: string) => `[data-catalogue-item="${id}"]`;
const textOf = (selector: string) =>
  tauri.evaluate(
    `(document.querySelector(${JSON.stringify(selector)})?.textContent ?? '').replace(/\\s+/g, ' ').trim()`,
  ) as Promise<string>;
const attrOf = (selector: string, name: string) =>
  tauri.evaluate(`document.querySelector(${JSON.stringify(selector)})?.getAttribute(${JSON.stringify(name)}) ?? null`) as Promise<string | null>;

test.beforeAll(async () => {
  seedSenseiMcp();
  claudeSessions = await seedCatalogue(DAEMON_URL, E2E_HOME);
});

test('AddMember lists the assistants and editors this machine has seen, and picking one words the note', async ({ tauriPage }) => {
  test.setTimeout(180_000);
  tauri = tauriPage;
  await seedSetupComplete(tauriPage);
  await navigateToScreen(tauriPage, '/relay', '[data-screen="relay"] [data-screen-tools]');
  await tauriPage.locator('[data-screen-tools] [aria-label^="Crew"]').click();
  await expect(tauriPage.locator('[data-add-member]')).toBeVisible({ timeout: STREAM });

  // Assistants: Claude Code with its sessions this week.
  const claude = `[data-catalogue-group="assistants"] ${item('claude')}`;
  await expect(tauriPage.locator(claude)).toBeVisible({ timeout: STREAM });
  const plural = claudeSessions === 1 ? 'session' : 'sessions';
  expect(await textOf(claude)).toContain('Claude Code');
  expect(await textOf(claude)).toContain(`${claudeSessions} ${plural} this week`);

  // Editors: Cursor with the model and where sensei's tools were found.
  const cursor = `[data-catalogue-group="editors"] ${item('cursor')}`;
  await expect(tauriPage.locator(cursor)).toBeVisible({ timeout: STREAM });
  expect(await textOf(cursor)).toContain('Cursor');
  expect(await textOf(cursor)).toContain(CATALOGUE_MODEL);
  expect(await textOf(cursor)).toContain('.cursor/mcp.json');

  const line = await textOf('[data-paste-text]');
  expect(line).toContain('register with sensei as');

  // Picking Cursor: pressed, and the note names its agent chat and the config.
  await tauriPage.locator(cursor).click();
  await expect.poll(() => attrOf(cursor, 'aria-pressed'), { timeout: STREAM }).toBe('true');
  await expect.poll(() => textOf('[data-command-note]'), { timeout: STREAM }).toContain("Paste it into Cursor's agent chat.");
  expect(await textOf('[data-command-note]')).toContain('.cursor/mcp.json');
  expect(await textOf('[data-paste-text]')).toBe(line);

  // Picking Claude Code: the note names a Claude Code session.
  await tauriPage.locator(claude).click();
  await expect.poll(() => attrOf(claude, 'aria-pressed'), { timeout: STREAM }).toBe('true');
  await expect.poll(() => textOf('[data-command-note]'), { timeout: STREAM }).toContain('Paste it into a Claude Code session');
  expect(await textOf('[data-paste-text]')).toBe(line);
});
