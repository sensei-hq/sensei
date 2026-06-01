// @vitest-environment jsdom
import { describe, it, expect, afterEach } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import AssistantCard from './AssistantCard.svelte';

let cleanup: Array<() => void> = [];
afterEach(() => { cleanup.forEach((fn) => fn()); cleanup = []; });

const PARTS_CLAUDE = [
  { id: 'plugins',  label: 'plugins',  status: 'idle'   as const },
  { id: 'skills',   label: 'skills',   status: 'idle'   as const },
  { id: 'commands', label: 'commands', status: 'idle'   as const },
];

function card(m: { container: HTMLElement }, id: string): HTMLElement {
  return m.container.querySelector(`[data-testid="assistant-card-${id}"]`) as HTMLElement;
}

describe('AssistantCard', () => {
  it('renders the family name and a chip per part', () => {
    const m = mountComponent(AssistantCard, {
      id: 'claude', name: 'Claude', found: true, enabled: true, parts: PARTS_CLAUDE,
    });
    cleanup.push(m.destroy);
    const root = card(m, 'claude');
    expect(root).toBeTruthy();
    expect(root.textContent).toContain('Claude');
    const chips = root.querySelectorAll('.chip');
    expect(chips).toHaveLength(PARTS_CLAUDE.length);
    expect(chips[0].textContent).toContain('plugins');
  });

  it('renders "not found" header label when found=false', () => {
    // The wizard greys these cards down — we lock the data-found attr so a
    // styling refactor that drops the class still leaves the e2e selector
    // working, and the label text so the user-visible signal is asserted.
    const m = mountComponent(AssistantCard, {
      id: 'kiro', name: 'Kiro', found: false, enabled: false,
      parts: [{ id: 'mcp', label: 'mcp server', status: 'idle' }],
    });
    cleanup.push(m.destroy);
    const root = card(m, 'kiro');
    expect(root.getAttribute('data-found')).toBe('false');
    expect(root.textContent).toContain('not found');
  });

  it('renders "off" header label when enabled=false but found', () => {
    const m = mountComponent(AssistantCard, {
      id: 'cursor', name: 'Cursor', found: true, enabled: false,
      parts: [{ id: 'mcp', label: 'mcp server', status: 'idle' }],
    });
    cleanup.push(m.destroy);
    expect(card(m, 'cursor').textContent).toContain('off');
  });

  it('renders "configuring…" header label when any part is configuring', () => {
    const m = mountComponent(AssistantCard, {
      id: 'claude', name: 'Claude', found: true, enabled: true,
      parts: [
        { id: 'plugins', label: 'plugins', status: 'configuring' },
        { id: 'skills',  label: 'skills',  status: 'idle' },
      ],
    });
    cleanup.push(m.destroy);
    expect(card(m, 'claude').textContent).toMatch(/configuring/);
  });

  it('renders "configured" header label only when every part is done', () => {
    // The mockup is explicit: a mix of done + idle still reads as
    // "not yet finished". Pin this so a refactor doesn't accidentally
    // flip the card to "configured" while a part is still pending.
    const m1 = mountComponent(AssistantCard, {
      id: 'claude', name: 'Claude', found: true, enabled: true,
      parts: [
        { id: 'plugins', label: 'plugins', status: 'done' },
        { id: 'skills',  label: 'skills',  status: 'idle' },
      ],
    });
    cleanup.push(m1.destroy);
    expect(card(m1, 'claude').textContent).not.toMatch(/configured/);

    const m2 = mountComponent(AssistantCard, {
      id: 'claude', name: 'Claude', found: true, enabled: true,
      parts: [
        { id: 'plugins', label: 'plugins', status: 'done' },
        { id: 'skills',  label: 'skills',  status: 'done' },
      ],
    });
    cleanup.push(m2.destroy);
    expect(card(m2, 'claude').textContent).toMatch(/configured/);
  });

  it('renders consolidated error block + retry when error and any part failed', () => {
    // Retry button is rendered only when an onRetry handler is supplied —
    // pass a noop so the assertion exercises both the error block and the
    // retry affordance.
    const m = mountComponent(AssistantCard, {
      id: 'claude', name: 'Claude', found: true, enabled: true,
      parts: [{ id: 'plugins', label: 'plugins', status: 'error' }],
      error: '~/.claude/agents — permission denied',
      onRetry: () => {},
    });
    cleanup.push(m.destroy);
    const root = card(m, 'claude');
    expect(root.querySelector('[role="alert"]')).toBeTruthy();
    expect(root.textContent).toContain('permission denied');
    expect(root.querySelector('button.retry')?.textContent).toMatch(/Retry/);
  });

  it('omits retry button when onRetry is not provided', () => {
    const m = mountComponent(AssistantCard, {
      id: 'claude', name: 'Claude', found: true, enabled: true,
      parts: [{ id: 'plugins', label: 'plugins', status: 'error' }],
      error: 'no callback supplied',
    });
    cleanup.push(m.destroy);
    expect(card(m, 'claude').querySelector('button.retry')).toBeFalsy();
  });

  it('hides error block while configuring (busy state suppresses it)', () => {
    // If configure() relaunched but the previous failure message is still
    // hanging around, we shouldn't display it under the chip strip — the
    // chip's own spinner is the active signal. Lock the suppression.
    const m = mountComponent(AssistantCard, {
      id: 'claude', name: 'Claude', found: true, enabled: true,
      parts: [{ id: 'plugins', label: 'plugins', status: 'configuring' }],
      error: 'prior failure that should not show during a retry',
    });
    cleanup.push(m.destroy);
    expect(card(m, 'claude').querySelector('[role="alert"]')).toBeFalsy();
  });

  it('chips forced to idle styling when enabled=false (even with done status)', () => {
    // Daemon truth says a part is done, but the user has toggled the
    // switch off — the chips should display as idle to reflect intent.
    const m = mountComponent(AssistantCard, {
      id: 'claude', name: 'Claude', found: true, enabled: false,
      parts: [{ id: 'plugins', label: 'plugins', status: 'done' }],
    });
    cleanup.push(m.destroy);
    const chip = card(m, 'claude').querySelector('.chip') as HTMLElement;
    expect(chip.getAttribute('data-status')).toBe('idle');
  });
});
