// @vitest-environment jsdom
import { describe, it, expect, afterEach } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import Header from './Header.svelte';
import { HealthState, emptyPayload } from '$lib/health-state.svelte.js';
import { MockTransport } from '$lib/health-transport.js';
import { COMPONENT_ORDER } from '$lib/health-types.js';
import type { HealthPayload, Remedy, HealthStatus } from '$lib/health-types.js';

let cleanup: Array<() => void> = [];
afterEach(() => { cleanup.forEach((fn) => fn()); cleanup = []; });

// ── Fixtures ──────────────────────────────────────────────────────────────────

const remedyFixture = (): Remedy => ({
  message: 'Run the script in your terminal.',
  script: 'brew install sensei-hq/tap/sensei',
  url: null,
});

const okPayload = (): HealthPayload => ({
  version: '0.2.14',
  uptimeSeconds: 12,
  platform: 'macos',
  packageManager: { id: 'homebrew', label: 'Homebrew', note: null, status: 'ready', version: '4.2.0', detail: null, installingVerb: 'installing', description: '' },
  components: COMPONENT_ORDER.map((id) => ({
    id, label: id, note: null, status: 'ready' as const, version: '1.0.0', detail: null,
    installingVerb: 'installing', description: '',
  })),
  status: 'ok',
  remedy: null,
});

const needsActionPayload = (): HealthPayload => ({
  ...okPayload(),
  packageManager: { id: 'homebrew', label: 'Homebrew', note: null, status: 'failed', version: null, detail: 'brew missing', installingVerb: 'installing', description: '' },
  components: COMPONENT_ORDER.map((id) => ({
    id, label: id, note: null, status: 'failed' as const, version: null, detail: 'blocked',
    installingVerb: 'installing', description: '',
  })),
  status: 'needs-action',
  remedy: remedyFixture(),
});

/** Build a HealthState seeded to the given payload with a no-op transport. */
function makeState(payload: HealthPayload): HealthState {
  return new HealthState(payload, new MockTransport({ checkPayload: payload }));
}

/** Build a checking/resolving payload (no remedy) with the given status. */
function busyPayload(status: Extract<HealthStatus, 'checking' | 'resolving'>): HealthPayload {
  return {
    version: '',
    uptimeSeconds: 0,
    platform: 'macos',
    packageManager: { id: 'homebrew', label: 'Homebrew', note: null, status: 'checking', version: null, detail: null, installingVerb: 'installing', description: '' },
    components: COMPONENT_ORDER.map((id) => ({
      id, label: id, note: null, status: 'checking' as const, version: null, detail: null,
      installingVerb: 'installing', description: '',
    })),
    status,
    remedy: null,
  };
}

// ── Tests ─────────────────────────────────────────────────────────────────────

describe('Header', () => {
  it('renders the sensei wordmark', () => {
    const state = makeState(emptyPayload);
    const m = mountComponent(Header, { state });
    cleanup.push(m.destroy);
    expect(m.container.textContent).toContain('sensei');
    // The logo mark (sensei.svg) lives in the wordmark
    expect(m.container.querySelector('[data-component="wordmark-mark"]')).not.toBeNull();
  });

  it.each([
    ['checking',     'starting'],
    ['resolving',    'setting up'],
    ['needs-action', 'needs your hand'],
    ['ok',           'ready'],
  ] as const)('eyebrow for status=%s is "%s"', (status, eyebrow) => {
    let payload: HealthPayload;
    if (status === 'ok') payload = okPayload();
    else if (status === 'needs-action') payload = needsActionPayload();
    else payload = busyPayload(status);
    const state = makeState(payload);
    const m = mountComponent(Header, { state });
    cleanup.push(m.destroy);
    expect(m.container.textContent?.toLowerCase()).toContain(eyebrow);
  });

  it('renders the "ok" headline', () => {
    const state = makeState(okPayload());
    const m = mountComponent(Header, { state });
    cleanup.push(m.destroy);
    expect(m.container.textContent).toContain('The foundation');
    expect(m.container.textContent).toContain('holds');
  });

  it('renders the "resolving" headline', () => {
    const state = makeState(busyPayload('resolving'));
    const m = mountComponent(Header, { state });
    cleanup.push(m.destroy);
    expect(m.container.textContent).toMatch(/Putting the room.*in order/);
  });

  it('renders the "needs-action" headline', () => {
    const state = makeState(needsActionPayload());
    const m = mountComponent(Header, { state });
    cleanup.push(m.destroy);
    expect(m.container.textContent).toContain('One last');
  });

  it('renders the "checking" headline', () => {
    const state = makeState(emptyPayload);
    const m = mountComponent(Header, { state });
    cleanup.push(m.destroy);
    expect(m.container.textContent).toMatch(/Checking the.*foundation/i);
  });

  it('renders a sub-copy paragraph beneath the headline', () => {
    // Sub-copy varies per state but every state has one — assert presence.
    const state = makeState(emptyPayload);
    const m = mountComponent(Header, { state });
    cleanup.push(m.destroy);
    const sub = m.container.querySelector('[data-sub]');
    expect(sub).not.toBeNull();
    expect(sub?.textContent?.trim().length).toBeGreaterThan(0);
  });
});
