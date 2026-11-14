<script lang="ts">
    import { Button } from '@rokkit/ui';
    import { Kanji, Eyebrow } from '$lib/components';
    import type { ActionItem, ActionUrgency } from './action-items.js';
    import type { ItemActions } from './action-items-state.svelte.js';

    // The workspace-scoped "action items" panel: the pending recommendations
    // about this metric, already score-ranked, each with the one action it
    // implies (propose a rule, review a skill or agent, show a file, open a lens,
    // or snooze). Pure template: the items come from `buildActionItems`, and every
    // action, its confirm and its outcome are `actions` (ActionItemsState) — this
    // component only renders them and hands clicks back.
    let {
        items,
        error = null,
        actions,
        scope,
    }: {
        items: ActionItem[];
        error?: string | null;
        actions: ItemActions;
        /** The panel's owner (the metric key): what it does, and says it did, belongs to it. */
        scope: string;
    } = $props();
    const done = $derived(actions.doneIn(scope));

    // Urgency → chip utility classes. This lookup IS the rendering rule (the one
    // kind of map the guidelines allow inside a component): high → danger,
    // medium → warning, low → a muted ink-faint chip. 'none' never renders.
    const URGENCY_CHIP: Record<ActionUrgency, string> = {
        high: 'bg-danger-soft text-on-danger-soft',
        medium: 'bg-warning-soft text-on-warning-soft',
        low: 'bg-paper-mute text-ink-faint',
        none: '',
    };
</script>

<section data-component="action-items" class="flex flex-col gap-3">
    <div class="flex items-center gap-2">
        <Kanji char="為" size="sm" tone="accent" />
        <Eyebrow>Action items</Eyebrow>
    </div>

    {#if done}
        <p data-action-done class="text-sm text-ink-mute">{done}</p>
    {/if}

    {#if error}
        <p data-action-items-error class="text-sm text-ink-soft">
            Couldn’t load action items — {error}
        </p>
    {:else if items.length}
        <div class="flex flex-col">
            {#each items as item (item.id)}
                {@const flow = actions.flow(item.id)}
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
                        <p data-action-why class="text-sm text-ink-mute leading-relaxed text-pretty">
                            {item.why}
                        </p>
                    {/if}
                    {#if item.impact}
                        <p data-action-impact class="text-xs text-ink-faint text-pretty">
                            {item.impact}
                        </p>
                    {/if}

                    {#if flow.status === 'confirm'}
                        {@const c = flow.confirm}
                        <div
                            data-action-confirm={c.kind}
                            class="flex flex-col gap-2 mt-2 p-3 rounded-lg bg-paper-soft border border-paper-edge"
                        >
                            <span class="text-sm text-ink font-medium">{c.heading}</span>
                            {#if c.path}
                                <span data-action-confirm-path class="mono text-xs text-ink-soft">
                                    sensei will write {c.path}
                                </span>
                                <pre
                                    class="mono text-xs text-ink-mute whitespace-pre-wrap max-h-60 overflow-auto m-0">{c.body}</pre>
                            {:else}
                                <p class="text-sm text-ink-mute leading-relaxed text-pretty m-0">{c.body}</p>
                            {/if}
                            {#if c.scope}
                                <span data-action-confirm-scope class="text-xs text-ink-faint">
                                    Added as a rule at {c.scope}
                                </span>
                            {/if}
                            <div class="flex items-center gap-2">
                                <Button
                                    variant="primary"
                                    size="sm"
                                    data-action-confirm-yes
                                    onclick={() => actions.confirm(item, scope)}
                                >{c.confirmLabel}</Button>
                                <Button
                                    variant="secondary"
                                    style="outline"
                                    size="sm"
                                    data-action-confirm-no
                                    onclick={() => actions.cancel(item)}
                                >Cancel</Button>
                            </div>
                        </div>
                    {:else}
                        <div class="flex items-center gap-2 mt-1">
                            {#if item.action.kind === 'open-lens'}
                                <Button
                                    variant="secondary"
                                    style="outline"
                                    size="sm"
                                    href={item.action.href}
                                    data-action-do="open-lens"
                                >{item.action.label}</Button>
                            {:else}
                                <Button
                                    variant="secondary"
                                    style="outline"
                                    size="sm"
                                    data-action-do={item.action.kind}
                                    disabled={flow.status === 'working'}
                                    aria-busy={flow.status === 'working'}
                                    onclick={() => actions.act(item, scope)}
                                >{item.action.label}</Button>
                            {/if}
                        </div>
                    {/if}

                    {#if flow.status === 'error'}
                        <p data-action-error class="text-sm text-ink-soft">{flow.message}</p>
                    {/if}
                </article>
            {/each}
        </div>
    {:else}
        <p data-action-items-empty class="text-sm text-ink-faint">No action items yet.</p>
    {/if}
</section>
