import { beforeEach, describe, expect, it, vi } from 'vitest';
import { toolVerdict, toolHealthView, toolUsageTotals, toolSourceView } from './mcp.svelte.js';

// Stub the appstate module so importing the mcp store doesn't drag in
// the whole Tauri appstate.svelte pipeline.
vi.mock('$lib/appstate.svelte.js', () => ({
    appState: { port: 7744 },
}));

// Stub senseiApi with call trackers we assert against later.
const mocks = {
    tryGetToolUsage: vi.fn(),
    tryGetToolSignals: vi.fn(),
    tryGetToolInsights: vi.fn(),
    tryGetToolsHealth: vi.fn(),
    tryGetSessionReplay: vi.fn(),
    tryGetMcpServerTools: vi.fn(),
    getMcpServers: vi.fn(),
    setMcpServerEnabled: vi.fn(),
    refreshMcpServers: vi.fn(),
    tryRegisterMcpServer: vi.fn(),
    tryRemoveMcpServer: vi.fn(),
};

// A representative slice of the tools-health wire: two connected probed
// sources (builtin + sensei MCP), one connected zero-share source, and one
// disconnected un-probed source (tools_registered null → no share).
const HEALTH_SOURCES = [
    { assistant_family: 'claude', source_type: 'builtin', source_key: 'builtin', name: 'builtin', connected: true, connection_state: null, server_id: null, tools_registered: 21, tools_invoked_14d: 19, calls_14d: 46697, share_invoked: 0.9047619047619048 },
    { assistant_family: 'claude', source_type: 'mcp', source_key: 'plugin_sensei_sensei', name: 'plugin_sensei_sensei', connected: true, connection_state: 'connected', server_id: 'b2c22577', tools_registered: 33, tools_invoked_14d: 6, calls_14d: 122, share_invoked: 0.18181818181818182 },
    { assistant_family: 'claude', source_type: 'mcp', source_key: 'plugin_semgrep_semgrep', name: 'plugin_semgrep_semgrep', connected: true, connection_state: 'connected', server_id: '6b9ab64a', tools_registered: 7, tools_invoked_14d: 0, calls_14d: 0, share_invoked: 0.0 },
    { assistant_family: 'claude', source_type: 'mcp', source_key: 'plugin_svelte_svelte', name: 'plugin_svelte_svelte', connected: false, connection_state: null, server_id: null, tools_registered: null, tools_invoked_14d: 2, calls_14d: 156, share_invoked: null },
];

vi.mock('$lib/api.js', async (importOriginal) => ({
    failureReason: ((await importOriginal()) as typeof import('$lib/api.js')).failureReason,
    senseiApi: () => mocks,
}));

const okRead = <T,>(data: T) => ({ ok: true as const, data });
const down = { ok: false as const, error: { status: 503, message: 'Service Unavailable' } };

// Re-import fresh for each test so the module-level $state resets.
async function freshStore() {
    vi.resetModules();
    const { mcp } = await import('./mcp.svelte.js');
    return mcp;
}

beforeEach(() => {
    for (const key of Object.keys(mocks)) {
        (mocks as Record<string, ReturnType<typeof vi.fn>>)[key].mockReset();
    }
    mocks.tryGetToolUsage.mockResolvedValue(okRead({ tools: [] }));
    mocks.tryGetToolSignals.mockResolvedValue(okRead({ signals: [], source: 'cache' }));
    mocks.tryGetToolInsights.mockResolvedValue(okRead({ insights: [{ toolName: 'search', computedAt: '2026-07-02T12:00:00Z', metrics: {}, variant: 'win', title: 'Workhorse', detail: 'ok' }] }));
    mocks.tryGetToolsHealth.mockResolvedValue(okRead({ sources: HEALTH_SOURCES }));
    mocks.tryGetSessionReplay.mockResolvedValue(okRead({ sessionId: 's1', calls: [], count: 0, summary: { used: 1, partial: 0, ignored: 0, total: 1 }, classified: 0 }));
    mocks.tryGetMcpServerTools.mockResolvedValue(okRead({ id: 'm', server_id: 'srv-1', tools: [{ name: 'list_issues' }], tool_count: 1, probed_at: '', ttl_seconds: 900, error: null }));
});

describe('toolVerdict', () => {
    it('reads as unused when there are no recorded calls', () => {
        expect(toolVerdict(0, 0)).toBe('unused');
        expect(toolVerdict(0, 3)).toBe('unused');
    });

    it('reads as warn when the error rate is above 10%', () => {
        expect(toolVerdict(10, 2)).toBe('warn');
        // Exactly 10% is tolerable, not a warning.
        expect(toolVerdict(10, 1)).not.toBe('warn');
    });

    it('reads as healthy when calls landed with zero errors', () => {
        expect(toolVerdict(42, 0)).toBe('healthy');
    });

    it('reads as ok for a tolerable non-zero error rate', () => {
        expect(toolVerdict(100, 5)).toBe('ok');
        expect(toolVerdict(10, 1)).toBe('ok');
    });
});

