/**
 * #288 against the REAL daemon: the Instruments store's reads land — the
 * Playground's sensei group lists the tools its server lists, Health shows one card per
 * tools-health source, and a session's Replay timeline has the daemon's calls —
 * with no error state, and a Playground call shows the server's answer (#293:
 * the call used to show `{}` when it failed). Read from the DOM: the sign-in overlay (#283) is modal
 * while it is up. Run: `bun run test:inspect -- instruments-store`.
 */
import { expect, test } from '../fixtures';
import { DAEMON_URL, navigateTo } from '../helpers';

const json = async (path: string) => (await fetch(`${DAEMON_URL}${path}`)).json();
const errors = `document.querySelectorAll('[data-screen-state="error"], [data-testid="replay-detail-error"]').length`;

test('instruments shows the daemon reads', async ({ tauriPage }) => {
  // Sensei's own server, as the assistant's config names it, and its tools as
  // the server itself lists them.
  const { servers } = await json('/api/instruments/mcp-servers');
  const sensei = servers.find((s: { mcp_key: string }) => /(^|_)sensei$/.test(s.mcp_key));
  expect(sensei, 'the sensei MCP is configured for an assistant').toBeTruthy();
  const tools = (await json(`/api/instruments/mcp-servers/${sensei.id}/tools`)).tools.length;
  const sources = (await json('/api/instruments/tools-health')).sources.length;

  await navigateTo(tauriPage, '/instruments');
  await expect
    .poll(() => tauriPage.evaluate(`document.querySelector('[data-testid="mcp-group-toggle-${sensei.id}"] .font-mono.text-xs.text-ink-faint')?.textContent?.trim() ?? null`), { timeout: 30_000 })
    .toBe(String(tools));
  const playground = (await tauriPage.evaluate(errors)) as number;

  // A read-only query with no inputs, run through the server: the panel shows
  // the server's answer and how long it took.
  await tauriPage.evaluate(`document.querySelector('[data-testid="tool-row-list_workspaces"]')?.click()`);
  await tauriPage.evaluate(`document.querySelector('[data-testid="tool-execute"]')?.click()`);
  await expect
    .poll(() => tauriPage.evaluate(`document.querySelector('[data-testid="tool-error"]')?.textContent ?? (document.querySelector('[data-testid="tool-ok"]') ? 'ok' : null)`), { timeout: 60_000 })
    .toBe('ok');
  const shown = (await tauriPage.evaluate(`[...document.querySelectorAll('pre')].map((p) => p.textContent).find((t) => /^[[{]/.test(t.trim())) ?? ''`)) as string;
  expect(() => JSON.parse(shown), 'the server answered with JSON').not.toThrow();

  await tauriPage.evaluate(`document.querySelector('[data-testid="instrument-tab-insights"]')?.click()`);
  await expect
    .poll(() => tauriPage.evaluate(`document.querySelectorAll('[data-testid^="tool-health-card-"]').length`), { timeout: 30_000 })
    .toBe(sources);
  const health = (await tauriPage.evaluate(errors)) as number;

  // A session that has tool calls, so the comparison means something.
  const { sessions } = await json('/api/sessions');
  let session = sessions[0];
  let replay = { calls: [] as unknown[] };
  for (const candidate of sessions.slice(0, 20)) {
    replay = await json(`/api/sessions/${encodeURIComponent(candidate.id)}/replay?limit=200`);
    session = candidate;
    if (replay.calls.length > 0) break;
  }
  // The page reads ?tab/?session once, at mount: leave it first so the deep
  // link mounts it afresh.
  await navigateTo(tauriPage, '/');
  await navigateTo(tauriPage, `/instruments?tab=replay&session=${encodeURIComponent(session.id)}`);
  await expect
    .poll(() => tauriPage.evaluate(`document.body.textContent.match(/timeline \\((\\d+)\\)/)?.[1] ?? null`), { timeout: 30_000 })
    .toBe(String(replay.calls.length));
  const rep = (await tauriPage.evaluate(errors)) as number;

  console.log(`\n[inspect] instruments: sensei tools ${tools}, list_workspaces ${shown.length}B shown, sources ${sources}, replay ${session.id.slice(0, 8)} ${replay.calls.length} calls; errors ${[playground, health, rep]}`);
  expect([playground, health, rep]).toEqual([0, 0, 0]);
});
