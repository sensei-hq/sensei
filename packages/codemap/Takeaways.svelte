<script lang="ts">
  // What to act on (M17), for the lens in view. Text, not List buttons (user,
  // 2026-11-02): a section per takeaway, open by default and collapsible, its
  // modules as selectable text with an open action beside each, and a Copy
  // action that puts the lens's takeaways on the clipboard as markdown.
  import { takeawaysMarkdown, type Takeaway } from './takeaways';

  let { items, onopen, label = 'Code map' }: {
    items: Takeaway[];
    onopen: (id: string) => void;
    /** The diagram's name, heading the copied markdown. */
    label?: string;
  } = $props();

  let copied = $state(false);
  async function copy() {
    await navigator.clipboard.writeText(takeawaysMarkdown(label, items));
    copied = true;
    setTimeout(() => (copied = false), 2000);
  }

  const TONE_TEXT = { danger: 'text-danger', warning: 'text-warning', success: 'text-success', ink: 'text-ink', faint: 'text-ink-faint' } as const;
</script>

<section class="flex flex-col gap-3 select-text" data-testid="codemap-takeaways">
  <div class="flex items-center justify-between gap-2">
    <h4 class="m-0 text-xs uppercase tracking-wide text-ink-mute">What to act on</h4>
    {#if items.length}
      <button class="px-2 py-0.5 text-xs rounded border border-paper-edge bg-transparent text-ink-mute cursor-pointer hover:text-ink"
        onclick={copy} data-testid="codemap-takeaways-copy">{copied ? 'Copied' : 'Copy'}</button>
    {/if}
  </div>
  {#if items.length}
    {#each items as t (t.title)}
      <details open class="flex flex-col gap-2 border-t border-paper-edge pt-3" data-takeaway>
        <summary class="cursor-pointer text-sm font-medium {TONE_TEXT[t.tone]}">{t.title}</summary>
        <p class="mt-2 mb-2 text-sm text-ink-soft">{t.detail}</p>
        <ul class="m-0 p-0 list-none flex flex-col gap-1">
          {#each t.items as item (item.text)}
            <li class="flex items-baseline gap-2 text-xs" data-takeaway-item>
              <span class="font-mono text-ink break-all" data-takeaway-text>{item.text}</span>
              {#if item.value}<span class="text-ink-mute whitespace-nowrap">{item.value}</span>{/if}
              {#if item.target}
                <button class="ml-auto shrink-0 p-0 bg-transparent border-none cursor-pointer text-ink-mute hover:text-ink"
                  aria-label={`Open ${item.target}`} title={`Open ${item.target}`} onclick={() => onopen(item.target!)}>↗</button>
              {/if}
            </li>
          {/each}
        </ul>
      </details>
    {/each}
  {:else}
    <p class="m-0 text-sm text-success">Nothing flagged on this view.</p>
  {/if}
</section>