describe('mcp store', () => {
    it('loadCatalog fills stats / signals from the API', async () => {
        const mcp = await freshStore();
        await mcp.loadCatalog();
        expect(mcp.catalogStatus).toBe('ready');
        expect(mcp.signalSource).toBe('cache');
        expect(mocks.tryGetToolUsage).toHaveBeenCalledTimes(1);
    });

    it('loadCatalog is idempotent — a second call hits no endpoints', async () => {
        const mcp = await freshStore();
        await mcp.loadCatalog();
        await mcp.loadCatalog();
        expect(mocks.tryGetToolUsage).toHaveBeenCalledTimes(1);
    });

    it('loadCatalog(force=true) re-fetches even after ready', async () => {
        const mcp = await freshStore();
        await mcp.loadCatalog();
        await mcp.loadCatalog(true);
        expect(mocks.tryGetToolUsage).toHaveBeenCalledTimes(2);
    });

    it('insightFor returns the cached insight row by tool name', async () => {
        const mcp = await freshStore();
        await mcp.loadInsights();
        expect(mcp.insightFor('search')?.variant).toBe('win');
        expect(mcp.insightFor('missing')).toBeUndefined();
    });

    it('refresh re-loads catalog + insights', async () => {
        const mcp = await freshStore();
        await mcp.loadCatalog();
        await mcp.loadInsights();
        await mcp.refresh();
        expect(mocks.tryGetToolUsage).toHaveBeenCalledTimes(2);
        expect(mocks.tryGetToolInsights).toHaveBeenCalledTimes(2);
    });

    // #288: a failed read is an error with its reason — the fallback reads
    // never threw, so this status could never become 'error' before.
    for (const read of ['tryGetToolUsage', 'tryGetToolSignals'] as const) {
        it(`a failed ${read} marks the catalog in error, with the reason`, async () => {
            mocks[read].mockResolvedValueOnce(down);
            const mcp = await freshStore();
            await mcp.loadCatalog();
            expect([mcp.catalogStatus, mcp.catalogError]).toEqual(['error', 'Service Unavailable']);
        });
    }

    it('a failed insights read marks insights in error, with the reason', async () => {
        mocks.tryGetToolInsights.mockResolvedValueOnce(down);
        const mcp = await freshStore();
        await mcp.loadInsights();
        expect([mcp.insightsStatus, mcp.insightsError, mcp.toolInsights]).toEqual(['error', 'Service Unavailable', []]);
    });
});

describe('mcp store — tools-health L1 slice', () => {
    it('loadToolsHealth fills the slice and marks status ready', async () => {
        const mcp = await freshStore();
        await mcp.loadToolsHealth();
        expect(mcp.toolsHealthStatus).toBe('ready');
        expect(mcp.toolsHealth.length).toBe(4);
        expect(mocks.tryGetToolsHealth).toHaveBeenCalledTimes(1);
    });

    it('loadToolsHealth is idempotent — a second call hits no endpoint', async () => {
        const mcp = await freshStore();
        await mcp.loadToolsHealth();
        await mcp.loadToolsHealth();
        expect(mocks.tryGetToolsHealth).toHaveBeenCalledTimes(1);
    });

    it('sorts connected sources first, then by 14d call volume desc', async () => {
        const mcp = await freshStore();
        await mcp.loadToolsHealth();
        const keys = mcp.toolsHealth.map((s) => s.source_key);
        // Connected (builtin 46697, sensei 122, semgrep 0) precede the
        // disconnected svelte source; within connected, calls descend.
        expect(keys).toEqual([
            'builtin',
            'plugin_sensei_sensei',
            'plugin_semgrep_semgrep',
            'plugin_svelte_svelte',
        ]);
        expect(mcp.toolsHealth.at(-1)?.connected).toBe(false);
    });

    it('derives header KPIs from the grid rows', async () => {
        const mcp = await freshStore();
        await mcp.loadToolsHealth();
        const k = mcp.toolsHealthKpis;
        expect(k.total).toBe(4);
        expect(k.connected).toBe(3);
        // Coverage counts probed sources only (svelte's null registered is
        // excluded from both numerator and denominator).
        expect(k.sumRegistered).toBe(21 + 33 + 7);
        expect(k.sumInvoked).toBe(19 + 6 + 0);
        expect(k.coverage).toBeCloseTo(25 / 61, 6);
        // Total calls span every source, connected or not.
        expect(k.totalCalls).toBe(46697 + 122 + 0 + 156);
    });

    it('coverage is null (not 0%) when no source has been probed', async () => {
        mocks.tryGetToolsHealth.mockResolvedValueOnce(okRead({
            sources: [
                { assistant_family: 'claude', source_type: 'mcp', source_key: 'x', name: 'x', connected: false, connection_state: null, server_id: null, tools_registered: null, tools_invoked_14d: 1, calls_14d: 4, share_invoked: null },
            ],
        }));
        const mcp = await freshStore();
        await mcp.loadToolsHealth();
        expect(mcp.toolsHealthKpis.coverage).toBeNull();
        expect(mcp.toolsHealthKpis.sumRegistered).toBe(0);
    });

    it('a failed tools-health read is an error, with the reason', async () => {
        mocks.tryGetToolsHealth.mockResolvedValueOnce(down);
        const mcp = await freshStore();
        await mcp.loadToolsHealth();
        expect([mcp.toolsHealthStatus, mcp.toolsHealthError]).toEqual(['error', 'Service Unavailable']);
    });

    it('refresh re-loads the tools-health slice', async () => {
        const mcp = await freshStore();
        await mcp.loadToolsHealth();
        await mcp.refresh();
        expect(mocks.tryGetToolsHealth).toHaveBeenCalledTimes(2);
    });
});

