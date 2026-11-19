<script lang="ts">
  // A form decision's fields (#389): the lane's schema drawn by @rokkit/forms FormRenderer,
  // laid out by the lane's layout or one built from the schema (toDecisionForm). Each field
  // is drawn through the renderer's own `child` snippet so it carries `data-decision-field`.
  // The card owns Answer: it calls `collect()`, which checks the form inline (validateAll),
  // shows each field's message, and hands back the values only when nothing failed.
  import { untrack } from 'svelte';
  import type { Attachment } from 'svelte/attachments';
  import DecisionPickSeveral from './DecisionPickSeveral.svelte';
  import { FormBuilder, FormRenderer, InputField, validateAll } from '@rokkit/forms';
  import {
    textCap,
    textMisses,
    toAnswerValues,
    toDecisionForm,
    withArrayLayout,
    withoutTextRules,
    type RelayDecisionResponse,
    type TextMiss,
  } from '@sensei/state';

  let {
    response,
    idPrefix,
    disabled = false,
  }: {
    response: RelayDecisionResponse;
    /** The decision's id: rokkit names each control's id and its label's `for` after the
     *  field, so two cards with the same property names would share ids without it. */
    idPrefix: string;
    disabled?: boolean;
  } = $props();

  // The form is drawn once for the decision it belongs to; a decision's response never changes.
  const form = untrack(() => toDecisionForm(response));
  // `override` routes every field through the snippet below; it picks no control (the schema does).
  const layout = { ...form.layout, elements: form.layout.elements.map((element) => ({ ...element, override: true })) };
  // rokkit reads `pattern` with a plain `new RegExp` and counts `maxLength` (and draws a native
  // `maxlength`) in UTF-16 units, where the daemon reads Unicode and counts characters; rokkit
  // checks the rest, and the form checks those two as the daemon does.
  const checked = withoutTextRules(form.schema);
  // A pick-several takes its label and `override` from the schema rokkit draws it from
  // (sensei-hq/rokkit#205: rokkit drops an array element's layout keys).
  const builder = new FormBuilder({ ...form.data }, withArrayLayout(checked, layout), layout);
  const labels: Record<string, string> = Object.fromEntries(
    form.layout.elements.map((e) => {
      const path = e.scope.replace(/^#\//, '');
      return [path, e.label ?? path];
    }),
  );
  function textMessage(path: string, miss: TextMiss) {
    const label = labels[path] ?? path;
    const text =
      miss === 'maxLength'
        ? `${label} must be no more than ${textCap(form.schema.properties[path])} characters`
        : `${label} format is invalid`;
    return { state: 'error', text };
  }

  /** The messages for `paths`: rokkit's, else a text rule missed. */
  function messagesFor(paths: string[]): Record<string, unknown> {
    const messages = { ...validateAll(builder.data, checked, layout) } as Record<string, unknown>;
    for (const [path, miss] of Object.entries(textMisses(form.schema, builder.data))) {
      if (paths.includes(path) && !messages[path]) messages[path] = textMessage(path, miss);
    }
    return messages;
  }

  /** Check the form; show every failing field's message. The values when it passes, else null. */
  export function collect(): Record<string, unknown> | null {
    const messages = messagesFor(Object.keys(form.schema.properties));
    builder.validation = messages;
    return Object.keys(messages).length > 0 ? null : toAnswerValues(form.schema, builder.data);
  }

  // `disabled` goes to InputField only when true: rokkit writes `data-field-disabled={disabled}`, so an
  // enabled field carries `"false"`, and zen-sumi fades any `[data-field-disabled]` field to 40%
  // (axe color-contrast; sensei-hq/rokkit#207).

  // A pick-several draws through DecisionPickSeveral: no control nested in the MultiSelect's trigger.
  const renderers = { multiselect: DecisionPickSeveral as never };

  /** Name each listbox rokkit opens in a field after the field's label: rokkit's Select and
   *  MultiSelect open a `role="listbox"` with no name (axe aria-input-field-name; sensei-hq/rokkit#206). */
  function nameListboxes(label: string): Attachment<HTMLElement> {
    return (node) => {
      const name = () => {
        for (const box of node.querySelectorAll<HTMLElement>('[role="listbox"]')) {
          if (!box.hasAttribute('aria-label') && !box.hasAttribute('aria-labelledby')) box.setAttribute('aria-label', label);
        }
      };
      name();
      const observer = new MutationObserver(name);
      observer.observe(node, { childList: true, subtree: true });
      return () => observer.disconnect();
    };
  }

  type Element = { scope?: string; type?: string; value?: unknown; props?: Record<string, unknown> };

  function change(path: string, value: unknown): void {
    builder.updateField(path, value);
    // A field already showing a message re-checks as it is fixed.
    if ((builder.validation as Record<string, unknown>)[path]) {
      builder.setFieldValidation(path, (messagesFor([path])[path] as never) ?? null);
    }
  }
</script>

{#snippet field(element: Element)}
  {@const path = (element.scope ?? '').replace(/^#\//, '')}
  <div data-decision-field={path} {@attach nameListboxes(labels[path] ?? path)}>
    <InputField
      name={`${idPrefix}-${path}`}
      type={element.type}
      value={element.value}
      {...element.props}
      disabled={disabled || undefined}
      {renderers}
      onchange={(value: unknown) => change(path, value)}
    />
  </div>
{/snippet}

<div data-decision-form>
  <FormRenderer {builder} child={field} className="grid grid-cols-1 gap-3 md:grid-cols-2" />
</div>
