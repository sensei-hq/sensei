<script lang="ts">
    import type { WizardStage } from './stages.js';

    /** The stage's top header — kanji icon + "Step" eyebrow + display title + description.
     *  Rendered by the wizard layout for every stage except `welcome`. */
    interface Props {
        stage: WizardStage;
    }
    let { stage }: Props = $props();
</script>

<div
    data-component="stage-header"
    class="shrink-0 px-16 pt-7 pb-6 border-b border-paper-edge bg-paper-soft relative z-1"
>
    <div class="text-xs text-ink-soft tracking-wide uppercase mb-2">
        <span class="kanji text-accent mr-1.5">{stage.icon}</span>Step
    </div>
    <h1 class="font-display text-4xl font-light tracking-tight m-0 mb-1.5">
        {stage.title}
    </h1>
    <p class="text-sm text-ink-soft m-0">
        {stage.description}
    </p>
</div>
