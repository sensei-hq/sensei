/**
 * EVERY SENSEI TOOL, RUN AND CHECKED (user, 2026-10-27): the baseline crawl
 * opened each tool in the Playground but never ran it, and a call that does not
 * fail is not yet a right answer. Here each tool is run through the Playground as
 * a person would — in the sensei workspace, fields filled, Query pressed —
 * against the REAL daemon, and its answer is held to what it should be, worked
 * out without the tool (`tool-oracles.ts`).
 *
 * A tool that writes to the live database, runs commands, or spends on a model
 * is not run; it is recorded as such. Every tool the server lists is accounted
 * for: run and checked, or not run with the reason.
 *
 * Run: `bun run test:inspect -- tool-calls`. Results: /tmp/sensei-tools.jsonl.
 */
import { writeFileSync, appendFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { expect, test } from '../fixtures';
import { DAEMON_URL, navigateTo, takeConsoleErrors, watchConsole } from '../helpers';
import { CASES, NOT_RUN, makeCtx, type Answer } from './tool-oracles';

const OUT = '/tmp/sensei-tools.jsonl';
const ROOT = resolve(process.cwd(), '..');

type Tool = { name: string; inputSchema?: { properties?: Record<string, unknown> } };

test('every sensei tool, run through the Playground and checked', async ({ tauriPage }) => {
  test.setTimeout(60 * 60_000);
  writeFileSync(OUT, '');
  const { servers } = await (await fetch(`${DAEMON_URL}/api/instruments/mcp-servers`)).json();
  const sensei = servers.find((s: { mcp_key: string }) => /(^|_)sensei$/.test(s.mcp_key));
  const { tools } = (await (await fetch(`${DAEMON_URL}/api/instruments/mcp-servers/${sensei.id}/tools`)).json()) as { tools: Tool[] };
  const workspaces = await (await fetch(`${DAEMON_URL}/api/workspaces`)).json();
  const wsId = (Array.isArray(workspaces) ? workspaces : workspaces.workspaces).find((w: { name: string }) => w.name === 'sensei').id;
  const ctx = makeCtx(ROOT, wsId, DAEMON_URL);

  const listed = tools.map((t) => t.name);
  const unaccounted = listed.filter((t) => !NOT_RUN[t] && !CASES.some((c) => c.tool === t));
  const gone = CASES.map((c) => c.tool).filter((t) => !listed.includes(t));
  expect({ unaccounted, gone }, 'every listed tool is checked or not run, and every check names a listed tool').toEqual({ unaccounted: [], gone: [] });

  await watchConsole(tauriPage);
  // The Playground runs each tool in the sensei workspace, as an assistant
  // working there would.
  await navigateTo(tauriPage, `/instruments?workspace=${wsId}`);
  await expect
    .poll(() => tauriPage.evaluate(`document.querySelector('[data-testid="run-in"]')?.textContent ?? ''`), { timeout: 60_000 })
    .toContain('sensei');

  const failures: string[] = [];
  for (const t of listed.filter((name) => NOT_RUN[name])) {
    appendFileSync(OUT, JSON.stringify({ tool: t, outcome: 'not-run', detail: NOT_RUN[t] }) + '\n');
  }
  for (const c of CASES) {
    const args = Object.fromEntries(Object.entries(c.args).map(([k, v]) => [k, v.replace('{root}', ROOT)]));
    // The last call is finished before this one starts: an answer still on
    // screen would be read as this tool's.
    await expect
      .poll(() => tauriPage.evaluate(`!document.querySelector('[data-testid="tool-execute"]')?.disabled`), { timeout: 120_000 })
      .toBe(true);
    await takeConsoleErrors(tauriPage);
    await tauriPage.evaluate(`document.querySelector('[data-testid="tool-row-${c.tool}"]')?.click()`);
    await new Promise((r) => setTimeout(r, 300));
    await tauriPage.evaluate(`(() => {
      for (const [name, value] of Object.entries(${JSON.stringify(args)})) {
        const el = document.querySelector('[data-testid="tool-input-' + name + '"]');
        if (!el) continue;
        el.value = value;
        el.dispatchEvent(new Event('input', { bubbles: true }));
      }
    })()`);
    const started = Date.now();
    await tauriPage.evaluate(`document.querySelector('[data-testid="tool-execute"]')?.click()`);

    let state: { error: string | null; ok: boolean; result: string } = { error: null, ok: false, result: '' };
    while (Date.now() - started < 90_000) {
      await new Promise((r) => setTimeout(r, 500));
      state = JSON.parse((await tauriPage.evaluate(`JSON.stringify({
        error: document.querySelector('[data-testid="tool-error"]')?.textContent ?? null,
        ok: !!document.querySelector('[data-testid="tool-ok"]'),
        result: [...document.querySelectorAll('pre')].map((p) => p.textContent).find((t) => t && t !== '—') ?? '',
      })`)) as string);
      if (state.error || state.ok) break;
    }
    let problems: string[];
    if (!state.error && !state.ok) problems = ['no answer within 90s'];
    else if (state.error && !state.result) problems = [`the call failed: ${state.error}`];
    else {
      let json: unknown = null;
      try {
        json = JSON.parse(state.result);
      } catch {
        /* a text answer */
      }
      const answer: Answer = { isError: state.error !== null, text: state.result, json };
      try {
        problems = await c.expect(answer, ctx);
      } catch (e) {
        problems = [`the check could not read the answer (${(e as Error).message}): ${state.result.slice(0, 160)}`];
      }
    }
    const console = await takeConsoleErrors(tauriPage);
    if (console.length) problems.push(`console: ${console.map((e: { text: string }) => e.text).join(' | ').slice(0, 200)}`);
    const outcome = problems.length ? 'wrong' : 'right';
    if (problems.length) failures.push(`${c.tool}: ${problems.join('; ')}`);
    appendFileSync(OUT, JSON.stringify({ tool: c.tool, args, oracle: c.oracle, outcome, problems, ms: Date.now() - started }) + '\n');
  }
  expect(failures, 'every tool answers what it should').toEqual([]);
});
