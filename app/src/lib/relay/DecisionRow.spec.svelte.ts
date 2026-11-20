// @vitest-environment jsdom
// The decision card (#389; COMPONENTS §3b DecisionCard): an options decision is buttons and
// Answer, as before; a form decision is the lane's schema drawn by @rokkit/forms, checked
// inline before anything is sent, sent once, and a refusal shown in place with the values kept.
import { describe, it, expect, afterEach, vi } from 'vitest';
import { flushSync, tick } from 'svelte';
import axe from 'axe-core';
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

/** The serious and critical axe violations in the card (WCAG A + AA; contrast is a browser check). */
async function seriousA11y(c: HTMLElement): Promise<string[]> {
  const result = await axe.run(c, {
    runOnly: { type: 'tag', values: ['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa'] },
    rules: { 'color-contrast': { enabled: false } },
  });
  return result.violations
    .filter((v) => v.impact === 'serious' || v.impact === 'critical')
    .map((v) => `${v.id}: ${v.nodes.map((n) => n.target.join(' ')).join(', ')}`);
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

  // rokkit 1.16.1 draws a Select's placeholder at full opacity (sensei-hq/rokkit#208 fixed):
  // the trigger shows it, and the field's label still names the control. Contrast is a browser
  // check, so this pins the cause: placeholder text is present, not blanked.
  it('an empty Select shows its placeholder; its label names it', () => {
    const c = mount(F.formDecision());
    const trigger = field(c, 'target')!.querySelector<HTMLElement>('[data-select-trigger]')!;
    expect(trigger.querySelector('[data-select-placeholder]')?.textContent?.trim() ?? '').not.toBe('');
    expect(c.querySelector(`label[for="${trigger.id}"]`)?.textContent).toContain('Target branch');
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

  // The daemon reads a pattern with Rust `regex` (Unicode); a plain `new RegExp` reads these
  // differently and would refuse a value the daemon takes, so nothing could be posted.
  it.each([
    ['^\\p{Lu}', 'Élan'],
    ['^.$', '😀'],
    ['^[^/]{2}$', 'a😀'],
  ])('a value the daemon would take under pattern %s is sent', async (pattern, text) => {
    const onanswer = vi.fn(async () => null);
    const decision = F.formDecision({
      response: { schema: { type: 'object', properties: { name: { type: 'string', pattern } }, required: ['name'] } },
    });
    const c = mount(decision, onanswer);
    const input = field(c, 'name')!.querySelector<HTMLInputElement>('input')!;
    input.value = text;
    input.dispatchEvent(new Event('input', { bubbles: true }));
    input.dispatchEvent(new Event('change', { bubbles: true }));
    await settle();
    answerButton(c).click();
    await settle();
    expect(field(c, 'name')!.querySelector('[data-message]')).toBeNull();
    expect(onanswer).toHaveBeenCalledWith({ values: { name: text } });
  });

  // The daemon counts maxLength in characters; the browser's `.length` and a native `maxlength`
  // count UTF-16 units, so an emoji counts twice and a value the daemon takes would be refused.
  it('a value within maxLength in characters is sent, emoji included; no native maxlength cuts it short', async () => {
    const onanswer = vi.fn(async () => null);
    const decision = F.formDecision({
      response: { schema: { type: 'object', properties: { tag: { type: 'string', maxLength: 2 } }, required: ['tag'] } },
    });
    const c = mount(decision, onanswer);
    const input = field(c, 'tag')!.querySelector<HTMLInputElement>('input')!;
    expect(input.getAttribute('maxlength')).toBeNull();
    input.value = '😀😀';
    input.dispatchEvent(new Event('input', { bubbles: true }));
    input.dispatchEvent(new Event('change', { bubbles: true }));
    await settle();
    answerButton(c).click();
    await settle();
    expect(field(c, 'tag')!.querySelector('[data-message]')).toBeNull();
    expect(onanswer).toHaveBeenCalledWith({ values: { tag: '😀😀' } });
  });

  it('a value over maxLength in characters is refused inline', async () => {
    const onanswer = vi.fn(async () => null);
    const decision = F.formDecision({
      response: { schema: { type: 'object', properties: { tag: { type: 'string', maxLength: 2 } }, required: ['tag'] } },
    });
    const c = mount(decision, onanswer);
    const input = field(c, 'tag')!.querySelector<HTMLInputElement>('input')!;
    input.value = '😀😀😀';
    input.dispatchEvent(new Event('input', { bubbles: true }));
    input.dispatchEvent(new Event('change', { bubbles: true }));
    await settle();
    answerButton(c).click();
    await settle();
    expect(onanswer).not.toHaveBeenCalled();
    expect(field(c, 'tag')!.querySelector('[data-message]')?.textContent).toContain('no more than 2 characters');
  });

  it('a pattern miss shown inline clears once the value is fixed', async () => {
    const onanswer = vi.fn(async () => null);
    const decision = F.formDecision({
      response: { schema: { type: 'object', properties: { path: { type: 'string', pattern: '^/' } }, required: ['path'] } },
    });
    const c = mount(decision, onanswer);
    const input = field(c, 'path')!.querySelector<HTMLInputElement>('input')!;
    const type = (value: string) => {
      input.value = value;
      input.dispatchEvent(new Event('input', { bubbles: true }));
      input.dispatchEvent(new Event('change', { bubbles: true }));
    };
    type('rel');
    await settle();
    answerButton(c).click();
    await settle();
    expect(field(c, 'path')!.querySelector('[data-message]')?.textContent).toContain('format is invalid');
    type('/abs');
    await settle();
    expect(field(c, 'path')!.querySelector('[data-message]')).toBeNull();
    answerButton(c).click();
    await settle();
    expect(onanswer).toHaveBeenCalledWith({ values: { path: '/abs' } });
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

  // Every state the card reaches passes axe, not only the empty form: a control's filled state
  // can draw new elements (a Select's value, a message under a field).
  it('passes axe empty, with its messages showing, and filled', async () => {
    const c = mount(F.formDecision());
    expect(await seriousA11y(c)).toEqual([]);
    answerButton(c).click();
    await settle();
    expect(field(c, 'target')!.querySelector('[data-message]')).not.toBeNull();
    expect(await seriousA11y(c)).toEqual([]);
    // Open, rokkit names the Select's listbox after its trigger (sensei-hq/rokkit#206 fixed in
    // 1.16.1); the card adds no name of its own. axe aria-input-field-name pins it end to end.
    field(c, 'target')!.querySelector<HTMLButtonElement>('[data-select-trigger]')!.click();
    await settle();
    expect(field(c, 'target')!.querySelector('[role="listbox"]')).not.toBeNull();
    expect(await seriousA11y(c)).toEqual([]);
    field(c, 'target')!.querySelector<HTMLButtonElement>('[data-select-trigger]')!.click();
    await settle();
    await pick(c, 'target', 'main');
    expect(await seriousA11y(c)).toEqual([]);
  });

  it('two cards with the same property names keep their ids apart; each label names its own control', () => {
    const c1 = mount(F.formDecision({ id: 'ask-1' }));
    const c2 = mount(F.formDecision({ id: 'ask-2' }));
    const ids = [...document.querySelectorAll('[data-decision-form] [id]')].map((e) => e.id);
    expect(new Set(ids).size).toBe(ids.length);
    // Every label names the control in its own card: the select's trigger, the number and the yes/no.
    for (const c of [c1, c2]) {
      const named = [...c.querySelectorAll<HTMLLabelElement>('[data-decision-form] label[for]')]
        .map((label) => document.getElementById(label.htmlFor))
        .filter((control) => control !== null);
      expect(named.length).toBe(3);
      expect(named.every((control) => c.contains(control))).toBe(true);
    }
  });
});

// R2 (@rokkit/forms 1.16.1, rokkit#197): an array of enum members draws rokkit's MultiSelect and an
// `x-multiline` string a textarea; both are answered and checked as the daemon checks them.
describe('DecisionRow · pick-several and long text', () => {
  const several = () =>
    F.formDecision({
      id: 'ask-several',
      response: {
        schema: {
          type: 'object',
          properties: {
            areas: { type: 'array', items: { type: 'string', enum: ['api', 'app', 'db'] }, uniqueItems: true, title: 'Areas' },
            notes: { type: 'string', 'x-multiline': true, title: 'Notes' },
          },
          required: ['areas'],
        },
      },
    });

  /** Press members in the MultiSelect: open it, press each option by its text. */
  async function pickSeveral(c: HTMLElement, name: string, labels: string[]) {
    field(c, name)!.querySelector<HTMLButtonElement>('[data-select-trigger]')!.click();
    await settle();
    for (const label of labels) {
      const option = [...field(c, name)!.querySelectorAll<HTMLElement>('[data-select-option]')].find(
        (o) => o.textContent?.trim() === label,
      );
      option!.click();
      await settle();
    }
  }

  /** Type into the textarea as the browser reports it. */
  function typeNotes(c: HTMLElement, value: string) {
    const area = field(c, 'notes')!.querySelector<HTMLTextAreaElement>('textarea')!;
    area.value = value;
    area.dispatchEvent(new Event('input', { bubbles: true }));
    area.dispatchEvent(new Event('change', { bubbles: true }));
  }

  it('draws a MultiSelect for the pick-several and a textarea for the long text', () => {
    const c = mount(several());
    expect(field(c, 'areas')!.querySelector('[data-multiselect]')).not.toBeNull();
    expect(field(c, 'areas')!.querySelector('[data-field-type="multiselect"]')).not.toBeNull();
    const area = field(c, 'notes')!.querySelector<HTMLTextAreaElement>('textarea');
    expect(area).not.toBeNull();
    // No native maxlength: the form counts characters as the daemon does.
    expect(area!.getAttribute('maxlength')).toBeNull();
    expect(field(c, 'areas')!.textContent).toContain('Areas');
  });

  it('Answer with no member picked for a required pick-several shows the field message and sends nothing', async () => {
    const onanswer = vi.fn(async () => null);
    const c = mount(several(), onanswer);
    answerButton(c).click();
    await settle();
    expect(onanswer).not.toHaveBeenCalled();
    expect(field(c, 'areas')!.querySelector('[data-message]')?.textContent).toContain('is required');
  });

  it('sends the picked members and the long text, line breaks kept', async () => {
    const onanswer = vi.fn(async () => null);
    const c = mount(several(), onanswer);
    await pickSeveral(c, 'areas', ['db', 'api']);
    typeNotes(c, 'first line\nsecond line');
    await settle();
    answerButton(c).click();
    await settle();
    expect(onanswer).toHaveBeenCalledTimes(1);
    const sent = (onanswer.mock.calls[0] as unknown as [{ values: { areas: string[]; notes: string } }])[0].values;
    expect([...sent.areas].sort()).toEqual(['api', 'db']);
    expect(sent.notes).toBe('first line\nsecond line');
  });

  it("long text over the daemon's 4000-character cap is refused inline; an empty optional one is left out", async () => {
    const onanswer = vi.fn(async () => null);
    const c = mount(several(), onanswer);
    await pickSeveral(c, 'areas', ['app']);
    typeNotes(c, 'x'.repeat(4001));
    await settle();
    answerButton(c).click();
    await settle();
    expect(onanswer).not.toHaveBeenCalled();
    expect(field(c, 'notes')!.querySelector('[data-message]')?.textContent).toContain('no more than 4000 characters');
    typeNotes(c, '');
    await settle();
    answerButton(c).click();
    await settle();
    expect(onanswer).toHaveBeenCalledWith({ values: { areas: ['app'] } });
  });

  // rokkit 1.16.1 draws each picked member as a label, not a control (sensei-hq/rokkit#206
  // fixed): no focusable remove sits inside the trigger <button>. axe nested-interactive pins it.
  it('passes axe with members picked: no control nested inside the trigger', async () => {
    const c = mount(several());
    expect(await seriousA11y(c)).toEqual([]);
    await pickSeveral(c, 'areas', ['db', 'api']);
    typeNotes(c, 'first line\nsecond line');
    await settle();
    expect(await seriousA11y(c)).toEqual([]);
    const trigger = field(c, 'areas')!.querySelector('[data-select-trigger]')!;
    expect(trigger.querySelectorAll('[role="button"], [tabindex], button, a[href], input').length).toBe(0);
  });

  // The card draws rokkit's MultiSelect directly: the picked members show as tags in the
  // trigger, with no beside-trigger list of its own.
  it('shows the picked members as tags in the trigger', async () => {
    const c = mount(several());
    await pickSeveral(c, 'areas', ['db', 'api']);
    const tags = [...field(c, 'areas')!.querySelectorAll('[data-select-trigger] [data-select-tag-text]')].map(
      (t) => t.textContent,
    );
    expect(tags).toEqual(expect.arrayContaining(['api', 'db']));
    expect(field(c, 'areas')!.querySelector('[data-decision-picked]')).toBeNull();
  });

  // rokkit 1.16.1 draws the placeholder at full opacity (sensei-hq/rokkit#208 fixed): the empty
  // pick-several shows it. Contrast is a browser check, so this pins the cause.
  it('an empty pick-several shows its placeholder', () => {
    const c = mount(several());
    const trigger = field(c, 'areas')!.querySelector<HTMLElement>('[data-select-trigger]')!;
    expect(trigger.querySelector('[data-select-placeholder]')?.textContent?.trim() ?? '').not.toBe('');
  });

  // rokkit 1.16.1 omits false-valued data-field-* attributes itself (sensei-hq/rokkit#207 fixed),
  // so zen-sumi's `[data-field-disabled]` fade only ever touches a field the card disabled: the
  // card passes `disabled` straight through. Contrast is a browser check, so this pins the cause:
  // an enabled card's fields carry no disabled mark; a sending one's do.
  it('an enabled card marks no field disabled; while it sends, every field is', async () => {
    let release: (v: string | null) => void = () => {};
    const onanswer = vi.fn(() => new Promise<string | null>((r) => (release = r)));
    const c = mount(several(), onanswer);
    await pickSeveral(c, 'areas', ['db', 'api']);
    typeNotes(c, 'first line');
    await settle();
    expect(c.querySelectorAll('[data-field-root]').length).toBe(2);
    expect(c.querySelectorAll('[data-field-disabled]').length).toBe(0);
    answerButton(c).click();
    await settle();
    expect(c.querySelectorAll('[data-field-root][data-field-disabled="true"]').length).toBe(2);
    release(null);
    await settle();
  });
});
