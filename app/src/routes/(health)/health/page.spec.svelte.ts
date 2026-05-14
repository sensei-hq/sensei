// @vitest-environment jsdom
import { describe, it, expect, afterEach, vi, beforeEach } from 'vitest';
import { tick } from 'svelte';
import { mountComponent } from '$lib/test-mount.js';
import { healthState } from '$lib/health-state.svelte.js';
import Page from './+page.svelte';

vi.mock('$app/navigation', () => ({ goto: vi.fn() }));

let cleanup: Array<() => void> = [];
afterEach(() => { cleanup.forEach((fn) => fn()); cleanup = []; vi.restoreAllMocks(); });

describe('/health/+page.svelte', () => {
  let initSpy: ReturnType<typeof vi.spyOn>;
  let verifySpy: ReturnType<typeof vi.spyOn>;

  beforeEach(() => {
    initSpy   = vi.spyOn(healthState, 'init').mockResolvedValue(undefined);
    verifySpy = vi.spyOn(healthState, 'verify').mockResolvedValue(undefined);
  });

  it('calls healthState.init() once on mount', async () => {
    const m = mountComponent(Page, {});
    cleanup.push(m.destroy);
    await tick();
    expect(initSpy).toHaveBeenCalledTimes(1);
  });

  it('clicking the Verify button (via Remedy) calls healthState.verify()', async () => {
    healthState.apply({
      version: '0.2.14', uptimeSeconds: 0, platform: 'macos',
      packageManager: { id: 'homebrew', label: 'Homebrew', note: null, status: 'failed', version: null, detail: 'brew missing' },
      components: ['postgres', 'ollama', 'sensei', 'database', 'daemon'].map((id) => ({
        id: id as 'postgres', label: id, note: null, status: 'failed' as const, version: null, detail: 'blocked',
      })),
      status: 'needs-action',
      remedy: { message: 'Run script', script: 'brew install sensei-hq/tap/sensei', url: null },
    });

    const m = mountComponent(Page, {});
    cleanup.push(m.destroy);
    await tick();

    const btn = m.container.querySelector('button[data-action="verify"]') as HTMLButtonElement | null;
    expect(btn).not.toBeNull();
    btn!.click();
    expect(verifySpy).toHaveBeenCalledTimes(1);
  });
});
