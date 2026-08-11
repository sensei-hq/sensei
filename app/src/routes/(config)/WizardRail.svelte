<script lang="ts">
    import { goto } from '$app/navigation';
    import { Wordmark, StatusDot } from '$lib/components';
    import type { WizardStage } from './stages.js';

    /** Setup wizard rail — sidebar nav with one entry per stage. The
     *  wizard layout passes the live `stages` array (sourced from
     *  wizardState.stages) and the rail handles its own click-to-goto.
     *  Active + done state come from the stage objects themselves. */
    interface Props {
        stages: WizardStage[];
    }
    let { stages }: Props = $props();
</script>

<aside class="flex flex-col px-6 py-6 border-r border-paper-edge bg-paper overflow-hidden">
    <div class="mb-7">
        <Wordmark size="md" />
    </div>

    <div class="text-xs tracking-wide text-ink-soft uppercase mb-3.5">
        Setup
    </div>

    <div data-testid="rail" class="flex flex-col">
        {#each stages as s (s.id)}
            {@const isDone = s.status === 'done'}
            <button
                data-rail-item
                data-stage-id={s.id}
                data-active={s.active}
                class="grid grid-cols-[24px_1fr_14px] px-2 py-1 gap-2.5 items-center rounded-md text-left border border-transparent text-ink-soft cursor-pointer transition-all duration-fast text-sm"
                class:active={s.active}
                class:done={isDone}
                onclick={() => goto(s.path)}
            >
                <span
                    class="rail-kanji kanji text-sm text-center text-ink-soft"
                    class:active={s.active}
                    class:done={isDone}
                    class:text-accent={s.active}
                >{s.icon}</span>
                <div class="overflow-hidden">
                    <div class:leading-tight={s.active}>
                        {s.title}
                    </div>
                    {#if s.active}
                        <div class="font-mono text-xs text-ink-soft mt-0.5">
                            {s.brief}
                        </div>
                    {/if}
                </div>
                <span
                    class="text-xs text-center leading-none text-success opacity-0 transition-opacity duration-fast"
                    class:opacity-100={isDone}
                >✓</span>
            </button>
        {/each}
    </div>

    <div class="mt-auto border-t border-paper-edge pt-3">
        <div class="flex items-center gap-2.5">
            <StatusDot status="ok" />
            <div class="text-xs text-ink-mute leading-snug">
                <div class="tracking-wide uppercase text-xs text-ink-soft">
                    Services
                </div>
                <div class="mt-0.5">all green</div>
            </div>
        </div>
    </div>
</aside>

<style>
    /* Rail item active/done states — color shifts that aren't expressible
       cleanly as Tailwind class:* directives because they conflict on the
       same property. Kept as scoped CSS against the data-* attributes. */
    [data-rail-item].active {
        background: var(--paper-soft);
        border-color: var(--paper-mute);
        color: var(--ink);
    }
    [data-rail-item].done {
        color: var(--ink-mute);
    }
    .rail-kanji.done {
        color: var(--ink-mute);
    }
</style>
