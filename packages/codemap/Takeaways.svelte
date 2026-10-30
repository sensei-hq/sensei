<script lang="ts">
  // What to act on (M17): each takeaway a title, why it matters, and the modules
  // it names — each opens on a click.
  import type { Takeaway } from './takeaways';

  let { items, onopen }: { items: Takeaway[]; onopen: (id: string) => void } = $props();
  const TONE = { ink: 'text-ink', danger: 'text-danger', warning: 'text-warning', success: 'text-success', faint: 'text-ink-faint' };
</script>

<section class="flex flex-col gap-4" data-testid="codemap-takeaways">
  <h4 class="m-0 text-xs uppercase tracking-wide text-ink-mute">What to act on</h4>
  {#each items as t, i (t.title)}
    <article class="flex flex-col gap-1" data-testid={`takeaway-${i}`}>
      <h5 class={`m-0 text-sm font-medium ${TONE[t.tone]}`}>{i + 1}. {t.title}</h5>
      <p class="m-0 text-sm text-ink-soft">{t.detail}</p>
      <ul class="m-0 p-0 list-none flex flex-col gap-0.5">
        {#each t.items as item (item.text)}
          <li class="flex items-baseline justify-between gap-2">
            {#if item.target}
              <button class="p-0 bg-transparent border-none cursor-pointer text-left font-mono text-xs text-ink hover:underline break-all"
                onclick={() => onopen(item.target!)} data-testid={`takeaway-item-${item.target}`}>{item.text}</button>
            {:else}
              <span class="font-mono text-xs text-ink break-all">{item.text}</span>
            {/if}
            {#if item.value}<span class="font-mono text-xs text-ink-mute shrink-0">{item.value}</span>{/if}
          </li>
        {/each}
      </ul>
    </article>
  {:else}
    <p class="m-0 text-sm text-success">No cycles, no upward dependencies, nothing flagged on this view.</p>
  {/each}
</section>
