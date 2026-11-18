<script lang="ts">
  // A form decision's fields (#389): the lane's schema drawn by @rokkit/forms FormRenderer,
  // laid out by the lane's layout or one built from the schema (toDecisionForm). Each field
  // is drawn through the renderer's own `child` snippet so it carries `data-decision-field`.
  // The card owns Answer: it calls `collect()`, which checks the form inline (validateAll),
  // shows each field's message, and hands back the values only when nothing failed.
  import { untrack, type Component } from 'svelte';
  import { FormBuilder, FormRenderer, InputField, validateAll } from '@rokkit/forms';
  import {
    isEmptyValue,
    patternMisses,
    toAnswerValues,
    toDecisionForm,
    withoutPatterns,
    type RelayDecisionResponse,
  } from '@sensei/state';
  import DecisionCheckbox from './DecisionCheckbox.svelte';

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
  // rokkit checks `pattern` with a plain `new RegExp`, which reads some patterns otherwise than
  // the daemon does; it checks the rest, and the form checks patterns as the daemon reads them.
  const checked = withoutPatterns(form.schema);
  const builder = new FormBuilder({ ...form.data }, checked, layout);
  const labels: Record<string, string> = Object.fromEntries(
    form.layout.elements.map((e) => {
      const path = e.scope.replace(/^#\//, '');
      return [path, e.label ?? path];
    }),
  );
  const patternMessage = (path: string) => ({ state: 'error', text: `${labels[path] ?? path} format is invalid` });

  /** The messages for `paths`: rokkit's, else a pattern miss. */
  function messagesFor(paths: string[]): Record<string, unknown> {
    const messages = { ...validateAll(builder.data, checked, layout) } as Record<string, unknown>;
    for (const path of patternMisses(form.schema, builder.data)) {
      if (paths.includes(path) && !messages[path]) messages[path] = patternMessage(path);
    }
    return messages;
  }

  /** Check the form; show every failing field's message. The values when it passes, else null. */
  export function collect(): Record<string, unknown> | null {
    const messages = messagesFor(Object.keys(form.schema.properties));
    builder.validation = messages;
    return Object.keys(messages).length > 0 ? null : toAnswerValues(form.schema, builder.data);
  }

  /** The yes/no drawn native, for assistive tech (DecisionCheckbox); every other control is rokkit's. */
  const renderers: Record<string, Component<Record<string, unknown>>> = {
    checkbox: DecisionCheckbox as Component<Record<string, unknown>>,
  };

  type Element = { scope?: string; type?: string; value?: unknown; props?: Record<string, unknown> };

  function change(path: string, value: unknown): void {
    // A control emptied is unset: rokkit's number input reports a cleared box as NaN, which
    // its own checks read as a bad number rather than as empty.
    builder.updateField(path, isEmptyValue(value) ? undefined : value);
    // A field already showing a message re-checks as it is fixed.
    if ((builder.validation as Record<string, unknown>)[path]) {
      builder.setFieldValidation(path, (messagesFor([path])[path] as never) ?? null);
    }
  }
</script>

{#snippet field(element: Element)}
  {@const path = (element.scope ?? '').replace(/^#\//, '')}
  <div data-decision-field={path}>
    <InputField
      name={`${idPrefix}-${path}`}
      type={element.type}
      value={element.value}
      {...element.props}
      {disabled}
      {renderers}
      onchange={(value: unknown) => change(path, value)}
    />
  </div>
{/snippet}

<div data-decision-form>
  <FormRenderer {builder} child={field} className="grid grid-cols-1 gap-3 md:grid-cols-2" />
</div>
