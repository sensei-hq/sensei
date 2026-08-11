/**
 * Inspect the REAL project Metrics pane for live projects (sensei, dbd, rokkit).
 * Not a regression gate — captures /tmp/sensei-shots/metrics-<project>.png and
 * logs the rendered cards so the wired /project/[id]/metrics screen can be
 * eyeballed against live data. Run: `bun run test:inspect`.
 */
import { test } from '../fixtures';
import { navigateToScreen, DAEMON_URL } from '../helpers';
import { mkdirSync } from 'fs';

const DIR = '/tmp/sensei-shots';
const PROJECTS = ['sensei', 'dbd', 'rokkit'];
const settle = (ms = 2500) => new Promise<void>((r) => setTimeout(r, ms));

type Project = { id: string; name: string };

async function idForName(name: string): Promise<string | null> {
  const res = await fetch(`${DAEMON_URL}/api/projects`);
  const body = await res.json();
  const list: Project[] = Array.isArray(body) ? body : (body.projects ?? []);
  return list.find((p) => p.name === name)?.id ?? null;
}

test.describe('inspect real project metrics', () => {
  for (const name of PROJECTS) {
    test(`metrics — ${name}`, async ({ tauriPage }) => {
      mkdirSync(DIR, { recursive: true });

      const id = await idForName(name);
      if (!id) {
        test.skip(true, `no project named "${name}" on the real daemon`);
        return;
      }

      await navigateToScreen(
        tauriPage,
        `/project/${id}/metrics`,
        '[data-component="project-shell"]',
      );
      await settle();

      const cards = (await tauriPage.evaluate(`
        Array.from(document.querySelectorAll('[data-component="metric-card"]')).map(c => ({
          name: c.querySelector('span.text-xs')?.textContent?.trim(),
          value: c.querySelector('[data-component="metric-value"]')?.textContent?.trim(),
          trend: c.querySelector('[data-component="metric-trend"]')?.textContent?.replace(/\\s+/g,' ').trim() || null,
        }))
      `)) as Array<{ name?: string; value?: string; trend: string | null }>;

      console.log(`\n[inspect] ${name} — ${cards.length} metric cards:`);
      for (const c of cards) console.log(`  ${(c.value ?? '').padEnd(9)} ${c.name ?? ''}${c.trend ? '  ' + c.trend : ''}`);

      await tauriPage.screenshot({ path: `${DIR}/metrics-${name}.png` });
    });
  }
});
