/**
 * #288 against the REAL daemon: this repository's project Instruments header
 * and Memories list show the daemon's reads, with no error state.
 * Run: `bun run test:inspect -- project-instruments-memories`.
 *
 * The page is read from the DOM rather than waited on with `waitForSelector`:
 * when an identity awaits the user (#283) the sign-in overlay is modal, and the
 * bridge's visibility check never sees the page beneath it — which still renders.
 */
import { expect, test } from '../fixtures';
import { DAEMON_URL, navigateTo } from '../helpers';

const sectionIs = (s: string) => `document.querySelector('[data-component="project-main"]')?.getAttribute('data-section') === '${s}'`;

test('project instruments and memories show the daemon reads', async ({ tauriPage }) => {
  const projects: { id: string; name: string }[] = await (await fetch(`${DAEMON_URL}/api/projects`)).json();
  const sensei = projects.find((p) => p.name === 'sensei');
  test.skip(!sensei, 'no sensei project on this daemon');
  const id = sensei!.id;
  const tools = (await (await fetch(`${DAEMON_URL}/api/projects/${id}/mcp-tool-stats`)).json()).tools.length;
  const memories = (await (await fetch(`${DAEMON_URL}/api/projects/${id}/memories`)).json()).active.length;
  const used = (await (await fetch(`${DAEMON_URL}/api/projects/${id}/tools`)).json()).tools.length;

  await navigateTo(tauriPage, `/project/${id}/instruments`);
  await expect.poll(() => tauriPage.evaluate(sectionIs('instruments')), { timeout: 30_000 }).toBe(true);
  await new Promise((r) => setTimeout(r, 1500));
  const inst = (await tauriPage.evaluate(`({
    error: document.querySelector('[data-screen-state="error"]')?.textContent?.trim() ?? null,
    tools: document.querySelector('[data-component="project-main"]')?.textContent?.match(/(\\d+) tools/)?.[1] ?? null,
    used: document.querySelectorAll('[data-testid="tools-used"] [data-tool]').length,
  })`)) as { error: string | null; tools: string | null; used: number };

  await navigateTo(tauriPage, `/project/${id}/memories`);
  await expect.poll(() => tauriPage.evaluate(sectionIs('memories')), { timeout: 30_000 }).toBe(true);
  await new Promise((r) => setTimeout(r, 1500));
  const mem = (await tauriPage.evaluate(`({
    error: document.querySelector('[data-screen-state="error"]')?.textContent?.trim() ?? null,
    rows: document.querySelectorAll('.memory-row').length,
  })`)) as { error: string | null; rows: number };

  // #293: selecting a memory and opening its drawer run through the screen's
  // state now. Read-only — nothing is proposed.
  const interacted = mem.rows > 0
    ? (await tauriPage.evaluate(`(async () => {
        const row = document.querySelector('.memory-row');
        row.querySelector('input[type="checkbox"]').click();
        row.querySelector('[data-testid^="memory-open-"]').click();
        await new Promise((r) => setTimeout(r, 300));
        return {
          selected: document.body.textContent.includes('1 selected'),
          drawer: document.body.textContent.includes('What:'),
        };
      })()`)) as { selected: boolean; drawer: boolean }
    : { selected: true, drawer: true };

  console.log(`\n[inspect] instruments ${JSON.stringify(inst)} (daemon ${tools} tools, ${used} used); memories ${JSON.stringify(mem)} (daemon ${memories}) ${JSON.stringify(interacted)}`);
  expect(interacted).toEqual({ selected: true, drawer: true });
  expect([inst.error, mem.error]).toEqual([null, null]);
  expect(inst.tools).toBe(String(tools));
  // #291: the "Tools used" bubbles — one per tool the daemon reports.
  expect(inst.used).toBe(used);
  expect(mem.rows).toBe(memories);
});
