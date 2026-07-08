import { beforeEach, describe, expect, it, vi } from 'vitest';
import { toolVerdict } from './mcp.svelte.js';

// Stub the appstate module so importing the mcp store doesn't drag in
// the whole Tauri appstate.svelte pipeline.
vi.mock('$lib/appstate.svelte.js', () => ({
    appState: { port: 7744 },
}));

// Stub senseiApi with call trackers we assert against later.
const mocks = {
    mcpListTools: vi.fn(),
    getToolUsage: vi.fn(),
    getToolSignals: vi.fn(),
    getToolInsights: vi.fn(),
    getToolsHealth: vi.fn(),
    getSessionToolTimeline: vi.fn(),
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

vi.mock('$lib/api.js', () => ({
    senseiApi: () => mocks,
}));

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
    mocks.mcpListTools.mockResolvedValue({ tools: [{ mcp: 'sensei', id: 'sensei.search', name: 'search', kind: 'query', summary: 's', inputs: [], example: { response: '' } }] });
    mocks.getToolUsage.mockResolvedValue({ tools: [] });
    mocks.getToolSignals.mockResolvedValue({ signals: [], source: 'cache' });
    mocks.getToolInsights.mockResolvedValue({ insights: [{ toolName: 'search', computedAt: '2026-07-02T12:00:00Z', metrics: {}, variant: 'win', title: 'Workhorse', detail: 'ok' }] });
    mocks.getToolsHealth.mockResolvedValue({ sources: HEALTH_SOURCES });
    mocks.getSessionToolTimeline.mockResolvedValue({ sessionId: 's1', calls: [{ callId: 1, toolName: 'search', family: 'claude', request: {}, response: null, success: null, startedAtMs: 0, completedAtMs: null, durationMs: null, startedAt: '', completedAt: null, inFlight: true }], count: 1 });
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
    it('loadCatalog fills tools / stats / signals from the API', async () => {
        const mcp = await freshStore();
        await mcp.loadCatalog();
        expect(mcp.catalogStatus).toBe('ready');
        expect(mcp.tools.length).toBe(1);
        expect(mcp.signalSource).toBe('cache');
        expect(mocks.mcpListTools).toHaveBeenCalledTimes(1);
    });

    it('loadCatalog is idempotent — a second call hits no endpoints', async () => {
        const mcp = await freshStore();
        await mcp.loadCatalog();
        await mcp.loadCatalog();
        expect(mocks.mcpListTools).toHaveBeenCalledTimes(1);
    });

    it('loadCatalog(force=true) re-fetches even after ready', async () => {
        const mcp = await freshStore();
        await mcp.loadCatalog();
        await mcp.loadCatalog(true);
        expect(mocks.mcpListTools).toHaveBeenCalledTimes(2);
    });

    it('insightFor returns the cached insight row by tool name', async () => {
        const mcp = await freshStore();
        await mcp.loadInsights();
        expect(mcp.insightFor('search')?.variant).toBe('win');
        expect(mcp.insightFor('missing')).toBeUndefined();
    });

    it('loadSessionTimeline caches per session id', async () => {
        const mcp = await freshStore();
        await mcp.loadSessionTimeline('s1');
        await mcp.loadSessionTimeline('s1');
        expect(mocks.getSessionToolTimeline).toHaveBeenCalledTimes(1);
        expect(mcp.sessionTimelines['s1'].length).toBe(1);
    });

    it('refresh clears the timeline cache and re-loads catalog + insights', async () => {
        const mcp = await freshStore();
        await mcp.loadCatalog();
        await mcp.loadInsights();
        await mcp.loadSessionTimeline('s1');
        await mcp.refresh();
        expect(mocks.mcpListTools).toHaveBeenCalledTimes(2);
        expect(mocks.getToolInsights).toHaveBeenCalledTimes(2);
        expect(Object.keys(mcp.sessionTimelines)).toHaveLength(0);
    });

    it('marks catalog status error when the API throws', async () => {
        mocks.mcpListTools.mockRejectedValueOnce(new Error('boom'));
        const mcp = await freshStore();
        await mcp.loadCatalog();
        expect(mcp.catalogStatus).toBe('error');
    });
});

describe('mcp store — tools-health L1 slice', () => {
    it('loadToolsHealth fills the slice and marks status ready', async () => {
        const mcp = await freshStore();
        await mcp.loadToolsHealth();
        expect(mcp.toolsHealthStatus).toBe('ready');
        expect(mcp.toolsHealth.length).toBe(4);
        expect(mocks.getToolsHealth).toHaveBeenCalledTimes(1);
    });

    it('loadToolsHealth is idempotent — a second call hits no endpoint', async () => {
        const mcp = await freshStore();
        await mcp.loadToolsHealth();
        await mcp.loadToolsHealth();
        expect(mocks.getToolsHealth).toHaveBeenCalledTimes(1);
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
        mocks.getToolsHealth.mockResolvedValueOnce({
            sources: [
                { assistant_family: 'claude', source_type: 'mcp', source_key: 'x', name: 'x', connected: false, connection_state: null, server_id: null, tools_registered: null, tools_invoked_14d: 1, calls_14d: 4, share_invoked: null },
            ],
        });
        const mcp = await freshStore();
        await mcp.loadToolsHealth();
        expect(mcp.toolsHealthKpis.coverage).toBeNull();
        expect(mcp.toolsHealthKpis.sumRegistered).toBe(0);
    });

    it('marks tools-health status error when the API throws', async () => {
        mocks.getToolsHealth.mockRejectedValueOnce(new Error('boom'));
        const mcp = await freshStore();
        await mcp.loadToolsHealth();
        expect(mcp.toolsHealthStatus).toBe('error');
    });

    it('refresh re-loads the tools-health slice', async () => {
        const mcp = await freshStore();
        await mcp.loadToolsHealth();
        await mcp.refresh();
        expect(mocks.getToolsHealth).toHaveBeenCalledTimes(2);
    });
});
