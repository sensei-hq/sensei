// @vitest-environment jsdom
import { describe, it, expect, afterEach } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import ToolBubbles from './ToolBubbles.svelte';
import type { ToolUsage } from '$lib/metrics/metric-view.js';

let cleanup: Array<() => void> = [];
afterEach(() => {
    cleanup.forEach((fn) => fn());
    cleanup = [];
});
const q = (r: HTMLElement, s: string) => r.querySelector(s) as HTMLElement | null;

function mount(tools: ToolUsage[]) {
    const m = mountComponent(ToolBubbles, { tools });
    cleanup.push(m.destroy);
    return m.container;
}

describe('ToolBubbles', () => {
    it('renders a bubble per tool, sizes the biggest largest, tints failures', () => {
        const root = mount([
            { tool: 'Bash', calls: 100, failed: 0, sessions: 10 },
            { tool: 'mcp__x__editor', calls: 4, failed: 2, sessions: 3 },
        ]);
        expect(root.querySelectorAll('[data-tool]').length).toBe(2);
        const bash = q(root, '[data-tool="Bash"]')!;
        const editor = q(root, '[data-tool="mcp__x__editor"]')!;
        // A larger call count → a larger bubble (area ∝ calls).
        const w = (el: HTMLElement) => parseInt(el.style.width, 10);
        expect(w(bash)).toBeGreaterThan(w(editor));
        // The readable MCP leaf name is shown as the label.
        expect(root.textContent).toContain('editor');
        // A tool that ever failed is warning-tinted; a clean one is accent.
        expect(editor.className).toContain('border-warning');
        expect(bash.className).toContain('border-accent');
    });

    it('shows an honest-empty state with no tools', () => {
        const root = mount([]);
        expect(root.textContent).toContain('No tool usage captured');
        expect(q(root, '[data-tool]')).toBeNull();
    });
});
