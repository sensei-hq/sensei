import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { HealthState, emptyPayload } from './health-state.svelte.js';
import { MockTransport } from './health-transport.js';
import { COMPONENT_ORDER } from './health-types.js';
import type { HealthPayload, Remedy } from './health-types.js';

// Pretend Tauri is present so HealthState's runtime bypass check
// (`window.__TAURI__`) resolves to "Tauri build" — i.e. non-bypass —
// which is what every test in this file except the dedicated bypass
// block assumes. Bypass-specific tests delete the global temporarily.
const win = (globalThis as unknown as { window?: Record<string, unknown> }).window
  ?? ((globalThis as unknown as { window: Record<string, unknown> }).window = {});
win.__TAURI__ = {};

const remedyFixture = (): Remedy => ({
  message: 'Run the script in your terminal.',
  script: 'brew install sensei-hq/tap/sensei',
  url: null,
});

const okPayload = (): HealthPayload => ({
  version: '0.2.14',
  uptimeSeconds: 12,
  platform: 'macos',
  packageManager: { id: 'homebrew', label: 'Homebrew', note: null, status: 'ready', version: '4.2.0', detail: null },
  components: COMPONENT_ORDER.map((id) => ({
    id, label: id, note: null, status: 'ready' as const, version: '1.0.0', detail: null,
  })),
  status: 'ok',
  remedy: null,
});

const needsActionPayload = (): HealthPayload => ({
  ...okPayload(),
  packageManager: { id: 'homebrew', label: 'Homebrew', note: null, status: 'failed', version: null, detail: 'brew missing' },
  components: COMPONENT_ORDER.map((id) => ({
    id, label: id, note: null, status: 'failed' as const, version: null, detail: 'blocked',
  })),
  status: 'needs-action',
  remedy: remedyFixture(),
});

describe('HealthState — construction', () => {
  it('defaults to the empty payload', () => {
    const s = new HealthState();
    expect(s.status).toBe('checking');
    expect(s.version).toBe('');
    expect(s.platform).toBe('macos');
    expect(s.components).toHaveLength(5);
    expect(s.components.map((c) => c.id)).toEqual([...COMPONENT_ORDER]);
    expect(s.components.every((c) => c.status === 'pending')).toBe(true);
    expect(s.packageManager.id).toBe('homebrew');
    expect(s.remedy).toBeNull();
    expect(s.latest).toBeNull();
  });

  it('applies a seed payload through apply()', () => {
    const s = new HealthState(okPayload());
    expect(s.status).toBe('ok');
    expect(s.version).toBe('0.2.14');
    expect(s.components.every((c) => c.status === 'ready')).toBe(true);
  });

  it('emptyPayload satisfies all invariants (constructor would throw otherwise)', () => {
    expect(() => new HealthState(emptyPayload)).not.toThrow();
  });
});

describe('HealthState — apply() happy paths', () => {
  it('applies an ok payload', () => {
    const s = new HealthState();
    s.apply(okPayload());
    expect(s.status).toBe('ok');
    expect(s.remedy).toBeNull();
    expect(s.components.every((c) => c.status === 'ready')).toBe(true);
  });

  it('applies a needs-action payload (remedy is set)', () => {
    const s = new HealthState();
    s.apply(needsActionPayload());
    expect(s.status).toBe('needs-action');
    expect(s.remedy?.script).toContain('brew install');
  });

  it('applies a resolving payload (remedy cleared)', () => {
    const s = new HealthState(needsActionPayload());
    s.apply({ ...okPayload(), status: 'resolving', remedy: null });
    expect(s.status).toBe('resolving');
    expect(s.remedy).toBeNull();
  });

  it('applies a checking payload', () => {
    const s = new HealthState(okPayload());
    s.apply({ ...okPayload(), status: 'checking', remedy: null });
    expect(s.status).toBe('checking');
  });

  it('replaces fields rather than merging on successive apply()', () => {
    const s = new HealthState(okPayload());
    s.apply({ ...okPayload(), version: '9.9.9', uptimeSeconds: 999 });
    expect(s.version).toBe('9.9.9');
  });
});

