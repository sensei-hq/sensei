/**
 * #288 against the REAL daemon: Libraries lists every library the daemon
 * reports, and a selected library's usage is the daemon's — no error state.
 * Read from the DOM: the sign-in overlay (#283) is modal while it is up.
 * Run: `bun run test:inspect -- libraries`.
 */
import { expect, test } from '../fixtures';
import { DAEMON_URL, navigateTo } from '../helpers';

test('libraries shows the daemon reads', async ({ tauriPage }) => {
  const { libs } = await (await fetch(`${DAEMON_URL}/api/libs?`)).json();
  await navigateTo(tauriPage, '/libraries');
  await expect
    .poll(() => tauriPage.evaluate(`document.querySelectorAll('.lib-card').length + document.querySelectorAll('[data-screen-state="error"]').length`), { timeout: 30_000 })
    .toBeGreaterThan(0);
  const cards = (await tauriPage.evaluate(`document.querySelectorAll('.lib-card').length`)) as number;
  const error = (await tauriPage.evaluate(`document.querySelector('[data-screen-state="error"]')?.textContent ?? null`)) as string | null;

  // The most-used library: click it, then compare its usage rows.
  const top = [...libs].sort((a: { repoCount: number }, b: { repoCount: number }) => b.repoCount - a.repoCount)[0];
  const usage = (await (await fetch(`${DAEMON_URL}/api/libs/${encodeURIComponent(top.id)}/usage`)).json()).usage.length;
  await tauriPage.evaluate(`[...document.querySelectorAll('.lib-card')].find((c) => c.querySelector('.font-medium')?.textContent.trim() === ${JSON.stringify(top.name)})?.click()`);
  await new Promise((r) => setTimeout(r, 2000));
  const rows = (await tauriPage.evaluate(`document.querySelectorAll('.sticky .justify-between.border-b').length`)) as number;
  const usageError = (await tauriPage.evaluate(`document.querySelector('.sticky .text-error')?.textContent ?? null`)) as string | null;

  console.log(`\n[inspect] libraries cards ${cards} (daemon ${libs.length}); ${top.name} usage rows ${rows} (daemon ${usage}); errors ${JSON.stringify([error, usageError])}`);
  expect([error, usageError]).toEqual([null, null]);
  expect(cards).toBe(libs.length);
  expect(rows).toBe(usage);
});

// #341: a library that ships a sensei.library.json shows what it provides —
// its skills, agents and doc pages, as many as the daemon holds.
test('a library shows the docs, skills and agents it provides', async ({ tauriPage }) => {
  const { libs } = await (await fetch(`${DAEMON_URL}/api/libs?`)).json();
  const lib = libs.find((l: { skillCount?: number }) => (l.skillCount ?? 0) > 0);
  expect(lib, 'a library with skills — rokkit, dbd or kavach').toBeTruthy();
  const read = async (what: string) =>
    (await (await fetch(`${DAEMON_URL}/api/libs/${encodeURIComponent(lib.name)}/${what}`)).json()).length;
  const [skills, agents, docs] = [await read('skills'), await read('agents'), await read('docs')];

  await navigateTo(tauriPage, '/libraries');
  await expect
    .poll(() => tauriPage.evaluate(`document.querySelectorAll('.lib-card').length`), { timeout: 30_000 })
    .toBeGreaterThan(0);
  await tauriPage.evaluate(`[...document.querySelectorAll('.lib-card')].find((c) => c.querySelector('.font-medium')?.textContent.trim() === ${JSON.stringify(lib.name)})?.click()`);
  const shown = (label: string) =>
    tauriPage.evaluate(`document.body.textContent.match(/${label} · (\\d+)/)?.[1] ?? '0'`);
  await expect.poll(() => shown('Skills'), { timeout: 30_000 }).toBe(String(skills));
  const counts = [await shown('Agents'), await shown('Docs')];

  console.log(`\n[inspect] ${lib.name} provides: skills ${skills}, agents ${agents}, docs ${docs}; shown ${[skills, ...counts]}`);
  expect(counts).toEqual([String(agents), String(docs)]);
});
