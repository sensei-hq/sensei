<script lang="ts">
  // One toolbar item (COMPONENTS.md §2): a view tab or a toggle, 36×36, with an
  // optional count badge. It draws what it is given and hands the click back;
  // the screen owns which item is pressed and what pressing it does. The icon
  // is a class, so the caller keeps the app's icon style.
  import {
    toolbarBadgeClass,
    toolbarIconClass,
    toolbarItemClass,
    toolbarLabel,
  } from './toolbar-item.js';

  let {
    id,
    label,
    icon,
    pressed = false,
    count = 0,
    kind = 'view',
    onclick = null,
  }: {
    id: string;
    label: string;
    /** The icon's class, e.g. an `i-solar-*` utility. */
    icon: string;
    pressed?: boolean;
    /** Shown as a badge when > 0, and folded into the accessible name. */
    count?: number;
    kind?: 'view' | 'toggle';
    onclick?: ((event: MouseEvent) => void) | null;
  } = $props();

  const name = $derived(toolbarLabel(label, count));
</script>

<button
  type="button"
  data-toolbar-item={id}
  data-kind={kind}
  data-pressed={String(pressed)}
  class={toolbarItemClass(pressed)}
  title={name}
  aria-label={name}
  role={kind === 'view' ? 'tab' : undefined}
  aria-selected={kind === 'view' ? pressed : undefined}
  aria-pressed={kind === 'toggle' ? pressed : undefined}
  {onclick}
>
  <span class={[icon, toolbarIconClass(kind, pressed)]} aria-hidden="true"></span>
  {#if count > 0}
    <span data-toolbar-count class={toolbarBadgeClass()}>{count}</span>
  {/if}
</button>
