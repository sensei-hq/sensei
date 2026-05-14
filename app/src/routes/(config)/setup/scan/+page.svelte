<script lang="ts">
    import { onDestroy } from "svelte";
    import { appState } from "$lib/appstate.svelte.js";
    import { senseiApi } from "$lib/api.js";
    import { wizardState } from "$lib/wizard-state.svelte.js";
    import { EventManager } from "$lib/events.js";
    import {
        ScanProjectState,
        ScanActivityState,
    } from "$lib/scan-state.svelte.js";
    import type {
        StateEvent,
        ScanProject,
        ScanFolderEvent,
        ActivityEvent,
    } from "$lib/types.js";

    const projects = new ScanProjectState();
    const activities = new ScanActivityState();

    let rootCount = $state(0);
    let started = $state(false);
    let unsub: (() => void) | null = null;
    let pollTimer: ReturnType<typeof setInterval> | null = null;

    const LEVEL_COLORS: Record<string, string> = {
        discover: "oklch(var(--color-surface-z6) / 1)",
        queue: "oklch(var(--color-warning-z5) / 1)",
        process: "oklch(var(--color-primary-z5) / 1)",
        info: "oklch(var(--color-surface-z7) / 1)",
        success: "oklch(var(--color-success-z5) / 1)",
        error: "oklch(var(--color-primary-z5) / 1)",
    };

    /**
     * Poll /api/index/status until the task queue is idle (pending=0, running=0).
     *
     * SSE gives us per-item progress events but no "queue idle" event, so we poll
     * the status endpoint as the authoritative completion signal. The interval is
     * long (1.5 s) and there is no artificial upper limit — large codebases can
     * take hours and should not be cut short.
     *
     * Polling stops naturally when:
     *   • The queue reaches idle (2 consecutive empty polls to debounce transients)
     *   • The component is destroyed (onDestroy clears the interval)
     *   • The user navigates away (same)
     */
    function startDonePoller(api: ReturnType<typeof senseiApi>) {
        let idlePolls = 0;
        setTimeout(() => {
            pollTimer = setInterval(async () => {
                try {
                    const s = await api.getIndexStatus();
                    if (s.queue.pending === 0 && s.queue.running === 0) {
                        idlePolls++;
                        // Require 2 consecutive idle polls to debounce a transient empty queue
                        if (idlePolls >= 2) {
                            wizardState.scan.done = true;
                            if (pollTimer) clearInterval(pollTimer);
                        }
                    } else {
                        idlePolls = 0; // reset on any active work
                    }
                } catch {
                    /* daemon temporarily unreachable — keep polling */
                }
            }, 1500);
        }, 1500);
    }

    async function startScan() {
        started = true;
        const api = senseiApi(appState.port);

        // Get root count from scan roots
        const roots = await api.getScanRoots();
        rootCount = roots.length;

        // Connect to SSE
        const events = new EventManager<StateEvent<any>>(
            `http://127.0.0.1:${appState.port}/api/scan/events`,
            (data) => JSON.parse(data),
        );

        unsub = events.subscribe((event) => {
            if (event.entity === "project")
                projects.apply(event as StateEvent<ScanProject>);
            if (event.entity === "folder")
                projects.applyFolder(event as StateEvent<ScanFolderEvent>);
            if (event.entity === "activity")
                activities.apply(event as StateEvent<ActivityEvent>);
        });

        // Only trigger scan for roots not yet scanned — already-watched roots
        // are managed by the daemon's file watcher and don't need re-posting.
        for (const root of roots.filter((r) => !r.scanned)) {
            await api.scanFolder(root.path);
        }

        // Begin polling for task queue idle — marks scan done when complete
        startDonePoller(api);
    }

    onDestroy(() => {
        unsub?.();
        if (pollTimer) clearInterval(pollTimer);
    });
</script>

