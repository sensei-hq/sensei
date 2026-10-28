<script lang="ts">
    import Kanji from '$lib/components/Kanji.svelte';
    import type { ToolHealthSource } from "$lib/types.js";
    import { toolSourceView } from "$lib/state/mcp.svelte.js";

    interface Props {
        source: ToolHealthSource;
        /** True while this card's L2 drill is open. */
        active?: boolean;
        /** Card click — drills into the per-tool (L2) view for this source. */
        onopen?: (source: ToolHealthSource) => void;
    }

    const { source, active = false, onopen }: Props = $props();

    // Probed or not, the invoked share, its colour and the glyph are
    // toolSourceView's; an unprobed source draws no share (never a made-up 0%).
    const view = $derived(toolSourceView(source));
</script>

<button
    type="button"
    class="w-full text-left bg-paper-soft border border-paper-edge rounded-lg py-3 px-4 flex flex-col gap-3 cursor-pointer"
    class:opacity-60={!source.connected}
    data-testid={`tool-health-card-${source.source_key}`}
    data-source-type={source.source_type}
    data-probed={view.probed}
    aria-pressed={active}
    onclick={() => onopen?.(source)}
>
    <!-- Header — kanji · name + type/key · connected chip -->
    <div class="flex items-start gap-2">
        <Kanji char={view.kanji} size="inherit" tone="inherit" class="text-xl text-accent leading-none w-6 text-center shrink-0" />
        <div class="flex-1 min-w-0">
            <div class="text-base text-ink truncate">{source.name}</div>
            <div class="flex items-center gap-2 font-mono text-xs text-ink-soft mt-1">
                <span class="uppercase tracking-wide">{source.source_type}</span>
                <span>·</span>
                <span class="truncate">{source.source_key}</span>
            </div>
        </div>
        <span class="flex items-center gap-1 shrink-0">
            <span
                class="rounded-full inline-block"
                class:bg-success={source.connected}
                class:bg-ink-faint={!source.connected}
                style="width: 7px; height: 7px;"
            ></span>
            <span
                class="text-xs uppercase tracking-wide"
                class:text-success={source.connected}
                class:text-ink-soft={!source.connected}
            >{source.connected ? "on" : "off"}</span>
        </span>
    </div>

    {#if view.probed}
        <!-- Share-invoked: headline % + proportional bar + count detail -->
        <div class="flex items-baseline gap-2">
            <span
                class="display text-2xl leading-none {view.pctClass}"
                style='font-feature-settings: "tnum";'
            >{view.pctLabel}</span>
            <span class="text-xs text-ink-soft">
                invoked {source.tools_invoked_14d} of {view.registered} tools
            </span>
        </div>
        <div class="h-2 rounded-sm overflow-hidden bg-paper-mute" data-testid="share-bar">
            <div class="h-full bg-accent" style={`width: ${view.pct ?? 0}%;`}></div>
        </div>
    {:else}
        <!-- Un-probed server — honest usage-only line, no bar. -->
        <div class="text-sm text-ink-mute" data-testid="registered-none">
            invoked {source.tools_invoked_14d} · registered —
        </div>
    {/if}

    <!-- Calls chip -->
    <div
        class="flex gap-3 pt-2 border-t border-paper-edge font-mono text-xs text-ink-soft"
        style='font-feature-settings: "tnum";'
    >
        <span>{source.calls_14d.toLocaleString()} calls · 14d</span>
    </div>
</button>
