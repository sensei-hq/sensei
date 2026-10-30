<script lang="ts">
  // What to act on (M17), as rokkit's List in accordion mode: each takeaway a
  // group — its title, and how many it names — opening onto why it matters and
  // the modules, each opening on select.
  import { List } from '@rokkit/ui';
  import type { ProxyItem } from '@rokkit/states';
  import type { Takeaway } from './takeaways';

  let { items, onopen }: { items: Takeaway[]; onopen: (id: string) => void } = $props();

  const groups = $derived(
    items.map((t, i) => ({
      label: t.title,
      value: `takeaway-${i}`,
      badge: String(t.items.length),
      children: [
        // Why it matters: read, not selected.
        { label: t.detail, value: `takeaway-${i}-why`, snippet: 'detail', disabled: true },
        ...t.items.map((item) => ({ label: item.text, badge: item.value || undefined, value: item.target ?? `takeaway-${i}-${item.text}` })),
      ],
    })),
  );
  const targets = $derived(new Set(items.flatMap((t) => t.items.map((i) => i.target)).filter(Boolean)));
</script>

{#snippet detail(proxy: ProxyItem)}
  <span class="text-sm text-ink-soft whitespace-normal">{proxy.label}</span>
{/snippet}

<section class="flex flex-col gap-2" data-testid="codemap-takeaways">
  <h4 class="m-0 text-xs uppercase tracking-wide text-ink-mute">What to act on</h4>
  {#if items.length}
    <List items={groups} collapsible label="What to act on" snippets={{ detail }}
      onselect={(value) => typeof value === 'string' && targets.has(value) && onopen(value)} />
  {:else}
    <p class="m-0 text-sm text-success">No cycles, no upward dependencies, nothing flagged on this view.</p>
  {/if}
</section>
