/**
 * #96 against the REAL daemon: the "log" toggle on a background task's row
 * opens that worker's own lines from today's daemon log, and nothing else.
 * Picks the first worker the API has lines for, so it reads what is live.
 * Captures /tmp/sensei-shots/task-log.png.
 * Run: `bun run test:inspect -- task-log`.
 */
import { expect, test } from '../fixtures';
import { DAEMON_URL, navigateToScreen, shot } from '../helpers';
import { mkdirSync } from 'fs';

const DIR = '/tmp/sensei-shots';

test('a task row opens the worker log tail', async ({ tauriPage }) => {
  mkdirSync(DIR, { recursive: true });
  const { tasks }: { tasks: { name: string }[] } = await (
    await fetch(`${DAEMON_URL}/api/tasks/scheduled`)
  ).json();
  let worker: string | undefined;
  for (const t of tasks) {
    const tail = await (await fetch(`${DAEMON_URL}/api/tasks/scheduled/${t.name}/log?lines=50`)).json();
    if (tail.lines.length > 0) {
      worker = t.name;
      break;
    }
  }
  test.skip(!worker, 'no worker has logged today');

  await navigateToScreen(tauriPage, '/activity-logs', '[data-testid="scheduled-task"]');
  const row = `[data-testid="scheduled-task"][data-task-name="${worker}"]`;
  await tauriPage.locator(`${row} [data-task-log-toggle]`).click();
  const pre = tauriPage.locator('[data-task-log] pre');
  await expect(pre).toBeVisible({ timeout: 20_000 });
  const lines = (await pre.innerText()).split('\n').filter(Boolean);
  console.log(`\n[inspect] ${worker}: ${lines.length} lines; last: ${lines.at(-1)?.slice(0, 160)}`);
  await shot(tauriPage, `${DIR}/task-log.png`);
  expect(lines.length).toBeGreaterThan(0);
  for (const l of lines) expect(l).toContain(`worker="${worker}"`);
});