describe('HealthState — apply() invariants', () => {
  it('INV-1: needs-action with null remedy throws', () => {
    const s = new HealthState();
    const bad = { ...okPayload(), status: 'needs-action', remedy: null } as unknown as HealthPayload;
    expect(() => s.apply(bad)).toThrow(/needs-action requires a remedy/);
  });

  it('INV-1: non-needs-action with non-null remedy throws', () => {
    const s = new HealthState();
    const bad = { ...okPayload(), status: 'ok', remedy: remedyFixture() } as unknown as HealthPayload;
    expect(() => s.apply(bad)).toThrow(/must not carry a remedy/);
  });

  it('INV-2: wrong components length throws', () => {
    const s = new HealthState();
    const bad = { ...okPayload(), components: okPayload().components.slice(0, 4) };
    expect(() => s.apply(bad)).toThrow(/expected 5 components, got 4/);
  });

  it('INV-2: wrong components order throws', () => {
    const s = new HealthState();
    const reordered = okPayload();
    [reordered.components[0], reordered.components[1]] = [reordered.components[1], reordered.components[0]];
    expect(() => s.apply(reordered)).toThrow(/components\[0\]\.id must be "postgres"/);
  });

  it('INV-3: macos platform with winget package manager throws', () => {
    const s = new HealthState();
    const bad = { ...okPayload(), platform: 'macos' as const,
      packageManager: { ...okPayload().packageManager, id: 'winget' as const } };
    expect(() => s.apply(bad)).toThrow(/platform=macos expects packageManager.id="homebrew"/);
  });

  it('INV-3: windows platform with homebrew package manager throws', () => {
    const s = new HealthState();
    const bad = { ...okPayload(), platform: 'windows' as const,
      packageManager: { ...okPayload().packageManager, id: 'homebrew' as const } };
    expect(() => s.apply(bad)).toThrow(/platform=windows expects packageManager.id="winget"/);
  });

  it('INV-3: linux platform requires homebrew', () => {
    const s = new HealthState();
    const bad = { ...okPayload(), platform: 'linux' as const,
      packageManager: { ...okPayload().packageManager, id: 'winget' as const } };
    expect(() => s.apply(bad)).toThrow(/platform=linux expects packageManager.id="homebrew"/);
  });
});

describe('HealthState — applyEvent("phase")', () => {
  it('sets status to checking', () => {
    const s = new HealthState(okPayload());
    s.applyEvent({ kind: 'phase', phase: 'checking' });
    expect(s.status).toBe('checking');
  });

  it('sets status to resolving', () => {
    const s = new HealthState();
    s.applyEvent({ kind: 'phase', phase: 'resolving' });
    expect(s.status).toBe('resolving');
  });
});

describe('HealthState — applyEvent("component")', () => {
  it('patches a known ledger component, leaves others intact', () => {
    const s = new HealthState(okPayload());
    s.applyEvent({ kind: 'component', id: 'postgres', patch: { status: 'installing' } });
    expect(s.components[0].status).toBe('installing');
    expect(s.components[1].status).toBe('ready');
    expect(s.components[2].status).toBe('ready');
  });

  it('patches the package manager', () => {
    const s = new HealthState(okPayload());
    s.applyEvent({ kind: 'component', id: 'homebrew', patch: { detail: 'permission denied' } });
    expect(s.packageManager.detail).toBe('permission denied');
    expect(s.packageManager.status).toBe('ready'); // un-patched fields intact
  });

  it('patches multiple fields at once', () => {
    const s = new HealthState(okPayload());
    s.applyEvent({ kind: 'component', id: 'daemon',
      patch: { status: 'failed', detail: 'port in use' } });
    expect(s.components[4].status).toBe('failed');
    expect(s.components[4].detail).toBe('port in use');
  });

  it('INV-4: unknown component id throws', () => {
    const s = new HealthState();
    expect(() =>
      s.applyEvent({ kind: 'component', id: 'not-a-thing' as never, patch: {} })
    ).toThrow(/unknown component id "not-a-thing"/);
  });
});

describe('HealthState — applyEvent("remedy")', () => {
  it('replaces remedy regardless of previous value', () => {
    const s = new HealthState();
    const r1 = remedyFixture();
    s.applyEvent({ kind: 'remedy', remedy: r1 });
    expect(s.remedy).toEqual(r1);
    const r2 = { ...r1, message: 'new message' };
    s.applyEvent({ kind: 'remedy', remedy: r2 });
    expect(s.remedy?.message).toBe('new message');
  });
});

describe('HealthState — applyEvent("report")', () => {
  it('terminal report is equivalent to apply()', () => {
    const s = new HealthState();
    s.applyEvent({ kind: 'report', payload: needsActionPayload() });
    expect(s.status).toBe('needs-action');
    expect(s.remedy).not.toBeNull();
    expect(s.components.every((c) => c.status === 'failed')).toBe(true);
  });
});

describe('HealthState — applyEvent INV-5', () => {
  it('unknown event kind throws', () => {
    const s = new HealthState();
    expect(() => s.applyEvent({ kind: 'bogus' } as never))
      .toThrow(/unknown event kind/);
  });
});

