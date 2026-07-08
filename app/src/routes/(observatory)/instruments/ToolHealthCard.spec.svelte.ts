// @vitest-environment jsdom
import { describe, it, expect, afterEach, vi } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import type { ToolHealthSource } from '$lib/types.js';
import ToolHealthCardHarness from './ToolHealthCard.harness.svelte';

let cleanup: Array<() => void> = [];
afterEach(() => { cleanup.forEach((fn) => fn()); cleanup = []; });

// A probed MCP source with a real registered/invoked split → share bar.
const probed: ToolHealthSource = {
  assistant_family: 'claude',
  source_type: 'mcp',
  source_key: 'plugin_sensei_sensei',
  name: 'plugin_sensei_sensei',
  connected: true,
  connection_state: 'connected',
  server_id: 'b2c22577',
  tools_registered: 33,
  tools_invoked_14d: 6,
  calls_14d: 122,
  share_invoked: 0.18181818181818182,
};

// A server that could never be started → tools_registered null, no bar.
const unprobed: ToolHealthSource = {
  assistant_family: 'claude',
  source_type: 'mcp',
  source_key: 'plugin_svelte_svelte',
  name: 'plugin_svelte_svelte',
  connected: false,
  connection_state: null,
  server_id: null,
  tools_registered: null,
  tools_invoked_14d: 2,
  calls_14d: 156,
  share_invoked: null,
};

describe('ToolHealthCard', () => {
  it('renders a share bar and "invoked N of M tools" for a probed source', () => {
    const m = mountComponent(ToolHealthCardHarness, { source: probed });
    cleanup.push(m.destroy);
    const bar = m.container.querySelector('[data-testid="share-bar"]');
    expect(bar).not.toBeNull();
    // No honest-degrade line when the source was probed.
    expect(m.container.querySelector('[data-testid="registered-none"]')).toBeNull();
    // Rounded share percentage + the invoked/registered detail.
    expect(m.container.textContent).toContain('18%');
    expect(m.container.textContent).toContain('invoked 6 of 33 tools');
    // Inner fill width reflects the share fraction.
    const fill = bar!.querySelector('div') as HTMLElement;
    expect(fill.getAttribute('style')).toContain('width: 18%');
  });

  it('renders "registered —" and NO bar when tools_registered is null', () => {
    const m = mountComponent(ToolHealthCardHarness, { source: unprobed });
    cleanup.push(m.destroy);
    // Honest degrade — no share bar at all, not a zero bar.
    expect(m.container.querySelector('[data-testid="share-bar"]')).toBeNull();
    const degrade = m.container.querySelector('[data-testid="registered-none"]');
    expect(degrade).not.toBeNull();
    expect(degrade!.textContent).toContain('invoked 2');
    expect(degrade!.textContent).toContain('registered —');
    // Should not fabricate a percentage.
    expect(m.container.textContent).not.toContain('%');
  });

  it('marks probed state and source type on the card for querying', () => {
    const m = mountComponent(ToolHealthCardHarness, { source: probed });
    cleanup.push(m.destroy);
    const card = m.container.querySelector('[data-testid="tool-health-card-plugin_sensei_sensei"]') as HTMLElement;
    expect(card).not.toBeNull();
    expect(card.getAttribute('data-probed')).toBe('true');
    expect(card.getAttribute('data-source-type')).toBe('mcp');
  });

  it('shows the calls chip with the 14d total', () => {
    const m = mountComponent(ToolHealthCardHarness, { source: probed });
    cleanup.push(m.destroy);
    expect(m.container.textContent).toContain('122 calls · 14d');
  });

  it('dims the card and shows "off" for a disconnected source', () => {
    const m = mountComponent(ToolHealthCardHarness, { source: unprobed });
    cleanup.push(m.destroy);
    const card = m.container.querySelector('[data-testid="tool-health-card-plugin_svelte_svelte"]') as HTMLElement;
    expect(card.className).toContain('opacity-60');
    expect(card.textContent).toContain('off');
  });

  it('invokes onopen with the source when the card is clicked (drills to L2)', () => {
    const onopen = vi.fn();
    const m = mountComponent(ToolHealthCardHarness, { source: probed, onopen });
    cleanup.push(m.destroy);
    const card = m.container.querySelector('[data-testid="tool-health-card-plugin_sensei_sensei"]') as HTMLElement;
    card.click();
    expect(onopen).toHaveBeenCalledTimes(1);
    expect(onopen.mock.calls[0][0]).toMatchObject({ source_key: 'plugin_sensei_sensei' });
  });
});
