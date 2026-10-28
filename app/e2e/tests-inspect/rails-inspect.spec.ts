/**
 * The three sidebars — main window, Settings, a workspace — are one Rail with
 * one rokkit List (user, 2026-10-28: "all should use same list component with
 * same styles … verify styles match using playwright"). Against the real app:
 * each route's rail is measured where it renders, and the measurements must be
 * identical — the frame, a group heading, an entry and the current entry.
 *
 * Run: `bun run test:inspect -- rails`.
 */
import { expect, test } from '../fixtures';
import { navigateToScreen } from '../helpers';

const MEASURE = `(async () => {
  for (let i = 0; i < 40 && !document.querySelector('aside[data-rail] [data-list-item][data-active="true"]'); i++)
    await new Promise((r) => setTimeout(r, 250));
  const a = document.querySelector('aside[data-rail]');
  const pick = (el, props) => {
    if (!el) return null;
    const c = getComputedStyle(el);
    return Object.fromEntries(props.map((p) => [p, c[p]]));
  };
  const entry = [...a.querySelectorAll('[data-list-item]')].find((e) => e.dataset.active !== 'true');
  return JSON.stringify({
    frame: pick(a, ['width', 'paddingTop', 'paddingLeft', 'backgroundColor', 'borderRightColor', 'rowGap']),
    heading: pick(a.querySelector('[data-list-group]'), ['fontFamily', 'fontSize', 'fontWeight', 'letterSpacing', 'textTransform', 'color', 'margin']),
    entry: pick(entry, ['fontFamily', 'fontSize', 'fontWeight', 'color', 'padding', 'borderRadius', 'height']),
    current: pick(a.querySelector('[data-list-item][data-active="true"]'), ['color', 'backgroundColor', 'fontSize', 'padding']),
  });
})()`;

const PORT = parseInt(process.env.SENSEI_PORT ?? '7744', 10);

async function firstWorkspaceId(): Promise<string> {
  const d = await (await fetch(`http://127.0.0.1:${PORT}/api/workspaces?watched=true`)).json();
  return (Array.isArray(d) ? d : d.workspaces)[0].id;
}

test('the main, settings and workspace sidebars render identically', async ({ tauriPage }) => {
  const routes: [string, string][] = [
    ['/workspaces', 'observatory-sidebar'],
    ['/settings/general', 'settings-sidebar'],
  ];
  const ws = await firstWorkspaceId();
  routes.push([`/workspace/${ws}/overview`, 'workspace-sidebar']);

  const measured: Record<string, unknown> = {};
  for (const [route, component] of routes) {
    await navigateToScreen(tauriPage, route, `aside[data-rail][data-component="${component}"]`);
    measured[component] = JSON.parse((await tauriPage.evaluate(MEASURE)) as string);
  }
  console.log(JSON.stringify(measured, null, 2));
  for (const part of ['frame', 'heading', 'entry', 'current'] as const) {
    const values = Object.values(measured).map((m) => JSON.stringify((m as Record<string, unknown>)[part]));
    expect(values[0], `${part} measured`).not.toBe('null');
    expect(new Set(values).size, `${part} differs: ${values.join(' | ')}`).toBe(1);
  }
});
