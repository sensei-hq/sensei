<script lang="ts">
    import type { ChipTone } from './memories-state.svelte.js';

    let {
        tone,
        label,
    }: {
        tone: ChipTone;
        label: string;
    } = $props();

    // Semantic tone → utility classes. Ink tokens have no `-soft`, so the
    // neutral pill sits on paper-mute; the status tones use their soft fill.
    const TONES: Record<ChipTone, string> = {
        success: 'bg-success-soft text-success',
        neutral: 'bg-paper-mute text-ink-mute',
        accent: 'bg-accent-soft text-accent',
    };
</script>

<span
    data-component="memory-chip"
    data-tone={tone}
    class="inline-flex items-center rounded-full px-2 py-0.5 text-xs whitespace-nowrap {TONES[tone]}"
>
    {label}
</span>
