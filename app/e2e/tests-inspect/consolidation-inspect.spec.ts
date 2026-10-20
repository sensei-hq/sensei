/**
 * #293 against the REAL daemon: the Consolidation screen shows what the daemon
 * holds — the empty state when nothing was ever consolidated, else that
 * ruleset's status — and its actions are wired and idle. Nothing is run: a
 * consolidation calls a model and writes a proposed ruleset.
 * Run: `bun run test:inspect -- consolidation`.
 */
import { expect, test } from '../fixtures';
import { DAEMON_URL, navigateTo } from '../helpers';

test('consolidation shows the daemon ruleset with idle actions', async ({ tauriPage }) => {
  const ruleset = await (await fetch(`${DAEMON_URL}/api/knowledge/rules/consolidated`)).json();

  await navigateTo(tauriPage, '/consolidation');
  await expect
    .poll(() => tauriPage.evaluate(`document.querySelectorAll('[data-action="consolidate"]').length`), { timeout: 30_000 })
    .toBeGreaterThan(0);
  const seen = (await tauriPage.evaluate(`({
    error: document.querySelector('[data-screen-state="error"]')?.textContent?.trim() ?? null,
    idle: [...document.querySelectorAll('[data-action]')].every((b) => !b.disabled),
    approve: document.querySelectorAll('[data-action="approve"]').length,
    notice: document.querySelector('[data-notice]')?.textContent ?? null,
    text: document.body.textContent,
  })`)) as { error: string | null; idle: boolean; approve: number; notice: string | null; text: string };

  console.log(`\n[inspect] consolidation: daemon ${ruleset ? `${ruleset.status} v${ruleset.version}` : 'none'}; screen error ${seen.error}, idle ${seen.idle}, approve ${seen.approve}, notice ${seen.notice}`);
  expect(seen.error).toBeNull();
  expect(seen.idle).toBe(true);
  expect(seen.notice).toBeNull();
  // Approve is offered for a proposed ruleset only.
  expect(seen.approve).toBe(ruleset?.status === 'proposed' ? 1 : 0);
  if (ruleset) expect(seen.text).toContain(`v${ruleset.version}`);
});
