<script lang="ts">
    import { Eyebrow, Rail, Wordmark } from '$lib/components';
    import { buildNavItems, resolveActiveHref } from './observatory-nav';
    import type { GlyphMode } from '$lib/glyphs';

    interface Props {
        /** Daemon port, shown in the footer. */
        port: number;
        /** Current route, passed from the layout (keeps this component pure). */
        pathname: string;
        /** Cached workspace count for the Workspaces badge; omit until loaded. */
        workspaceCount?: number;
        /** Kanji or icons (#133); the layout reads it from the preference. */
        glyphs?: GlyphMode;
        /** What needs you, in one sentence (PersonaList.attentionLine), or null. */
        attention?: string | null;
        /** Where that is handled — Settings · Dōjō. */
        onAttention?: () => void;
    }
    let { port, pathname, workspaceCount, glyphs = 'kanji', attention = null, onAttention }: Props = $props();

    const items = $derived(buildNavItems({ workspaceCount }));
    const activeHref = $derived(resolveActiveHref(pathname));
</script>

<Rail component="observatory-sidebar" label="Observatory" {items} value={activeHref} {glyphs}>
    {#snippet header()}
        <div class="px-1">
            <Wordmark />
        </div>
        <div class="px-2">
            <Eyebrow>Observatory</Eyebrow>
        </div>
    {/snippet}
    {#snippet footer()}
        {#if attention}
            <button
                type="button"
                data-attention
                class="flex items-start gap-2 w-full text-left text-xs leading-snug text-warning bg-transparent border-0 p-1 rounded-sm cursor-pointer hover:bg-paper-mute"
                onclick={() => onAttention?.()}
            >
                <span class="mt-1 w-1.5 h-1.5 shrink-0 rounded-full bg-warning" aria-hidden="true"></span>
                <span>{attention}</span>
            </button>
        {/if}
        <div class="border-t border-paper-edge pt-2 text-xs leading-relaxed">
            <span class="font-mono text-ink-soft">daemon · running</span><br />
            <span class="font-mono text-ink-faint">port {port}</span>
        </div>
    {/snippet}
</Rail>
