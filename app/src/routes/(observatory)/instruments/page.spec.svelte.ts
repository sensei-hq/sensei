// @vitest-environment jsdom
import { describe, it, expect, afterEach, vi } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import Page from './+page.svelte';

let cleanup: Array<() => void> = [];
afterEach(() => { cleanup.forEach((fn) => fn()); cleanup = []; vi.unstubAllGlobals(); });

const json = (body: unknown) => ({ ok: true, status: 200, statusText: 'OK', json: async () => body });

// #288: a server list that could not be read is said so, and not re-asked
// without end.
describe('Instruments /+page — a failed server read', () => {
  it('says the servers could not be read, and stops asking', async () => {
    const fetch = vi.fn().mockResolvedValue({ ok: false, status: 503, statusText: 'Service Unavailable', json: async () => ({}) });
    vi.stubGlobal('fetch', fetch);
    const m = mountComponent(Page, {});
    cleanup.push(m.destroy);
    await vi.waitFor(() =>
      expect(m.container.querySelector('[data-testid="servers-load-error"]')).not.toBeNull(),
    );
    await new Promise((r) => setTimeout(r, 200));
    const settled = fetch.mock.calls.length;
    await new Promise((r) => setTimeout(r, 300));
    expect(fetch.mock.calls.length).toBe(settled);
  });
});

// The Playground is what an assistant sees: sensei's server first, its tools
// as the server lists them, a form from each tool's schema, and a call that
// runs through the server.
describe('Instruments /+page — the Playground', () => {
  it('lists the server’s tools, builds the form from the schema, and runs the tool', async () => {
    const servers = { servers: [{ id: 'se', acp_family: 'claude', mcp_key: 'plugin_sensei_sensei', enabled: true }] };
    const tools = {
      id: 'm', server_id: 'se', tool_count: 1, error: null,
      tools: [{
        name: 'list_library_skills', description: 'Skills a library provides',
        inputSchema: { type: 'object', properties: { library: { type: 'string' } }, required: ['library'] },
      }],
    };
    const call = { result: { content: [{ type: 'text', text: '[{"name":"dbd"}]' }] }, isError: false, duration_ms: 12 };
    const fetch = vi.fn().mockImplementation(async (url: string, init?: { method?: string; body?: string }) => {
      if (url.includes('/mcp-servers/se/call')) return json(call);
      if (url.includes('/mcp-servers/se/tools')) return json(tools);
      if (url.includes('/mcp-servers')) return json(servers);
      if (url.includes('/api/workspaces')) return json([{ id: 'ws-1', name: 'sensei' }]);
      return json({});
    });
    vi.stubGlobal('fetch', fetch);
    const m = mountComponent(Page, {});
    cleanup.push(m.destroy);

    await vi.waitFor(() => expect(m.container.querySelector('[data-testid="tool-row-list_library_skills"]')).not.toBeNull());
    (m.container.querySelector('[data-testid="tool-row-list_library_skills"]') as HTMLButtonElement).click();
    const input = await vi.waitFor(() => {
      const el = m.container.querySelector('[data-testid="tool-input-library"]') as HTMLInputElement | null;
      expect(el).not.toBeNull();
      return el!;
    });
    // What is typed is what is sent: the WebView must not capitalise "sensei"
    // into "Sensei" (user, 2026-10-27).
    expect([input.getAttribute('autocapitalize'), input.getAttribute('autocorrect'), input.getAttribute('spellcheck')]).toEqual(['off', 'off', 'false']);
    input.value = 'dbd';
    input.dispatchEvent(new Event('input', { bubbles: true }));
    (m.container.querySelector('[data-testid="tool-execute"]') as HTMLButtonElement).click();

    await vi.waitFor(() => expect(m.container.querySelector('[data-testid="tool-ok"]')?.textContent).toContain('12 ms'));
    const sent = fetch.mock.calls.find(([u]) => String(u).includes('/call'))!;
    expect(JSON.parse(sent[1].body)).toEqual({ tool: 'list_library_skills', arguments: { library: 'dbd' } });
    expect(m.container.textContent).toContain('"name": "dbd"');
  });
});

// A source is an assistant's: Claude and Zed can both configure a server under
// one key. Keyed without the family, the second was a duplicate key that
// blanked the whole Health grid.
describe('Instruments /+page — Health', () => {
  it('shows a card per assistant for a server two assistants share', async () => {
    const source = (assistant_family: string) => ({
      assistant_family, source_type: 'mcp', source_key: 'context7', name: 'context7', connected: true,
      connection_state: 'connected', server_id: null, tools_registered: 2, tools_invoked_14d: 1, calls_14d: 3, share_invoked: 0.5,
    });
    vi.stubGlobal('fetch', vi.fn().mockImplementation(async (url: string) =>
      json(url.includes('/tools-health') ? { sources: [source('claude'), source('zed')] } : url.includes('/api/workspaces') ? [] : {}),
    ));
    const m = mountComponent(Page, {});
    cleanup.push(m.destroy);
    (m.container.querySelector('[data-testid="instrument-tab-insights"]') as HTMLButtonElement).click();
    await vi.waitFor(() =>
      expect(m.container.querySelectorAll('[data-testid="tool-health-card-context7"]').length).toBe(2),
    );
  });
});

// #343: a server added by hand is registered with the daemon and listed.
describe('Instruments /+page — adding a server', () => {
  it('registers the server from the form and lists it', async () => {
    let servers: unknown[] = [];
    const fetch = vi.fn().mockImplementation(async (url: string, init?: { method?: string; body?: string }) => {
      if (url.endsWith('/mcp-servers') && init?.method === 'POST') {
        const b = JSON.parse(init.body!);
        servers = [{ id: 'm1', acp_family: 'manual', mcp_key: b.name, enabled: true }];
        return json({ id: 'm1' });
      }
      if (url.includes('/mcp-servers/m1/tools')) return json({ tools: [], error: null });
      if (url.includes('/mcp-servers')) return json({ servers });
      if (url.includes('/api/workspaces')) return json([{ id: 'ws-1', name: 'sensei' }]);
      return json({});
    });
    vi.stubGlobal('fetch', fetch);
    const m = mountComponent(Page, {});
    cleanup.push(m.destroy);
    await vi.waitFor(() => expect(m.container.querySelector('[data-testid="add-server"]')).not.toBeNull());
    (m.container.querySelector('[data-testid="add-server"]') as HTMLButtonElement).click();
    const set = async (testid: string, value: string) => {
      const el = await vi.waitFor(() => {
        const e = m.container.querySelector(`[data-testid="${testid}"]`) as HTMLInputElement | null;
        expect(e).not.toBeNull();
        return e!;
      });
      el.value = value;
      el.dispatchEvent(new Event('input', { bubbles: true }));
    };
    await set('add-server-name', 'github');
    await set('add-server-command', 'npx');
    await set('add-server-args', '-y\nserver-github');
    (m.container.querySelector('[data-testid="add-server-form"]') as HTMLFormElement).requestSubmit();

    await vi.waitFor(() => expect(m.container.querySelector('[data-testid="servers-family-manual"]')?.textContent).toContain('Added in sensei'));
    const sent = fetch.mock.calls.find(([u, i]) => String(u).endsWith('/mcp-servers') && i?.method === 'POST')!;
    expect(JSON.parse(sent[1].body)).toEqual({ name: 'github', command: 'npx', args: ['-y', 'server-github'], env: {} });
    expect(m.container.querySelector('[data-testid="server-remove-github"]')).not.toBeNull();
  });
});
