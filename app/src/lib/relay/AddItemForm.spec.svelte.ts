// @vitest-environment jsdom
// Add an open item to triage (#423; CONTRACT §1d "open items the person adds"): a title
// and its kind; the form calls back and says why a refusal happened. Not in the r1
// mockup — the smallest form the contract needs, for Design to shape.
import { describe, it, expect, afterEach, vi } from 'vitest';
import { tick } from 'svelte';
import { mountComponent } from '$lib/test-mount.js';
import AddItemForm from './AddItemForm.svelte';

let cleanup: Array<() => void> = [];
afterEach(() => {
  cleanup.forEach((fn) => fn());
  cleanup = [];
});

function mount(onsubmit = vi.fn(async () => null as string | null)) {
  const m = mountComponent(AddItemForm, { onsubmit });
  cleanup.push(m.destroy);
  return { c: m.container, onsubmit };
}

const type = (el: HTMLInputElement | HTMLSelectElement, value: string) => {
  el.value = value;
  el.dispatchEvent(new Event('input', { bubbles: true }));
  el.dispatchEvent(new Event('change', { bubbles: true }));
};

describe('AddItemForm', () => {
  it('a title and its kind go out together; the field clears once it is added', async () => {
    const { c, onsubmit } = mount();
    const title = c.querySelector('input[data-add-item-title]') as HTMLInputElement;
    type(title, 'Hook skips the dōjō tests');
    type(c.querySelector('select[data-add-item-kind]') as HTMLSelectElement, 'finding');
    (c.querySelector('[data-add-item-submit]') as HTMLButtonElement).click();
    await vi.waitFor(() => expect(onsubmit).toHaveBeenCalledWith({ title: 'Hook skips the dōjō tests', kind: 'finding' }));
    await tick();
    expect(title.value).toBe('');
  });

  it('an empty title is not sent', async () => {
    const { c, onsubmit } = mount();
    expect((c.querySelector('[data-add-item-submit]') as HTMLButtonElement).disabled).toBe(true);
    expect(onsubmit).not.toHaveBeenCalled();
  });

  it('a refusal says why and keeps what was typed', async () => {
    const { c } = mount(vi.fn(async () => 'title is too long'));
    const title = c.querySelector('input[data-add-item-title]') as HTMLInputElement;
    type(title, 'x'.repeat(10));
    (c.querySelector('[data-add-item-submit]') as HTMLButtonElement).click();
    await vi.waitFor(() => expect(c.querySelector('[data-add-item-error]')?.textContent).toContain('title is too long'));
    expect(title.value).toBe('x'.repeat(10));
  });
});
