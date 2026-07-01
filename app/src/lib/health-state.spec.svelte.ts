import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { HealthState, emptyPayload } from './health-state.svelte.js';
import { MockTransport } from './health-transport.js';
import { COMPONENT_ORDER } from './health-types.js';
import type { HealthPayload, Remedy, Component, HealthStatus } from './health-types.js';

const remedyFixture = (): Remedy => ({
  message: 'Run the script in your terminal.',
  script: 'brew install sensei-hq/tap/sensei',
  url: null,
});

// Fixtures stand in for wire payloads. `description` is required by the
// Component type, but the value is irrelevant here — HealthState.apply()
// always overwrites it from the frontend DESCRIPTIONS map before exposing
// the Component to the UI. We pass empty strings to satisfy the type.
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

describe('HealthState — construction', () => {
  it('defaults to the empty payload', () => {
    const s = new HealthState();
    expect(s.status).toBe('checking');
    expect(s.version).toBe('');
    expect(s.platform).toBe('macos');
    expect(s.components).toHaveLength(5);
    expect(s.components.map((c) => c.id)).toEqual([...COMPONENT_ORDER]);
    expect(s.components.every((c) => c.status === 'checking')).toBe(true);
    expect(s.packageManager.status).toBe('checking');
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

describe('HealthState — description hydration', () => {
  // Descriptions are a frontend concern (poetic copy per gate) hydrated by the
  // state. The wire payload never has to carry them — apply() and #patch()
  // overwrite from the frontend map. This guarantees Ledger and Hero can
  // render `c.description` without falsy checks.

  it('emptyComponent hydrates every component with a non-empty description', () => {
    const s = new HealthState();
    for (const c of s.components) {
      expect(c.description.length).toBeGreaterThan(0);
    }
    expect(s.packageManager.description.length).toBeGreaterThan(0);
  });

  it('every gate gets the canonical description from the frontend map', () => {
    const s = new HealthState();
    const byId = new Map(s.components.map((c) => [c.id, c.description]));
    expect(byId.get('postgres')).toBe('The local database where every session and memory is stored.');
    expect(byId.get('ollama')).toBe('Runs the models on-device, so your code never leaves the machine.');
    expect(byId.get('sensei')).toBe('The CLI, the MCP server assistants talk to, and the watcher.');
    expect(byId.get('database')).toBe('Creates the schema and vector index memories are searched through.');
    expect(byId.get('daemon')).toBe('Watches sessions in the background — nothing works without it.');
    expect(s.packageManager.description).toBe('Installs and updates everything else from one manifest.');
  });

  it('apply() overwrites description from frontend map even if the wire omits it', () => {
    const payload = okPayload();
    payload.components.forEach((c) => { delete (c as Partial<typeof c>).description; });
    delete (payload.packageManager as Partial<typeof payload.packageManager>).description;

    const s = new HealthState();
    s.apply(payload);
    expect(s.components.every((c) => c.description.length > 0)).toBe(true);
    expect(s.packageManager.description.length).toBeGreaterThan(0);
  });

  it('#patch() preserves description on a component event', () => {
    const s = new HealthState(okPayload());
    const before = s.components[0].description;
    s.applyEvent({ kind: 'component', id: 'postgres', patch: { status: 'installing' } });
    expect(s.components[0].description).toBe(before);
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
  it('calls transport.resolve() exactly once and applies the terminal payload', async () => {
    const transport = new MockTransport({ checkPayload: okPayload() });
    const s = new HealthState(emptyPayload, transport);
    await s.init();
    expect(transport.resolveCalls).toHaveLength(1);
    expect(s.status).toBe('ok');
  });

  it('arrives at needs-action when terminal payload requires it', async () => {
    const transport = new MockTransport({ checkPayload: needsActionPayload() });
    const s = new HealthState(emptyPayload, transport);
    await s.init();
    expect(transport.resolveCalls).toHaveLength(1);
    expect(s.status).toBe('needs-action');
  });

  it('concurrent init() callers share one in-flight promise (resolve called once)', async () => {
    const transport = new MockTransport({ checkPayload: okPayload() });
    const s = new HealthState(emptyPayload, transport);
    await Promise.all([s.init(), s.init()]);
    expect(transport.resolveCalls).toHaveLength(1);
  });

  it('HealthEvent fed via resolve callback mutates state correctly', async () => {
    // Terminal payload reflects the patched state — in the streaming flow
    // the terminal `report` event is the authoritative final state, so a
    // patched component must also appear in resolveTerminal for the post-
    // report `apply()` to land it.
    const recoveredTerminal: HealthPayload = {
      ...needsActionPayload(),
      components: needsActionPayload().components.map((c, i) =>
        i === 0 ? { ...c, status: 'ready', version: '16.0' } : c,
      ),
    };
    const transport = new MockTransport({
      checkPayload: needsActionPayload(),
      resolveEvents: [
        { kind: 'component', id: 'postgres', patch: { status: 'ready', version: '16.0' } },
      ],
      resolveTerminal: recoveredTerminal,
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
  // The `sensei:health` sessionStorage cache was removed — nothing read it
  // (HealthState's $state is the authoritative source for reroute via
  // appState.healthOk), so the writes were dead. Tests that asserted those
  // writes/removes are gone; what remains is the actual contract:
  // verify() triggers a fresh transport.resolve() and is idempotent in flight.

  it('causes a fresh transport.resolve() call after a prior init()', async () => {
    const transport = new MockTransport({ checkPayload: okPayload() });
    const s = new HealthState(emptyPayload, transport);
    await s.init();
    expect(transport.resolveCalls).toHaveLength(1);
    await s.verify();
    expect(transport.resolveCalls).toHaveLength(2);
  });

  it('concurrent verify() calls trigger only one resolve pass', async () => {
    const transport = new MockTransport({ checkPayload: okPayload() });
    const s = new HealthState(emptyPayload, transport);
    await Promise.all([s.verify(), s.verify()]);
    expect(transport.resolveCalls).toHaveLength(1);
  });

  it('does not throw when sessionStorage is undefined', async () => {
    const transport = new MockTransport({ checkPayload: okPayload() });
    const s = new HealthState(emptyPayload, transport);
    await expect(s.verify()).resolves.toBeUndefined();
  });
});

// HealthState — B4: removed.
// Previously asserted that apply() wrote/removed a `sensei:health`
// sessionStorage cache key. The cache was dead — nothing consumed it —
// so the writes are gone (see health-cache.ts deletion). HealthState's
// $state is the canonical source for reroute via appState.healthOk.

export { okPayload, needsActionPayload, remedyFixture };

// ── makeState helper ────────────────────────────────────────────────────────
// Creates a HealthState instance pre-populated with specific gate statuses
// without going through apply() (which enforces strict invariants). Fields
// are written directly since HealthState uses $state which is publicly writable.

interface MakeStateOpts {
  status?: HealthStatus;
  readyIds?: string[];
  installingId?: string;
  failedId?: string;
  installingVerbs?: Record<string, string>;
  transport?: { retry?: (id: string) => void };
}

function makeComponent(id: string, label: string, opts: MakeStateOpts): Component {
  let status: Component['status'] = 'pending';
  if (opts.readyIds?.includes(id)) status = 'ready';
  if (opts.installingId === id) status = 'installing';
  if (opts.failedId === id) status = 'failed';
  return {
    id: id as Component['id'],
    label,
    detail: null,
    note: null,
    status,
    version: null,
    installingVerb: opts.installingVerbs?.[id] ?? 'installing',
    description: `${label} description`,
  };
}

function makeState(opts: MakeStateOpts): HealthState {
  // Build a minimal transport that satisfies HealthTransport for construction.
  // MockTransport requires a checkPayload so we pass the emptyPayload.
  const baseTransport = new MockTransport({ checkPayload: emptyPayload });
  // Attach an optional retry method from opts.transport if provided.
  const transport = opts.transport
    ? Object.assign(baseTransport, { retry: opts.transport.retry })
    : baseTransport;

  const s = new HealthState(emptyPayload, transport);
  // Override $state fields directly — bypasses apply() invariants so tests
  // can set arbitrary combinations of gate statuses.
  s.status = opts.status ?? 'checking';
  s.packageManager = makeComponent('homebrew', 'Homebrew', opts);
  s.components = [
    makeComponent('postgres', 'PostgreSQL', opts),
    makeComponent('ollama',   'Ollama',     opts),
    makeComponent('sensei',   'Sensei',     opts),
    makeComponent('database', 'Database',   opts),
    makeComponent('daemon',   'Daemon',     opts),
  ];
  return s;
}

// ── New derivation tests ────────────────────────────────────────────────────

describe('HealthState — derivations', () => {
  it('gates returns packageManager + components in that order', () => {
    const s = makeState({});
    expect(s.gates[0].id).toBe(s.packageManager.id);
    expect(s.gates.length).toBe(1 + s.components.length);
  });

  it('total counts all gates', () => {
    const s = makeState({});
    expect(s.total).toBe(6); // pm + 5 components
  });

  it('readyCount counts gates with status="ready"', () => {
    const s = makeState({ readyIds: ['homebrew', 'postgres', 'ollama'] });
    expect(s.readyCount).toBe(3);
  });

  it('activeLabel returns label of first installing/checking gate', () => {
    const s = makeState({
      readyIds: ['homebrew', 'postgres'],
      installingId: 'ollama',
    });
    expect(s.activeLabel).toBe('Ollama');
  });

  it('activeLabel is empty when no gate is active', () => {
    const s = makeState({ readyIds: ['homebrew', 'postgres', 'ollama', 'sensei', 'database', 'daemon'] });
    expect(s.activeLabel).toBe('');
  });

  it('firstBlockedIdx returns index of first failed gate, -1 when none', () => {
    const s1 = makeState({ failedId: 'ollama' }); // index 2 in gates (pm=0, postgres=1, ollama=2)
    expect(s1.firstBlockedIdx).toBe(2);

    const s2 = makeState({});
    expect(s2.firstBlockedIdx).toBe(-1);
  });
});

describe('HealthState — display', () => {
  it('checking status produces "starting" eyebrow', () => {
    const s = makeState({ status: 'checking' });
    expect(s.display.eyebrow).toBe('starting');
    expect(s.display.headlinePre).toBe('Checking the');
    expect(s.display.headlineKey).toBe('foundation.');
    expect(s.display.headlineTone).toBe('accent');
  });

  it('resolving status produces "setting up" eyebrow + accent in-order key', () => {
    const s = makeState({ status: 'resolving' });
    expect(s.display.eyebrow).toBe('setting up');
    expect(s.display.headlineKey).toBe('in order.');
    expect(s.display.headlineTone).toBe('accent');
  });

  it('needs-action status produces "needs your hand" + step. key', () => {
    const s = makeState({ status: 'needs-action' });
    expect(s.display.eyebrow).toBe('needs your hand');
    expect(s.display.headlineKey).toBe('step.');
    expect(s.display.headlineTone).toBe('accent');
  });

  it('ok status produces "ready" + holds. key with success tone', () => {
    const s = makeState({ status: 'ok' });
    expect(s.display.eyebrow).toBe('ready');
    expect(s.display.headlineKey).toBe('holds.');
    expect(s.display.headlineTone).toBe('success');
  });

  it('heroTitle uses installingVerb when status="resolving"', () => {
    const s = makeState({
      status: 'resolving',
      readyIds: ['homebrew', 'postgres'],
      installingId: 'ollama',
      installingVerbs: { ollama: 'installing' },
    });
    expect(s.display.heroTitle).toBe('Installing · 2/6');
  });

  it('heroTitle capitalizes whatever verb the wire provides', () => {
    const s = makeState({
      status: 'resolving',
      readyIds: ['homebrew', 'postgres', 'ollama', 'sensei'],
      installingId: 'database',
      installingVerbs: { database: 'creating' },
    });
    expect(s.display.heroTitle).toBe('Creating · 4/6');
  });

  it('heroTitle is "All systems ready" when status="ok"', () => {
    const s = makeState({ status: 'ok' });
    expect(s.display.heroTitle).toBe('All systems ready');
  });

  it('heroTitle is "Needs your hand" when status="needs-action"', () => {
    const s = makeState({ status: 'needs-action' });
    expect(s.display.heroTitle).toBe('Needs your hand');
  });
});

describe('HealthState — retry()', () => {
  it('retry(id) triggers a check for the given gate via transport', () => {
    const calls: string[] = [];
    const s = makeState({ transport: { retry: (id: string) => { calls.push(id); } } });
    s.retry('ollama');
    expect(calls).toEqual(['ollama']);
  });
});