// A failed read or write must not look like a quiet success: the list used to
// come back `[]` on failure ("No MCP servers discovered") and the toggle flipped
// its row whether or not the daemon took it.
describe('discovered MCP servers', () => {
    const SERVER = { id: 'srv-1', acp_family: 'claude', mcp_key: 'github', enabled: true };
    const fail = { ok: false, error: { status: 500, message: 'Internal Server Error' } };

    it('loadMcpServers fills the list from the daemon', async () => {
        mocks.getMcpServers.mockResolvedValue({ ok: true, data: { servers: [SERVER] } });
        const mcp = await freshStore();
        await mcp.loadMcpServers();
        expect(mcp.mcpServersStatus).toBe('ready');
        expect(mcp.mcpServers.map((s) => s.mcp_key)).toEqual(['github']);
    });

    it('loadMcpServers reports a failed read as an error, not as no servers', async () => {
        mocks.getMcpServers.mockResolvedValue(fail);
        const mcp = await freshStore();
        await mcp.loadMcpServers();
        expect(mcp.mcpServersStatus).toBe('error');
    });

    it('toggleMcpServer flips the row once the daemon took it', async () => {
        mocks.getMcpServers.mockResolvedValue({ ok: true, data: { servers: [SERVER] } });
        mocks.setMcpServerEnabled.mockResolvedValue({ ok: true, data: undefined });
        const mcp = await freshStore();
        await mcp.loadMcpServers();
        expect(await mcp.toggleMcpServer('srv-1', false)).toBeNull();
        expect(mcp.mcpServers[0].enabled).toBe(false);
    });

    it('toggleMcpServer leaves the row as it was when the daemon refused, and says why', async () => {
        mocks.getMcpServers.mockResolvedValue({ ok: true, data: { servers: [SERVER] } });
        mocks.setMcpServerEnabled.mockResolvedValue(fail);
        const mcp = await freshStore();
        await mcp.loadMcpServers();
        expect(await mcp.toggleMcpServer('srv-1', false)).toMatch(/500/);
        expect(mcp.mcpServers[0].enabled).toBe(true);
    });

    // #343: a server added by hand is the daemon's row once it took it.
    it('registerMcpServer reloads the list once the daemon took it, and says why it did not', async () => {
        const server = { name: 'gh', command: 'npx', args: [], env: {} };
        mocks.tryRegisterMcpServer.mockResolvedValueOnce({ ok: true, data: { id: 'm1' } });
        mocks.getMcpServers.mockResolvedValue({ ok: true, data: { servers: [{ ...SERVER, id: 'm1', acp_family: 'manual' }] } });
        const mcp = await freshStore();
        expect(await mcp.registerMcpServer(server)).toBeNull();
        expect(mocks.tryRegisterMcpServer).toHaveBeenCalledWith(server);
        expect(mcp.mcpServers.map((s) => s.id)).toEqual(['m1']);

        mocks.tryRegisterMcpServer.mockResolvedValueOnce({ ok: false, error: { status: 409, message: 'a server is already registered under that name' } });
        expect(await mcp.registerMcpServer(server)).toMatch(/already registered/);
    });

    it('removeMcpServer drops the row once the daemon removed it, and keeps it when refused', async () => {
        mocks.getMcpServers.mockResolvedValue({ ok: true, data: { servers: [SERVER] } });
        const mcp = await freshStore();
        await mcp.loadMcpServers();
        mocks.tryRemoveMcpServer.mockResolvedValueOnce(fail);
        expect(await mcp.removeMcpServer('srv-1')).toMatch(/500/);
        expect(mcp.mcpServers).toHaveLength(1);
        mocks.tryRemoveMcpServer.mockResolvedValueOnce({ ok: true, data: { removed: true } });
        expect(await mcp.removeMcpServer('srv-1')).toBeNull();
        expect(mcp.mcpServers).toHaveLength(0);
    });
});

