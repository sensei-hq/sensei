<script lang="ts">
    import { Kanji, Eyebrow } from '$lib/components';
    import type { ActionItem, ActionUrgency } from './action-items.js';

    // The project-scoped "action items" panel: the pending recommendations,
    // already score-ranked, each paired with the observation the screen makes
    // above. Read-only display — accept/reject lives on the Impact screen. Pure
    // template: the wire→view-model mapping is `buildActionItems` (a pure helper
    // the page calls); this component only iterates the result.
    let { items, error = null }: { items: ActionItem[]; error?: string | null } = $props();

    // Urgency → chip utility classes. This lookup IS the rendering rule (the one
    // kind of map the guidelines allow inside a component): high → danger,
    // medium → warning, low → a muted ink-faint chip. 'none' never renders.
    const URGENCY_CHIP: Record<ActionUrgency, string> = {
        high: 'bg-danger-soft text-danger',
        medium: 'bg-warning-soft text-warning',
        low: 'bg-paper-mute text-ink-faint',
        none: '',
    };
</script>

<section data-component="action-items" class="flex flex-col gap-3">
    <div class="flex items-center gap-2">
        <Kanji char="為" size="sm" tone="accent" />
        <Eyebrow>Action items</Eyebrow>
    </div>

    {#if error}
        <p data-action-items-error class="text-sm text-ink-mute">
            Couldn’t load action items — {error}
        </p>
    {:else if items.length}
        <div class="flex flex-col">
            {#each items as item (item.id)}
                <article
                    data-action-item={item.id}
                    class="flex flex-col gap-1 py-3 border-t border-paper-edge"
                >
                    <div class="flex items-baseline justify-between gap-3">
                        <span class="text-sm text-ink font-medium text-pretty">{item.title}</span>
                        {#if item.urgency !== 'none'}
                            <span
                                data-action-urgency={item.urgency}
                                class="shrink-0 text-xs px-2 py-1 rounded-full {URGENCY_CHIP[item.urgency]}"
                            >{item.urgencyLabel}</span>
                        {/if}
                    </div>
                    {#if item.why}
                        <p data-action-why class="text-sm text-ink-soft leading-relaxed text-pretty">
                            {item.why}
                        </p>
                    {/if}
                    {#if item.impact}
                        <p data-action-impact class="text-xs text-ink-faint text-pretty">
                            {item.impact}
                        </p>
                    {/if}
                </article>
            {/each}
        </div>
    {:else}
        <p data-action-items-empty class="text-sm text-ink-faint">No action items yet.</p>
    {/if}
</section>
