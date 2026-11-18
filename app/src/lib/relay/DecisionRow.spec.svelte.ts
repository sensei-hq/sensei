// @vitest-environment jsdom
// The decision card (#389; COMPONENTS §3b DecisionCard): an options decision is buttons and
// Answer, as before; a form decision is the lane's schema drawn by @rokkit/forms, checked
// inline before anything is sent, sent once, and a refusal shown in place with the values kept.
import { describe, it, expect, afterEach, vi } from 'vitest';
import { flushSync, tick } from 'svelte';
import { mountComponent } from '$lib/test-mount.js';
import type { RelayDecision, RelayDecisionChoice } from '@sensei/state';
import * as F from '$lib/test-stubs/relay-fixtures.js';
import DecisionRow from './DecisionRow.svelte';

let cleanup: Array<() => void> = [];
afterEach(() => {
  cleanup.forEach((fn) => fn());
  cleanup = [];
});

type Answer = (choice: RelayDecisionChoice) => Promise<string | null>;

function mount(decision: RelayDecision, onanswer: Answer = vi.fn(async () => null)) {
  const m = mountComponent(DecisionRow as never, { decision, href: null, onanswer } as never);
  cleanup.push(m.destroy);
  return m.container;
}

const field = (c: HTMLElement, name: string) => c.querySelector<HTMLElement>(`[data-decision-field="${name}"]`);
const answerButton = (c: HTMLElement) => c.querySelector<HTMLButtonElement>('[data-decision-answer]')!;
const settle = async () => {
  flushSync();
  await tick();
  await Promise.resolve();
  flushSync();
};

/** Type a number the way the browser reports it: the input's value, then its change event. */
function setNumber(c: HTMLElement, name: string, value: string) {
  const input = field(c, name)!.querySelector<HTMLInputElement>('input[type="number"]')!;
  input.value = value;
  input.dispatchEvent(new Event('input', { bubbles: true }));
  input.dispatchEvent(new Event('change', { bubbles: true }));
}

/** Pick an option in rokkit's Select: open it, then press the option. */
async function pick(c: HTMLElement, name: string, label: string) {
  field(c, name)!.querySelector<HTMLButtonElement>('[data-select-trigger]')!.click();
  await settle();
  const option = [...c.querySelectorAll<HTMLElement>('[data-select-option]')].find((o) => o.textContent?.trim() === label);
  option!.click();
  await settle();
}

describe('DecisionRow · an options decision', () => {
  it('renders buttons with the recommendation marked, and no form', () => {
    const c = mount(F.decision());
    expect([...c.querySelectorAll('[data-option]')].map((o) => o.getAttribute('data-option'))).toEqual(['db', 'file']);
    expect(c.querySelector('[data-decision-form]')).toBeNull();
    expect(c.querySelector('[data-decision-field]')).toBeNull();
  });

  it('Answer sends the chosen option', async () => {
    const onanswer = vi.fn(async () => null);
    const c = mount(F.decision(), onanswer);
    answerButton(c).click();
    await settle();
    expect(onanswer).toHaveBeenCalledWith({ option: 'db' });
  });
});

