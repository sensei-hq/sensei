<script lang="ts">
    import type { ToolUsage } from '$lib/metrics/metric-view.js';

    // The tool-usage bubble view (Pass 2c): one bubble per tool the project's ACPs
    // invoked, its AREA ∝ call count (so size reads honestly), tinted by whether it
    // ever failed. This answers "which tools were used" directly — the thing a
    // single unused-count line can't. Pure template.
    let { tools }: { tools: ToolUsage[] } = $props();

    const maxCalls = $derived(tools.reduce((m, t) => Math.max(m, t.calls), 1));
    // Area ∝ calls → radius ∝ √calls; mapped to a comfortable [30, 92]px range.
    const sizeFor = (calls: number): number =>
        Math.round(30 + (Math.sqrt(calls) / Math.sqrt(maxCalls)) * 62);

    // MCP tools are namespaced (`mcp__plugin_x__leaf`) — show the readable leaf.
    const short = (name: string): string => {
        const parts = name.split('__');
        return parts[parts.length - 1] || name;
    };
</script>

<div data-component="tool-bubbles" class="flex flex-wrap items-end gap-4">
    {#if tools.length}
        {#each tools as t (t.tool)}
            {@const size = sizeFor(t.calls)}
            <div
                class="flex flex-col items-center gap-1"
                style={`width:${Math.max(size, 60)}px`}
                title={`${t.tool} — ${t.calls} calls${t.failed ? `, ${t.failed} failed` : ''} · ${t.sessions} sessions`}
            >
                <div
                    data-tool={t.tool}
                    class="rounded-full flex items-center justify-center border {t.failed
                        ? 'bg-warning-soft border-warning'
                        : 'bg-accent-soft border-accent'}"
                    style={`width:${size}px;height:${size}px`}
                >
                    <span class="tabular-nums text-xs {t.failed ? 'text-warning' : 'text-accent'}"
                        >{t.calls}</span
                    >
                </div>
                <span class="text-xs text-ink-mute truncate w-full text-center">{short(t.tool)}</span>
            </div>
        {/each}
    {:else}
        <div class="text-sm text-ink-mute py-6">No tool usage captured for this project yet.</div>
    {/if}
</div>
