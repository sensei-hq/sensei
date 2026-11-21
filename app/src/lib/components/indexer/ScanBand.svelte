<script lang="ts">
    import { goto } from "$app/navigation";
    import { scanState } from "$lib/scan.js";
    import { appState } from "$lib/appstate.svelte.js";
    import { Button } from "@rokkit/ui";
    import { Notice, failureNotice } from "@sensei/ui";

    const workspaces = scanState.workspaces;
    const activities = scanState.activities;
    const failNotice = $derived(
        scanState.status === "error" ? failureNotice("Scan failed", scanState.error) : null,
    );

    // Full-width but small-height: stats plus the latest activity, only while
    // the user expanded it and a run is live (or failed). Collapses itself
    // when the run ends.
    const visible = $derived(
        scanState.expanded && (scanState.status === "scanning" || scanState.status === "error"),
    );
    // Post-setup the scan route redirects unless re-entered with ?force=1
    // (the View → Setup path) — the band uses that re-entry.
    const fullHref = $derived(appState.setupOk ? "/setup/scan?force=1" : "/setup/scan");
    const latest = $derived(activities.recent[0]?.message ?? null);
    const pct = $derived(
        workspaces.totalFiles > 0 ? (workspaces.completedFiles / workspaces.totalFiles) * 100 : null,
    );
</script>

{#if visible}
    <div
        data-component="scan-band"
        class="fixed bottom-0 inset-x-0 z-40 border-t border-paper-edge bg-paper-soft"
    >
        <div class="flex items-center gap-4 px-4 py-2 text-xs max-w-[1200px] mx-auto">
            {#if failNotice}
                <span class="flex-1 min-w-0"><Notice size="inline" expanded {...failNotice} /></span>
            {:else if !scanState.daemonReachable}
                <span class="text-warning font-medium">Daemon unreachable — retrying…</span>
                <span class="flex-1"></span>
            {:else}
                <span class="font-mono text-ink-soft tabular-nums whitespace-nowrap">
                    {scanState.rootCount} roots · {activities.discovered} discovered ·
                    {workspaces.pendingFolders} queued · {workspaces.readyFolders} processed
                </span>
                {#if pct !== null}
                    <span class="flex-1 h-1 bg-paper-mute rounded-sm overflow-hidden min-w-[60px]">
                        <span
                            class="block h-full bg-warning rounded-sm transition-[width] duration-slow"
                            style="width: {pct}%"
                        ></span>
                    </span>
                    <span class="font-mono text-ink-soft tabular-nums whitespace-nowrap">
                        {workspaces.completedFiles.toLocaleString()}/{workspaces.totalFiles.toLocaleString()}
                    </span>
                {:else}
                    <span class="flex-1"></span>
                {/if}
                {#if latest}
                    <span class="text-ink-mute truncate max-w-[320px]">{latest}</span>
                {/if}
            {/if}
            <Button style="link" size="sm" onclick={() => (scanState.expanded = false)} label="Collapse" />
            <Button style="link" size="sm" onclick={() => void goto(fullHref)} label="Full view" />
        </div>
    </div>
{/if}
