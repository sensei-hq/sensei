<script lang="ts">
    import { scanState } from "$lib/scan.js";
    import { miniSummary } from "@sensei/state";
    import { Notice, failureNotice } from "@sensei/ui";

    const workspaces = scanState.workspaces;
    const failNotice = $derived(
        scanState.status === "error" ? failureNotice("Scan failed", scanState.error) : null,
    );

    const summary = $derived(miniSummary(scanState.queueSnapshot, workspaces.completedFiles, workspaces.totalFiles));
    const pct = $derived(
        workspaces.totalFiles > 0 ? (workspaces.completedFiles / workspaces.totalFiles) * 100 : null,
    );
    // The footer's compact progress: visible while a run is live or failed,
    // nothing while idle — sidebar footers keep their current content.
    const visible = $derived(scanState.status === "scanning" || scanState.status === "error");
</script>

{#if visible}
    <div data-component="scan-mini" class="px-2 pt-3 border-t border-paper-edge">
        {#if failNotice}
            <Notice size="inline" expanded {...failNotice} />
            <button
                type="button"
                class="text-xs text-ink-mute bg-transparent border-0 p-0 mt-1 cursor-pointer hover:text-ink"
                onclick={() => (scanState.expanded = true)}
            >
                Details
            </button>
        {:else if !scanState.daemonReachable}
            <div class="text-xs text-warning">Daemon unreachable — retrying…</div>
        {:else}
            <button
                type="button"
                aria-label="Expand indexing progress"
                title="Show indexing details"
                class="block w-full text-left bg-transparent border-0 p-0 cursor-pointer"
                onclick={() => (scanState.expanded = true)}
            >
                <span class="block text-xs font-mono text-ink-soft tabular-nums">{summary}</span>
                {#if pct !== null}
                    <span class="block h-1 bg-paper-mute rounded-sm overflow-hidden mt-1.5">
                        <span
                            class="block h-full bg-warning rounded-sm transition-[width] duration-slow"
                            style="width: {pct}%"
                        ></span>
                    </span>
                {/if}
            </button>
        {/if}
    </div>
{/if}