<div class="max-w-[960px]">
    {#if !started}
        <div
            class="flex flex-col items-center text-center gap-6 bg-surface-z2 rounded-lg px-10 py-12"
        >
            <div class="kanji text-7xl text-primary-z5 opacity-25 leading-none">
                探
            </div>
            <p
                class="text-body text-surface-z7 max-w-[440px] leading-relaxed m-0"
            >
                The daemon will recurse your folders, identify folders, and
                extract the code graph.
            </p>
            <button class="btn-solid" onclick={startScan}>Begin scan →</button>
        </div>
    {:else}
        <!-- Stats bar -->
        <div
            class="grid grid-cols-4 gap-4 mb-6 pb-6 border-b border-surface-z2"
        >
            {#each [{ value: rootCount, label: "ROOTS" }, { value: activities.discovered, label: "DISCOVERED" }, { value: activities.queued, label: "QUEUED" }, { value: activities.processed, label: "PROCESSED" }] as stat}
                <div class="text-center">
                    <div class="display text-3xl leading-tight">
                        {stat.value}
                    </div>
                    <div class="text-3xs tracking-cap text-surface-z6 mt-1">
                        {stat.label}
                    </div>
                </div>
            {/each}
        </div>

        <div class="grid grid-cols-[1fr_340px] gap-6">
            <!-- Left: Project cards -->
            <div class="flex flex-col gap-4">
                {#each projects.items as proj (proj.id)}
                    {@const path = projects.projectPath(proj)}
                    {@const folderCount = proj.folders.length}
                    {@const readyCount = proj.folders.filter(
                        (f) => f.status === "indexed",
                    ).length}
                    {@const totalFiles = proj.folders.reduce(
                        (s, f) => s + f.filesTotal,
                        0,
                    )}
                    {@const completedFiles = proj.folders.reduce(
                        (s, f) => s + f.filesCompleted,
                        0,
                    )}

                    <div
                        class="bg-surface-z2 rounded-lg p-5 border border-surface-z3"
                    >
                        <div class="flex justify-between items-start mb-3">
                            <div>
                                <div class="text-body font-semibold">
                                    {proj.name}
                                </div>
                                <div class="text-xs text-surface-z6 mt-0.5">
                                    {path} · {folderCount} folders · {readyCount}
                                    ready
                                </div>
                            </div>
                            <span
                                class="text-3xs tracking-widest text-surface-z6 font-semibold"
                                class:text-success-z5={proj.status === "active"}
                                >{proj.status.toUpperCase()}</span
                            >
                        </div>

                        {#if totalFiles > 0}
                            <div class="text-ui text-surface-z7 font-mono mb-3">
                                {completedFiles.toLocaleString()} / {totalFiles.toLocaleString()}
                            </div>
                        {/if}

                        <div class="flex flex-col gap-2">
                            {#each proj.folders as f (f.id)}
                                <div class="flex items-center gap-3">
                                    <span
                                        class="text-ui font-medium min-w-[120px]"
                                        >{f.name}</span
                                    >
                                    {#if f.stack.length > 0}
                                        <span
                                            class="text-2xs text-surface-z6 min-w-[80px]"
                                            >{f.stack.join(", ")}</span
                                        >
                                    {:else if f.filesTotal > 0}
                                        <span
                                            class="text-2xs text-surface-z6 min-w-[80px]"
                                            >{f.filesTotal}f</span
                                        >
                                    {/if}
                                    {#if f.filesTotal > 0}
                                        <div
                                            class="flex-1 h-1 bg-surface-z3 rounded-sm overflow-hidden"
                                        >
                                            <div
                                                class="h-full bg-warning-z5 rounded-sm transition-[width] duration-300"
                                                style="width: {(f.filesCompleted /
                                                    f.filesTotal) *
                                                    100}%"
                                            ></div>
                                        </div>
                                    {/if}
                                </div>
                            {/each}
                        </div>
                    </div>
                {/each}

                {#if projects.items.length === 0 && started}
                    <div class="text-ui text-surface-z5 p-6 text-center">
                        Discovering projects...
                    </div>
                {/if}
            </div>

            <!-- Right: Activity feed -->
            <div
                class="bg-surface-z2 rounded-lg p-4 border border-surface-z3 max-h-[600px] flex flex-col"
            >
                <div
                    class="flex justify-between pb-3 border-b border-surface-z2 mb-3 text-2xs text-surface-z6"
                >
                    <span class="mono">SSE · /EVENTS</span>
                    <span class="mono text-surface-z7"
                        >{activities.totalElapsed.toFixed(1)}s</span
                    >
                </div>
                <div class="flex-1 overflow-y-auto flex flex-col gap-0.5">
                    {#each activities.recent as evt (evt.id)}
                        <div class="flex gap-3 text-2xs py-0.5 items-baseline">
                            <span
                                class="mono text-surface-z5 min-w-[52px] text-right"
                                >+{evt.elapsed.toFixed(2)}s</span
                            >
                            <span
                                class="font-medium min-w-[56px]"
                                style="color: {LEVEL_COLORS[evt.level] ??
                                    'oklch(var(--color-surface-z6) / 1)'}"
                                >{evt.level}</span
                            >
                            <span class="text-surface-z7 flex-1"
                                >{evt.message}</span
                            >
                        </div>
                    {/each}
                </div>
            </div>
        </div>
    {/if}
</div>
