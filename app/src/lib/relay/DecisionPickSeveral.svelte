<script lang="ts">
  // A pick-several on a decision card (#389 R2): rokkit's MultiSelect, drawn through the form's
  // `renderers` registry in place of @rokkit/forms InputMultiSelect so the trigger never holds a
  // tag. rokkit 1.16 puts a focusable `role="button"` remove control per picked member inside its
  // trigger `<button>` (axe nested-interactive, WCAG 4.1.2; sensei-hq/rokkit#206); with
  // `maxDisplay={0}` the trigger reads "N selected" and the picked members show beside it.
  import { MultiSelect } from '@rokkit/ui';

  type Option = string | Record<string, unknown>;

  let {
    value = $bindable(),
    fields,
    options = [],
    placeholder,
    disabled = false,
    size,
    id,
    onchange,
  }: {
    value?: unknown[];
    fields?: Record<string, string>;
    options?: Option[];
    placeholder?: string;
    disabled?: boolean;
    size?: 'sm' | 'md' | 'lg';
    id?: string;
    onchange?: (values: unknown[], items: unknown[]) => void;
  } & Record<string, unknown> = $props();

  const labelOf = (o: Option): string =>
    typeof o === 'string' ? o : String(o[fields?.label ?? 'label'] ?? o[fields?.value ?? 'value'] ?? '');
  const valueOf = (o: Option): unknown => (typeof o === 'string' ? o : o[fields?.value ?? 'value']);
  // In the lane's order, not the order they were pressed.
  const picked = $derived(options.filter((o) => (value ?? []).includes(valueOf(o))).map(labelOf));
</script>

<MultiSelect items={options} {fields} bind:value {placeholder} {disabled} {size} {id} {onchange} maxDisplay={0} />
{#if picked.length > 0}
  <p data-decision-picked class="text-ink-mute mt-1 text-xs">{picked.join(', ')}</p>
{/if}