describe('DecisionRow · a form decision', () => {
  it('draws one control per schema property: a select for the enum, a number input, a checkbox; no option buttons', () => {
    const c = mount(F.formDecision());
    expect(c.querySelector('[data-decision-form]')).not.toBeNull();
    expect([...c.querySelectorAll('[data-decision-field]')].map((f) => f.getAttribute('data-decision-field'))).toEqual([
      'target',
      'max_retries',
      'ship',
    ]);
    expect(field(c, 'target')!.querySelector('[data-select]')).not.toBeNull();
    expect(field(c, 'max_retries')!.querySelector('input[type="number"]')).not.toBeNull();
    expect(field(c, 'ship')!.querySelector('input[type="checkbox"]')).not.toBeNull();
    expect(c.querySelector('[data-option]')).toBeNull();
    // Labels from the schema: the title, else the name in words.
    expect(field(c, 'target')!.textContent).toContain('Target branch');
    expect(field(c, 'max_retries')!.textContent).toContain('Max retries');
  });

  it('the yes/no field is a native checkbox a keyboard reaches; nothing focusable is hidden from assistive tech', () => {
    const c = mount(F.formDecision());
    const box = field(c, 'ship')!.querySelector<HTMLInputElement>('input[type="checkbox"]')!;
    expect(box.hidden).toBe(false);
    expect(c.querySelector('[aria-hidden="true"][tabindex]:not([tabindex="-1"])')).toBeNull();
  });

  it('the default seeds its field', () => {
    const c = mount(F.formDecision());
    expect(field(c, 'max_retries')!.querySelector<HTMLInputElement>('input[type="number"]')!.value).toBe('2');
  });

  it('Answer with a required field empty shows the field message and sends nothing', async () => {
    const onanswer = vi.fn(async () => null);
    const c = mount(F.formDecision(), onanswer);
    answerButton(c).click();
    await settle();
    expect(onanswer).not.toHaveBeenCalled();
    expect(field(c, 'target')!.querySelector('[data-message]')?.textContent).toContain('is required');
  });

  it('a value past a bound is refused inline too', async () => {
    const onanswer = vi.fn(async () => null);
    const c = mount(F.formDecision(), onanswer);
    await pick(c, 'target', 'main');
    setNumber(c, 'max_retries', '9');
    await settle();
    answerButton(c).click();
    await settle();
    expect(onanswer).not.toHaveBeenCalled();
    expect(field(c, 'max_retries')!.querySelector('[data-message]')?.textContent).toContain('no more than 5');
  });

  it('a value breaking a pattern is refused inline', async () => {
    const onanswer = vi.fn(async () => null);
    const decision = F.formDecision({
      response: {
        schema: { type: 'object', properties: { path: { type: 'string', pattern: '^/' } }, required: ['path'] },
      },
    });
    const c = mount(decision, onanswer);
    const input = field(c, 'path')!.querySelector<HTMLInputElement>('input')!;
    input.value = 'relative/path';
    input.dispatchEvent(new Event('input', { bubbles: true }));
    input.dispatchEvent(new Event('change', { bubbles: true }));
    await settle();
    answerButton(c).click();
    await settle();
    expect(onanswer).not.toHaveBeenCalled();
    expect(field(c, 'path')!.querySelector('[data-message]')?.textContent).toContain('format is invalid');
  });

  it('Answer sends the values once; a second press while it is in flight sends nothing', async () => {
    let release: (v: string | null) => void = () => {};
    const onanswer = vi.fn(() => new Promise<string | null>((r) => (release = r)));
    const c = mount(F.formDecision(), onanswer);
    await pick(c, 'target', 'main');
    answerButton(c).click();
    await settle();
    expect(answerButton(c).disabled).toBe(true);
    answerButton(c).click();
    await settle();
    expect(onanswer).toHaveBeenCalledTimes(1);
    // The optional ship was never touched: absent, not false (D83). The default counts as filled.
    expect(onanswer).toHaveBeenCalledWith({ values: { target: 'main', max_retries: 2 } });
    release(null);
    await settle();
  });

  it("the daemon's refusal shows in place and the values stay", async () => {
    const onanswer = vi.fn(async () => 'This decision was already answered.');
    const c = mount(F.formDecision(), onanswer);
    await pick(c, 'target', 'main');
    setNumber(c, 'max_retries', '4');
    await settle();
    answerButton(c).click();
    await settle();
    expect(c.querySelector('[data-decision-error]')?.textContent).toContain('This decision was already answered.');
    expect(field(c, 'target')!.querySelector('[data-select-value-text]')?.textContent?.trim()).toBe('main');
    expect(field(c, 'max_retries')!.querySelector<HTMLInputElement>('input[type="number"]')!.value).toBe('4');
    expect(answerButton(c).disabled).toBe(false);
    // Pressed again, it sends the same values.
    answerButton(c).click();
    await settle();
    expect(onanswer).toHaveBeenLastCalledWith({ values: { target: 'main', max_retries: 4 } });
  });

  it('an optional number typed then cleared is left out, and Answer still sends', async () => {
    const onanswer = vi.fn(async () => null);
    const c = mount(F.formDecision(), onanswer);
    await pick(c, 'target', 'main');
    setNumber(c, 'max_retries', '3');
    await settle();
    setNumber(c, 'max_retries', '');
    await settle();
    answerButton(c).click();
    await settle();
    expect(field(c, 'max_retries')!.querySelector('[data-message]')).toBeNull();
    expect(onanswer).toHaveBeenCalledWith({ values: { target: 'main' } });
  });

  it('a required yes/no left unchecked answers no', async () => {
    const onanswer = vi.fn(async () => null);
    const decision = F.formDecision({
      response: { schema: { type: 'object', properties: { confirm: { type: 'boolean' } }, required: ['confirm'] } },
    });
    const c = mount(decision, onanswer);
    expect(field(c, 'confirm')!.querySelector<HTMLInputElement>('input[type="checkbox"]')!.checked).toBe(false);
    answerButton(c).click();
    await settle();
    expect(field(c, 'confirm')!.querySelector('[data-message]')).toBeNull();
    expect(onanswer).toHaveBeenCalledWith({ values: { confirm: false } });
  });

  it('two cards with the same property names keep their ids apart; each label names its own control', () => {
    const c1 = mount(F.formDecision({ id: 'ask-1' }));
    const c2 = mount(F.formDecision({ id: 'ask-2' }));
    const ids = [...document.querySelectorAll('[data-decision-form] [id]')].map((e) => e.id);
    expect(new Set(ids).size).toBe(ids.length);
    for (const c of [c1, c2]) {
      for (const label of c.querySelectorAll<HTMLLabelElement>('[data-decision-form] label[for]')) {
        expect(c.querySelector(`[id="${CSS.escape(label.htmlFor)}"]`)).not.toBeNull();
      }
    }
  });
});
