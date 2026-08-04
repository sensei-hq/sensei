<script lang="ts">
    import { Button } from '@rokkit/ui';
    import type { WizardStage } from './stages.js';

    /** Bottom nav for the setup wizard — step counter + progress bar + back/next buttons.
     *  All navigation logic stays in the layout; this is a pure template that calls
     *  callbacks on click. */
    interface Props {
        stage: WizardStage;
        currentIdx: number;
        total: number;
        canAdvance: boolean;
        committing: boolean;
        isFirst: boolean;
        isLast: boolean;
        onBack: () => void;
        onNext: () => void;
    }
    let {
        stage,
        currentIdx,
        total,
        canAdvance,
        committing,
        isFirst,
        isLast,
        onBack,
        onNext,
    }: Props = $props();

    // Next-button label is a status-to-label lookup — acceptable in the
    // component per the guideline's "the lookup IS the rendering rule".
    const nextLabel = $derived.by(() => {
        if (committing) return stage?.id === 'assistants' ? 'Configuring…' : 'Saving…';
        if (isLast) return 'Enter observatory →';
        if (stage?.id === 'assistants') return 'Configure & Continue →';
        return 'Continue →';
    });
</script>

<div
    data-component="stage-nav"
    class="flex items-center gap-5 px-16 py-3.5 border-t border-paper-mute bg-paper-soft shrink-0"
>
    <div class="flex items-baseline gap-3">
        <span class="text-xs tracking-wide text-ink-soft uppercase">
            {String(currentIdx + 1).padStart(2, '0')}
            <span class="text-ink-soft">/ {total}</span>
        </span>
        <span class="text-sm text-ink-mute">{stage.title}</span>
    </div>

    <div class="flex-1 flex gap-1 items-center">
        {#each Array(total) as _, i}
            <span
                class="flex-1 h-0.5 rounded-sm bg-paper-mute transition-colors duration"
                class:bg-ink={i <= currentIdx}
            ></span>
        {/each}
    </div>

    <div class="flex gap-2 items-center">
        <Button variant="default" style="ghost" size="sm" onclick={onBack} disabled={isFirst}>
            ← Back
        </Button>
        <Button
            variant="primary"
            size="sm"
            data-action="next"
            onclick={onNext}
            disabled={!canAdvance || committing}
        >
            {nextLabel}
        </Button>
    </div>
</div>
