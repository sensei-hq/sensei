<script lang="ts">
    import type { ToolHealthSource } from "$lib/types.js";

    interface Props {
        source: ToolHealthSource;
        /** True while this card's L2 drill is open. */
        active?: boolean;
        /** Card click — drills into the per-tool (L2) view for this source. */
        onopen?: (source: ToolHealthSource) => void;
    }

    const { source, active = false, onopen }: Props = $props();

    // A source is "probed" only when its tool registry was enumerated. Null
    // tools_registered means the server could never be started — we have no
    // share to draw, so the card degrades to an honest usage-only line
    // (`invoked N · registered —`) with no bar. See the screen spec's
    // Wrong-gate: a zero bar or fabricated percentage is equally wrong.
    const probed = $derived(source.tools_registered != null);
    const registered = $derived(source.tools_registered ?? 0);
    const pct = $derived(
        source.share_invoked != null ? Math.round(source.share_invoked * 100) : 0,
    );
    // Source-type glyph — 器 (vessel) for a built-in tool set, 接 (connect)
    // for a probed MCP server.
    const kanji = $derived(source.source_type === "builtin" ? "器" : "接");
</script>

<button
    type="button"
    class="w-full text-left bg-paper-soft border border-paper-edge rounded-lg py-3 px-4 flex flex-col gap-3 cursor-pointer"
    class:opacity-60={!source.connected}
    data-testid={`tool-health-card-${source.source_key}`}
    data-source-type={source.source_type}
    data-probed={probed}
    aria-pressed={active}
    onclick={() => onopen?.(source)}
>
    <!-- Header — kanji · name + type/key · connected chip -->
    <div class="flex items-start gap-2">
        <span class="kanji text-xl text-accent leading-none w-6 text-center shrink-0">{kanji}</span>
        <div class="flex-1 min-w-0">
            <div class="text-base text-ink truncate">{source.name}</div>
            <div class="flex items-center gap-2 font-mono text-xs text-ink-mute mt-1">
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
                class:text-ink-mute={!source.connected}
            >{source.connected ? "on" : "off"}</span>
        </span>
    </div>

    {#if probed}
        <!-- Share-invoked: headline % + proportional bar + count detail -->
        <div class="flex items-baseline gap-2">
            <span
                class="display text-2xl leading-none"
                class:text-ink={pct >= 70}
                class:text-warning={pct < 70}
                style='font-feature-settings: "tnum";'
            >{pct}%</span>
            <span class="text-xs text-ink-mute">
                invoked {source.tools_invoked_14d} of {registered} tools
            </span>
        </div>
        <div class="h-2 rounded-sm overflow-hidden bg-paper-mute" data-testid="share-bar">
            <div class="h-full bg-accent" style={`width: ${pct}%;`}></div>
        </div>
    {:else}
        <!-- Un-probed server — honest usage-only line, no bar. -->
        <div class="text-sm text-ink-soft" data-testid="registered-none">
            invoked {source.tools_invoked_14d} · registered —
        </div>
    {/if}

    <!-- Calls chip -->
    <div
        class="flex gap-3 pt-2 border-t border-paper-edge font-mono text-xs text-ink-mute"
        style='font-feature-settings: "tnum";'
    >
        <span>{source.calls_14d.toLocaleString()} calls · 14d</span>
    </div>
</button>
