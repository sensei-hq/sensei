// @vitest-environment jsdom
// A link out of the app (#381): the Tauri webview ignores target=_blank, so a link opens
// in the system browser through the opener plugin; outside Tauri (a plain browser in dev)
// the plugin refuses and the link opens in a new tab instead.
import { describe, it, expect, vi, afterEach } from 'vitest';

const opener = vi.hoisted(() => ({ openUrl: vi.fn(async (_url: string) => {}) }));
vi.mock('@tauri-apps/plugin-opener', () => opener);

import { openExternal } from './open-external.js';

afterEach(() => {
  vi.unstubAllGlobals();
  opener.openUrl.mockReset();
});

describe('openExternal', () => {
  it('opens the link in the system browser', async () => {
    const open = vi.fn();
    vi.stubGlobal('open', open);
    await openExternal('https://github.com/sensei-hq/sensei/issues/381');
    expect(opener.openUrl).toHaveBeenCalledWith('https://github.com/sensei-hq/sensei/issues/381');
    expect(open).not.toHaveBeenCalled();
  });

  it('opens a new tab when the opener refuses (no Tauri)', async () => {
    opener.openUrl.mockRejectedValueOnce(new Error('not in tauri'));
    const open = vi.fn();
    vi.stubGlobal('open', open);
    await openExternal('https://x/SEN-52');
    expect(open).toHaveBeenCalledWith('https://x/SEN-52', '_blank', 'noopener');
  });
});
