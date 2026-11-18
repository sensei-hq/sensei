<script lang="ts">
  // A form decision's fields (#389): the lane's schema drawn by @rokkit/forms FormRenderer,
  // laid out by the lane's layout or one built from the schema (toDecisionForm). Each field
  // is drawn through the renderer's own `child` snippet so it carries `data-decision-field`.
  // The card owns Answer: it calls `collect()`, which checks the form inline (validateAll),
  // shows each field's message, and hands back the values only when nothing failed.
  import { untrack, type Component } from 'svelte';
  import { FormBuilder, FormRenderer, InputField, validateAll } from '@rokkit/forms';
  import { toAnswerValues, toDecisionForm, type RelayDecisionResponse } from '@sensei/state';
  import DecisionCheckbox from './DecisionCheckbox.svelte';

  let { response, disabled = false }: { response: RelayDecisionResponse; disabled?: boolean } = $props();

  // The form is drawn once for the decision it belongs to; a decision's response never changes.
  const form = untrack(() => toDecisionForm(response));
  // `override` routes every field through the snippet below; it picks no control (the schema does).
  const layout = { ...form.layout, elements: form.layout.elements.map((element) => ({ ...element, override: true })) };
  const builder = new FormBuilder({ ...form.data }, form.schema, layout);

  /** Check the form; show every failing field's message. The values when it passes, else null. */
  export function collect(): Record<string, unknown> | null {
    const messages = validateAll(builder.data, form.schema, layout);
    builder.validation = messages;
    return Object.keys(messages).length > 0 ? null : toAnswerValues(form.schema, builder.data);
  }

  /** The yes/no drawn native, for assistive tech (DecisionCheckbox); every other control is rokkit's. */
  const renderers: Record<string, Component<Record<string, unknown>>> = {
    checkbox: DecisionCheckbox as Component<Record<string, unknown>>,
  };

  type Element = { scope?: string; type?: string; value?: unknown; props?: Record<string, unknown> };

  function change(path: string, value: unknown): void {
    builder.updateField(path, value);
    // A field already showing a message re-checks as it is fixed.
    if ((builder.validation as Record<string, unknown>)[path]) builder.validateField(path);
  }
</script>

{#snippet field(element: Element)}
  {@const path = (element.scope ?? '').replace(/^#\//, '')}
  <div data-decision-field={path}>
    <InputField
      name={element.scope}
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