describe('HealthState — derived getters', () => {
  it.each([
    ['checking',     { isOk: false, isBusy: true,  needsAction: false }],
    ['resolving',    { isOk: false, isBusy: true,  needsAction: false }],
    ['ok',           { isOk: true,  isBusy: false, needsAction: false }],
    ['needs-action', { isOk: false, isBusy: false, needsAction: true  }],
  ] as const)('status=%s → isOk/isBusy/needsAction', (status, expected) => {
    const s = new HealthState();
    if (status === 'needs-action') {
      s.apply(needsActionPayload());
    } else {
      s.apply({ ...okPayload(), status, remedy: null });
    }
    expect(s.isOk).toBe(expected.isOk);
    expect(s.isBusy).toBe(expected.isBusy);
    expect(s.needsAction).toBe(expected.needsAction);
  });
});

describe('HealthState — latest', () => {
  it('is a writable reactive field (Phase 2 fills it from a transport)', () => {
    const s = new HealthState();
    expect(s.latest).toBeNull();
    s.latest = '0.3.0';
    expect(s.latest).toBe('0.3.0');
  });
});

describe('HealthState — B1: constructor accepts a transport', () => {
  it('accepts a MockTransport without throwing', () => {
    const transport = new MockTransport({ checkPayload: okPayload() });
    expect(() => new HealthState(emptyPayload, transport)).not.toThrow();
  });
});

describe('HealthState — B2: init() lifecycle', () => {
  it('calls transport.check() exactly once and applies result', async () => {
    const transport = new MockTransport({ checkPayload: okPayload() });
    const s = new HealthState(emptyPayload, transport);
    await s.init();
    expect(transport.checkCalls).toHaveLength(1);
    expect(s.status).toBe('ok');
  });

  it('calls transport.resolve() when check returns non-ok', async () => {
    const transport = new MockTransport({ checkPayload: needsActionPayload() });
    const s = new HealthState(emptyPayload, transport);
    await s.init();
    expect(transport.resolveCalls).toHaveLength(1);
  });

  it('does NOT call transport.resolve() when check returns ok', async () => {
    const transport = new MockTransport({ checkPayload: okPayload() });
    const s = new HealthState(emptyPayload, transport);
    await s.init();
    expect(transport.resolveCalls).toHaveLength(0);
  });

  it('concurrent init() callers share one in-flight promise (check called once)', async () => {
    const transport = new MockTransport({ checkPayload: okPayload() });
    const s = new HealthState(emptyPayload, transport);
    await Promise.all([s.init(), s.init()]);
    expect(transport.checkCalls).toHaveLength(1);
  });

  it('HealthEvent fed via resolve callback mutates state correctly', async () => {
    const transport = new MockTransport({
      checkPayload: needsActionPayload(),
      resolveEvents: [
        { kind: 'component', id: 'postgres', patch: { status: 'ready', version: '16.0' } },
      ],
    });
    const s = new HealthState(emptyPayload, transport);
    await s.init();
    expect(s.components[0].status).toBe('ready');
    expect(s.components[0].version).toBe('16.0');
  });

  it('resolves with undefined after check + resolve complete', async () => {
    const transport = new MockTransport({ checkPayload: needsActionPayload() });
    const s = new HealthState(emptyPayload, transport);
    const result = await s.init();
    expect(result).toBeUndefined();
  });
});

describe('HealthState — B3: verify() forces a fresh check', () => {
  it('calls sessionStorage.removeItem for sensei:health during verify', async () => {
    const sessionStore = new Map<string, string>();
    const removedKeys: string[] = [];
    vi.stubGlobal('sessionStorage', {
      getItem:    (k: string) => sessionStore.get(k) ?? null,
      setItem:    (k: string, v: string) => sessionStore.set(k, v),
      removeItem: (k: string) => { removedKeys.push(k); sessionStore.delete(k); },
    });
    sessionStore.set('sensei:health', 'ready');

    const transport = new MockTransport({ checkPayload: okPayload() });
    const s = new HealthState(emptyPayload, transport);
    await s.verify();
    expect(removedKeys).toContain('sensei:health');

    vi.unstubAllGlobals();
  });

  it('causes a fresh transport.check() call after a prior init()', async () => {
    const transport = new MockTransport({ checkPayload: okPayload() });
    const s = new HealthState(emptyPayload, transport);
    await s.init();
    expect(transport.checkCalls).toHaveLength(1);
    await s.verify();
    expect(transport.checkCalls).toHaveLength(2);
  });

  it('concurrent verify() calls trigger only one check pass', async () => {
    const transport = new MockTransport({ checkPayload: okPayload() });
    const s = new HealthState(emptyPayload, transport);
    await Promise.all([s.verify(), s.verify()]);
    expect(transport.checkCalls).toHaveLength(1);
  });

  it('does not throw when sessionStorage is undefined', async () => {
    const transport = new MockTransport({ checkPayload: okPayload() });
    const s = new HealthState(emptyPayload, transport);
    await expect(s.verify()).resolves.toBeUndefined();
  });
});

