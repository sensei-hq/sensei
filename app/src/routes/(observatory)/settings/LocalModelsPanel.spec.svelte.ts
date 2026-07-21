// @vitest-environment jsdom
import { describe, it, expect, afterEach, vi } from 'vitest';
import { tick } from 'svelte';
import { mountComponent } from '$lib/test-mount.js';
import type { ApiResult, SenseiApi } from '$lib/api.js';
import type { ProvisionModel, ProvisionPhase } from '$lib/types.js';
import { LocalModels } from './local-models.svelte.js';
import LocalModelsPanelHarness from './LocalModelsPanel.harness.svelte';

let cleanup: Array<() => void> = [];
afterEach(() => { cleanup.forEach((fn) => fn()); cleanup = []; });

// A no-op timer so mounting a downloading/queued fixture never schedules a real
// poll during a component render test (the poll lifecycle is covered by the
// state spec). `set` returns a dummy handle; `clear` is a no-op.
const noPoll = {
  set: () => 0 as unknown as ReturnType<typeof setTimeout>,
  clear: () => {},
};

const model = (over: Partial<ProvisionModel> = {}): ProvisionModel => ({
  id: 'gemma2:2b',
  name: 'Gemma 2 2B Instruct',
  phase: { phase: 'absent' },
  ...over,
});

// Build a controller whose api returns the given fixtures from provisionStatus,
// then load() so the panel's onMount populates the same rows synchronously
// after a tick.
function fixtureController(
  models: ProvisionModel[],
  provisionModel?: SenseiApi['provisionModel'],
): LocalModels {
  const api = {
    provisionStatus: vi.fn().mockResolvedValue({ models }),
    provisionModel:
      provisionModel ??
      vi.fn().mockResolvedValue(
        { ok: true, data: { model: 'gemma2:2b', phase: { phase: 'queued' } } } as ApiResult<{
          model: string; phase: ProvisionPhase;
        }>,
      ),
  } as unknown as SenseiApi;
  return new LocalModels(api, noPoll);
}

// Mount the harness and let onMount's load() settle (a couple of microtasks).
async function mountLoaded(controller: LocalModels) {
  const m = mountComponent(LocalModelsPanelHarness, { controller });
  cleanup.push(m.destroy);
  await controller.load(); // deterministic: same fixtures the mock returns
  await tick();
  return m;
}

describe('LocalModelsPanel', () => {
  it('renders the panel shell, header, and subtitle', async () => {
    const m = await mountLoaded(fixtureController([model()]));
    const panel = m.container.querySelector('[data-testid="settings-local-models"]') as HTMLElement;
    expect(panel).toBeTruthy();
    expect(panel.className).toMatch(/\bbg-paper-mute\b/);
    expect(panel.className).toMatch(/\brounded-lg\b/);
    expect(m.container.textContent).toContain('Local models');
    expect(m.container.textContent).toContain('runs offline');
  });

  it('an absent model shows "not pulled" and a Pull button', async () => {
    const m = await mountLoaded(fixtureController([model({ phase: { phase: 'absent' } })]));
    expect(m.container.querySelector('[data-testid="local-model-gemma2:2b"]')).toBeTruthy();
    const pull = m.container.querySelector('[data-testid="local-model-pull-gemma2:2b"]') as HTMLElement;
    expect(pull).toBeTruthy();
    expect(pull.textContent?.trim()).toBe('Pull');
    expect(m.container.textContent).toContain('gemma2:2b');
    expect(m.container.textContent).toContain('Gemma 2 2B Instruct');
  });

  it('a downloading model shows the percent label + a progress bar, no Pull button', async () => {
    const m = await mountLoaded(
      fixtureController([model({ phase: { phase: 'downloading', done: 30, total: 100 } })]),
    );
    const status = m.container.querySelector('[data-testid="local-model-status-gemma2:2b"]') as HTMLElement;
    expect(status.textContent?.trim()).toBe('downloading 30%');
    expect(status.className).toMatch(/\btext-accent\b/);
    const bar = m.container.querySelector('[data-testid="local-model-progress-gemma2:2b"] > div') as HTMLElement;
    expect(bar.style.width).toBe('30%');
    // In flight → no pull button.
    expect(m.container.querySelector('[data-testid="local-model-pull-gemma2:2b"]')).toBeNull();
  });

  it('a ready model shows "ready" in success tone, full progress, no button', async () => {
    const m = await mountLoaded(fixtureController([model({ phase: { phase: 'ready' } })]));
    const status = m.container.querySelector('[data-testid="local-model-status-gemma2:2b"]') as HTMLElement;
    expect(status.textContent?.trim()).toBe('ready');
    expect(status.className).toMatch(/\btext-success\b/);
    const bar = m.container.querySelector('[data-testid="local-model-progress-gemma2:2b"] > div') as HTMLElement;
    expect(bar.style.width).toBe('100%');
    expect(m.container.querySelector('[data-testid="local-model-pull-gemma2:2b"]')).toBeNull();
  });

  it('a failed model shows a Retry button and carries the error as a tooltip', async () => {
    const m = await mountLoaded(
      fixtureController([model({ phase: { phase: 'failed', error: 'disk full' } })]),
    );
    const retry = m.container.querySelector('[data-testid="local-model-pull-gemma2:2b"]') as HTMLElement;
    expect(retry.textContent?.trim()).toBe('Retry');
    expect(retry.getAttribute('title')).toBe('disk full');
  });

  it('clicking Pull flips the row to queued within a tick (immediate feedback)', async () => {
    const provisionModel = vi.fn().mockResolvedValue({
      ok: true, data: { model: 'gemma2:2b', phase: { phase: 'queued' } },
    });
    const c = fixtureController([model({ phase: { phase: 'absent' } })], provisionModel);
    const m = await mountLoaded(c);

    const pull = m.container.querySelector('[data-testid="local-model-pull-gemma2:2b"]') as HTMLElement;
    pull.click();
    await tick();
    await tick();

    expect(provisionModel).toHaveBeenCalledWith('gemma2:2b');
    const status = m.container.querySelector('[data-testid="local-model-status-gemma2:2b"]') as HTMLElement;
    expect(status.textContent?.trim()).toBe('queued…');
    expect(m.container.querySelector('[data-testid="local-model-pull-gemma2:2b"]')).toBeNull();
  });

  it('empty catalog shows the "no local models configured" line', async () => {
    const m = await mountLoaded(fixtureController([]));
    const empty = m.container.querySelector('[data-testid="local-models-empty"]') as HTMLElement;
    expect(empty).toBeTruthy();
    expect(empty.textContent).toContain('No local models configured.');
  });

  it('a 501 on pull renders the not-available-in-this-build notice', async () => {
    const provisionModel = vi.fn().mockResolvedValue({
      ok: false, error: { status: 501, message: 'embedded provisioning not available in this build' },
    });
    const c = fixtureController([model({ phase: { phase: 'absent' } })], provisionModel);
    const m = await mountLoaded(c);

    (m.container.querySelector('[data-testid="local-model-pull-gemma2:2b"]') as HTMLElement).click();
    await tick();
    await tick();

    const notice = m.container.querySelector('[data-testid="local-models-notice"]') as HTMLElement;
    expect(notice).toBeTruthy();
    expect(notice.textContent).toMatch(/aren.t available in this build/i);
  });
});