// #288: a session's replay and a server's tools are the daemon's, or why not.
describe('replay and server tools', () => {
    it('loadSessionReplay caches a read, and says why one failed', async () => {
        const mcp = await freshStore();
        expect((await mcp.loadSessionReplay('s1')).error).toBeNull();
        await mcp.loadSessionReplay('s1');
        expect(mocks.tryGetSessionReplay).toHaveBeenCalledTimes(1);
        mocks.tryGetSessionReplay.mockResolvedValueOnce(down);
        const failed = await mcp.loadSessionReplay('s2');
        expect([failed.replay, failed.error]).toEqual([null, 'Service Unavailable']);
    });

    it('loadServerTools records a failed read per server — not a server with no tools', async () => {
        mocks.tryGetMcpServerTools.mockResolvedValueOnce(down);
        const mcp = await freshStore();
        await mcp.loadServerTools('srv-1');
        expect([mcp.serverToolManifests['srv-1'], mcp.serverToolErrors['srv-1']]).toEqual([undefined, 'Service Unavailable']);
        await mcp.loadServerTools('srv-1');
        expect([mcp.serverToolManifests['srv-1']?.tool_count, mcp.serverToolErrors['srv-1']]).toEqual([1, undefined]);
    });
});

// Audit item 8 (2026-10-28): the per-tool error rate turned red at its own
// inline >0.1 beside the verdict chip's >0.1 — two copies of one rule — and the
// workspace screen turned red at ANY error. One view, one threshold.
describe('toolHealthView', () => {
  it('decides the verdict, its colour and the error rate from one threshold', () => {
    expect(toolHealthView(100, 0)).toEqual({ verdict: 'healthy', verdictClass: 'text-success', errorRate: '', errorClass: '' });
    expect(toolHealthView(100, 5)).toEqual({ verdict: 'ok', verdictClass: 'text-ink-mute', errorRate: '(5%)', errorClass: '' });
    expect(toolHealthView(100, 11)).toEqual({ verdict: 'warn', verdictClass: 'text-warning', errorRate: '(11%)', errorClass: 'text-danger' });
    expect(toolHealthView(0, 0)).toEqual({ verdict: 'unused', verdictClass: 'text-ink-soft', errorRate: '', errorClass: '' });
  });

  it('turns the error count red exactly when the verdict warns', () => {
    for (const [calls, errors] of [[10, 1], [10, 2], [3, 1], [1000, 100], [1000, 101]]) {
      const v = toolHealthView(calls, errors);
      expect(v.errorClass === 'text-danger', `${errors}/${calls}`).toBe(v.verdict === 'warn');
    }
  });
});

describe('toolUsageTotals', () => {
  // The drill-in's "total calls" sat one click from the grid's "total tool
  // calls" — different populations (sensei's own tools, all time, vs every
  // source over 14 days) under the same words.
  it('totals sensei\'s own tools and says which population it is', () => {
    const t = toolUsageTotals([
      { call_count: 90, error_count: 9 },
      { call_count: 10, error_count: 0 }
    ] as never);
    expect(t).toEqual({ calls: 100, errors: 9, errorRate: '9.0% rate', errorClass: 'text-warning', tools: 2, scope: 'sensei tools · all time' });
    expect(toolUsageTotals([]).errorRate).toBe('—');
  });
});

describe('toolSourceView', () => {
  // The coverage card decided its own state inline, with a local <70% rule.
  it('a probed source shows its invoked share, warning under the coverage floor', () => {
    const src = { source_type: 'mcp', tools_registered: 10, tools_invoked_14d: 6, share_invoked: 0.6 };
    expect(toolSourceView(src as never)).toEqual({ probed: true, registered: 10, pct: 60, pctLabel: '60%', pctClass: 'text-warning', kanji: '接' });
    expect(toolSourceView({ ...src, share_invoked: null } as never)).toMatchObject({ probed: true, pct: null, pctLabel: '—' });
    expect(toolSourceView({ ...src, share_invoked: 0.7 } as never).pctClass).toBe('text-ink');
  });

  it('an unprobed source draws no share — never a fabricated 0%', () => {
    const v = toolSourceView({ source_type: 'builtin', tools_registered: null, tools_invoked_14d: 3, share_invoked: null } as never);
    expect(v).toEqual({ probed: false, registered: null, pct: null, pctLabel: '—', pctClass: '', kanji: '器' });
  });
});