describe('HealthState — B4: apply() writes sessionStorage cache', () => {
  let sessionStore: Map<string, string>;

  beforeEach(() => {
    sessionStore = new Map<string, string>();
    vi.stubGlobal('sessionStorage', {
      getItem:    (k: string) => sessionStore.get(k) ?? null,
      setItem:    (k: string, v: string) => sessionStore.set(k, v),
      removeItem: (k: string) => sessionStore.delete(k),
    });
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('applying an ok payload writes ready to sensei:health', () => {
    const s = new HealthState(emptyPayload);
    s.apply(okPayload());
    expect(sessionStore.get('sensei:health')).toBe('ready');
  });

  it('applying a needs-action payload removes sensei:health', () => {
    sessionStore.set('sensei:health', 'ready');
    const s = new HealthState(emptyPayload);
    s.apply(needsActionPayload());
    expect(sessionStore.has('sensei:health')).toBe(false);
  });

  it('applying a checking payload removes sensei:health', () => {
    sessionStore.set('sensei:health', 'ready');
    const s = new HealthState(emptyPayload);
    s.apply({ ...okPayload(), status: 'checking', remedy: null });
    expect(sessionStore.has('sensei:health')).toBe(false);
  });

  it('applying a resolving payload removes sensei:health', () => {
    sessionStore.set('sensei:health', 'ready');
    const s = new HealthState(emptyPayload);
    s.apply({ ...okPayload(), status: 'resolving', remedy: null });
    expect(sessionStore.has('sensei:health')).toBe(false);
  });

  it('does not throw when sessionStorage is undefined', () => {
    vi.unstubAllGlobals();
    const s = new HealthState(emptyPayload);
    expect(() => s.apply(okPayload())).not.toThrow();
  });
});

describe('HealthState — bypass when no Tauri runtime', () => {
  let sessionStore: Map<string, string>;
  let savedTauri: unknown;

  beforeEach(() => {
    sessionStore = new Map<string, string>();
    vi.stubGlobal('sessionStorage', {
      getItem:    (k: string) => sessionStore.get(k) ?? null,
      setItem:    (k: string, v: string) => sessionStore.set(k, v),
      removeItem: (k: string) => sessionStore.delete(k),
    });
    // Bypass is decided by `window.__TAURI__` at runtime. The file-level
    // stub sets it for every other test in this file; for this block we
    // temporarily delete it to simulate a browser-only (no-Tauri)
    // context.
    savedTauri = (win as { __TAURI__?: unknown }).__TAURI__;
    delete (win as { __TAURI__?: unknown }).__TAURI__;
  });

  afterEach(() => {
    vi.unstubAllGlobals();
    if (savedTauri !== undefined) (win as { __TAURI__?: unknown }).__TAURI__ = savedTauri;
  });

  it('constructor marks status as ok and writes the cache key when bypass is set', () => {
    const transport = new MockTransport({ checkPayload: needsActionPayload() });
    const s = new HealthState(emptyPayload, transport);
    expect(s.status).toBe('ok');
    expect(sessionStore.get('sensei:health')).toBe('ready');
    expect(transport.checkCalls).toHaveLength(0);
  });

  it('init() does not call transport.check() when bypass is set', async () => {
    const transport = new MockTransport({ checkPayload: needsActionPayload() });
    const s = new HealthState(emptyPayload, transport);
    await s.init();
    expect(transport.checkCalls).toHaveLength(0);
    expect(transport.resolveCalls).toHaveLength(0);
    expect(s.status).toBe('ok');
  });

  it('verify() does not call transport.check() when bypass is set', async () => {
    const transport = new MockTransport({ checkPayload: needsActionPayload() });
    const s = new HealthState(emptyPayload, transport);
    await s.verify();
    expect(transport.checkCalls).toHaveLength(0);
    expect(transport.resolveCalls).toHaveLength(0);
    expect(s.status).toBe('ok');
    expect(sessionStore.get('sensei:health')).toBe('ready');
  });
});

export { okPayload, needsActionPayload, remedyFixture };
