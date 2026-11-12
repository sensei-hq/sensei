// @vitest-environment jsdom
import { describe, it, expect, afterEach, vi } from 'vitest';
import { flushSync } from 'svelte';
import { mountComponent } from '$lib/test-mount.js';
import CreateSprintForm from './CreateSprintForm.svelte';
import * as F from '$lib/test-stubs/relay-fixtures.js';
import type { RelayCreateRun } from '$lib/relay-api.js';

// Queue cutover (phase 4): the form the queue tab opens over a selection.
// Submit stays off until a title, a goal and (with auto-merge on) a base
// checkout are there; the body it emits is exactly what the daemon takes.

let cleanup: Array<() => void> = [];
afterEach(() => {
  cleanup.forEach((fn) => fn());
  cleanup = [];
});

type Body = Omit<RelayCreateRun, 'item_ids' | 'crew_id'>;

function mount(over: Partial<{ error: string | null; onsubmit: (body: Body) => Promise<string | null> }> = {}) {
  const onsubmit = over.onsubmit ?? vi.fn(async () => null);
  const oncancel = vi.fn();
  const m = mountComponent(CreateSprintForm, {
    selected: [F.backlogItem(), F.backlogItem({ item_id: 'b2', title: 'Two' })],
    defaults: { base_branch: 'develop', base_checkout: '', verify_cmd: 'make test-fast' },
    error: over.error ?? null,
    onsubmit,
    oncancel,
  });
  cleanup.push(m.destroy);
  const root = m.container.querySelector('[data-create-sprint]') as HTMLElement;
  const field = (name: string) => root.querySelector(`[data-field="${name}"]`) as HTMLInputElement;
  const type = (name: string, value: string) => {
    const el = field(name);
    el.value = value;
    el.dispatchEvent(new Event('input', { bubbles: true }));
    flushSync();
  };
  const submit = () => root.querySelector('[data-sprint-submit]') as HTMLButtonElement;
  return { root, field, type, submit, onsubmit, oncancel };
}

describe('CreateSprintForm', () => {
  it('lists the selection and keeps submit off until title, goal and checkout are there', () => {
    const { root, type, submit, field } = mount();
    expect(root.querySelectorAll('[data-sprint-item]')).toHaveLength(2);
    expect(submit().disabled).toBe(true);
    type('title', 'Sprint 15');
    type('goal', 'Run a sprint from the screen');
    // Auto-merge is on by default, so the gate needs a base checkout.
    expect((field('auto_merge') as HTMLInputElement).checked).toBe(true);
    expect(submit().disabled).toBe(true);
    type('base_checkout', '/w/sensei');
    expect(submit().disabled).toBe(false);
  });

  it('emits the body the daemon takes', async () => {
    const onsubmit = vi.fn(async () => null);
    const { type, submit } = mount({ onsubmit });
    type('title', 'Sprint 15');
    type('goal', 'Run a sprint from the screen');
    type('base_checkout', '/w/sensei');
    submit().click();
    await vi.waitFor(() => expect(onsubmit).toHaveBeenCalledTimes(1));
    expect(onsubmit.mock.calls[0][0]).toEqual({
      title: 'Sprint 15',
      goal: 'Run a sprint from the screen',
      repo_key: null,
      workspace_id: null,
      config: { base_checkout: '/w/sensei', base_branch: 'develop', verify_cmd: 'make test-fast', auto_merge: true },
    });
  });

  it('with auto-merge off the checkout is optional and an empty one goes as null', async () => {
    const onsubmit = vi.fn(async () => null);
    const { type, submit, field } = mount({ onsubmit });
    type('title', 'Sprint 15');
    type('goal', 'Hold the merges');
    const toggle = field('auto_merge');
    toggle.click();
    flushSync();
    expect(toggle.checked).toBe(false);
    expect(submit().disabled).toBe(false);
    submit().click();
    await vi.waitFor(() => expect(onsubmit).toHaveBeenCalledTimes(1));
    expect(onsubmit.mock.calls[0][0].config).toEqual({
      base_checkout: null,
      base_branch: 'develop',
      verify_cmd: 'make test-fast',
      auto_merge: false,
    });
  });

  it('shows the refusal it was given and cancels', () => {
    const { root, oncancel } = mount({ error: 'a run needs a repository' });
    expect((root.querySelector('[data-sprint-error]') as HTMLElement).textContent).toContain('a run needs a repository');
    (root.querySelector('[data-sprint-cancel]') as HTMLButtonElement).click();
    expect(oncancel).toHaveBeenCalledTimes(1);
  });
});
