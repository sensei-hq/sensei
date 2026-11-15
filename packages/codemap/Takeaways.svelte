<script lang="ts">
  // What to act on (M17), for the lens in view. Text, not List buttons (user,
  // 2026-11-02): a section per takeaway, open by default and collapsible, its
  // modules as selectable text with an open action beside each, and a Copy
  // action that puts the lens's takeaways on the clipboard as markdown. Readable
  // (user, 2026-11-03): each module is its name with its folder dimmed, a cycle
  // lists its members, the number sits on its own line, the action is a button.
  import { Button } from '@rokkit/ui';
  import { takeawaysMarkdown, type Takeaway } from './takeaways';

  let { items, onfocus, label = 'Code map' }: {
    items: Takeaway[];
    onfocus: (id: string) => void;
    /** The diagram's name, heading the copied markdown. */
    label?: string;
  } = $props();

  let copied = $state(false);
  async function copy() {
    await navigator.clipboard.writeText(takeawaysMarkdown(label, items));
    copied = true;
    setTimeout(() => (copied = false), 2000);
  }

  /** A module id as its folder and its name — the name is what a reader scans for. */
  const split = (id: string) => {
    const at = id.lastIndexOf('/');
    return at < 0 ? { folder: '', name: id } : { folder: id.slice(0, at + 1), name: id.slice(at + 1) };
  };

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
        <ul class="m-0 p-0 list-none flex flex-col gap-3">
          {#each t.items as item (item.text)}
            <li class="flex items-start gap-3 text-sm" data-takeaway-item>
              <div class="flex-1 min-w-0 flex flex-col gap-0.5" data-takeaway-text>
                {#each item.paths ?? [item.text] as path, i (i)}
                  {@const p = split(path)}
                  <span class="flex items-baseline gap-1.5 min-w-0" data-path title={path}>
                    {#if i > 0 && item.join}<span class="shrink-0 text-ink-mute" aria-hidden="true">{item.join}</span>{/if}
                    <span class="min-w-0 flex"><span class="min-w-0 truncate text-ink-mute" data-path-folder>{p.folder}</span><span class="shrink-0 font-medium text-ink" data-path-name>{p.name}</span></span>
                  </span>
                {/each}
                {#if item.value}<span class="text-xs text-ink-mute tabular-nums" data-takeaway-value>{item.value}</span>{/if}
              </div>
              {#if item.target}
                <Button class="shrink-0" size="sm" style="outline" icon="i-solar-target-bold-duotone" label="Focus"
                  aria-label={`Focus ${item.target}`} title={`Focus ${item.target}`} onclick={() => onfocus(item.target!)} />
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
