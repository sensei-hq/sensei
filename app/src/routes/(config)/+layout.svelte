<script lang="ts">
    import { page } from "$app/state";
    import { goto } from "$app/navigation";
    import { onMount } from "svelte";
    import {
        stageIndex,
        nextStagePath,
        prevStagePath,
    } from "./stages.js";
    import { wizardState } from "$lib/wizard-state.svelte.js";
    import { appState } from "$lib/appstate.svelte.js";
    import { StatusDot } from "$lib/components";

    let { children } = $props();

    const stages = $derived(wizardState.stages);
    const currentIdx = $derived(stageIndex(page.url.pathname));
    const stage = $derived(stages[currentIdx]);
    const isFirst = $derived(currentIdx === 0);
    const isLast = $derived(currentIdx === stages.length - 1);
    const total = $derived(stages.length);
    const canAdvance = $derived(wizardState.canAdvance(stage?.id ?? ""));
    let committing = $state(false);
    let loaded = $state(false);
    let commitError = $state<string | null>(null);

    // Drive the transient `active` flag from the current route so both the
    // rail and the header read the same shape per stage.
    $effect(() => {
        if (stage) wizardState.setActive(stage.id);
    });

    onMount(async () => {
        // wizardState.load() owns the fetch+apply cycle — matches
        // healthState.init() / appState.load(). The layout doesn't reach
        // into the daemon directly.
        await wizardState.load();
        loaded = true;
        // Daemon-canonical: if setup is already complete, the user was
        // dropped here during the cold-start race (appState wasn't loaded
        // when reroute decided). Send them to the observatory now —
        // UNLESS the user explicitly asked to re-enter setup (via the
        // View → Setup menu, which appends `?force=1`). That's the path
        // that lets a fully-configured user re-run the wizard.
        const forced = page.url.searchParams.get("force") === "1";
        if (appState.setupOk && !forced) goto("/");
    });

    async function next() {
        if (committing) return;
        if (!canAdvance) return;

        committing = true;
        commitError = null;
        try {
            if (isLast) {
                await wizardState.commitStage("done");
                // Reload appState so reroute sees setup_complete=1 before goto fires.
                await appState.load();
                // The microtask break + invalidateAll are load-bearing:
                // without yielding once, pending wizard-layout $effects can
                // hold an in-flight navigation context and SvelteKit will
                // coalesce the goto into a no-op (reroute never fires).
                // invalidateAll re-runs every layout/page load so the
                // observatory doesn't render with stale wizard state.
                await Promise.resolve();
                await goto("/", { invalidateAll: true });
                return;
            }
            await wizardState.commitStage(stage.id);
            const path = nextStagePath(page.url.pathname);
            if (path) goto(path);
        } catch (e) {
            commitError = e instanceof Error ? e.message : String(e);
        } finally {
            committing = false;
        }
    }

    function back() {
        const path = prevStagePath(page.url.pathname);
        if (path) goto(path);
    }
</script>

<div
    class="w-full h-screen flex flex-col bg-paper-soft text-ink overflow-hidden"
>
    <div class="drag-region h-8 shrink-0"></div>

    <div class="flex-1 grid grid-cols-[260px_1fr] min-h-0">
        <!-- Rail -->
        <aside
            class="flex flex-col px-6 py-6 border-r border-paper-mute bg-paper-mute overflow-hidden"
        >
            <div class="flex items-baseline gap-2 mb-7">
                <span class="kanji text-[22px] text-accent">先生</span>
                <span class="display text-lg">Sensei</span>
            </div>

            <div class="text-xs tracking-wide text-ink-soft uppercase mb-3.5">
                Setup
            </div>

            <div data-testid="rail" class="flex flex-col">
                {#each stages as s (s.id)}
                    {@const isDone = s.status === "done"}
                    <button
                        data-rail-item
                        data-stage-id={s.id}
                        data-active={s.active}
                        class="grid grid-cols-[24px_1fr_14px] px-2 py-1 gap-2.5 items-center rounded-md text-left border border-transparent text-ink-soft cursor-pointer transition-all duration-fast text-sm"
                        class:active={s.active}
                        class:done={isDone}
                        onclick={() => goto(s.path)}
                    >
                        <span
                            class="rail-kanji kanji text-sm text-center text-ink-soft"
                            class:active={s.active}
                            class:done={isDone}
                            class:text-accent={s.active}>{s.icon}</span
                        >
                        <div class="overflow-hidden">
                            <div class:leading-tight={s.active}>
                                {s.title}
                            </div>
                            {#if s.active}
                                <div
                                    class="mono text-xs text-ink-soft mt-0.5"
                                >
                                    {s.brief}
                                </div>
                            {/if}
                        </div>
                        <span
                            class="text-xs text-center leading-none text-success opacity-0 transition-opacity duration-fast"
                            class:opacity-100={isDone}>✓</span
                        >
                    </button>
                {/each}
            </div>

            <div class="mt-auto border-t border-paper-mute pt-3">
                <div class="flex items-center gap-2.5">
                    <StatusDot status="ok" />
                    <div class="text-xs text-ink-mute leading-snug">
                        <div
                            class="tracking-wide uppercase text-xs text-ink-soft"
                        >
                            Services
                        </div>
                        <div class="mt-0.5">all green</div>
                    </div>
                </div>
            </div>
        </aside>

        <!-- Content -->
        <div class="flex flex-col min-h-0">
            {#if stage?.id !== "welcome"}
                <div
                    class="shrink-0 px-16 pt-7 pb-6 border-b border-paper-mute bg-paper-soft relative z-1"
                >
                    <div
                        class="text-xs text-ink-soft tracking-wide uppercase mb-2"
                    >
                        <span class="kanji text-accent mr-1.5 opacity-60"
                            >{stage.icon}</span
                        >Step
                    </div>
                    <h1
                        class="display text-4xl font-light tracking-tight m-0 mb-1.5"
                    >
                        {stage.title}
                    </h1>
                    <p class="text-sm text-ink-soft m-0">
                        {stage.description}
                    </p>
                </div>
            {/if}

            <div
                class="flex-1 overflow-y-auto px-16 relative"
                class:py-8={stage?.id !== "welcome"}
                class:pt-11={stage?.id === "welcome"}
                class:pb-8={stage?.id === "welcome"}
            >
                {#if stage?.watermark}
                    <span
                        class="watermark kanji absolute right-16 bottom-8 text-4xl text-accent opacity-[0.035] leading-none select-none pointer-events-none z-0"
                        >{stage.icon}</span
                    >
                {/if}
                {@render children()}
            </div>

            {#if commitError}
                <div class="mx-16 mb-2 p-3 rounded-md border border-danger bg-paper-mute text-xs text-danger select-text">
                    Could not finish: {commitError} — fix and try Continue again.
                </div>
            {/if}

            <!-- Bottom nav -->
            <div
                class="flex items-center gap-5 px-16 py-3.5 border-t border-paper-mute bg-paper-soft shrink-0"
            >
                <div class="flex items-baseline gap-3">
                    <span
                        class="text-xs tracking-wide text-ink-soft uppercase"
                    >
                        {String(currentIdx + 1).padStart(2, "0")}
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
                    <button
                        class="btn-back text-xs text-ink-mute px-3.5 py-1 leading-3 bg-none border-none cursor-pointer"
                        onclick={back}
                        disabled={isFirst}
                    >
                        ← Back
                    </button>
                    <button
                        class="btn-primary text-sm leading-3 bg-ink text-paper-soft px-6 py-2.5 rounded-md border-none tracking-normal cursor-pointer"
                        onclick={next}
                        disabled={!canAdvance || committing}
                    >
                        {#if committing}
                            {stage?.id === "assistants"
                                ? "Configuring…"
                                : "Saving…"}
                        {:else if isLast}
                            Enter observatory →
                        {:else if stage?.id === "assistants"}
                            Configure &amp; Continue →
                        {:else}
                            Continue →
                        {/if}
                    </button>
                </div>
            </div>
        </div>
    </div>
</div>

<style>
    /* Rail item states */
    /*.rail-item {
        padding: 7px 10px;
    }*/
    [data-rail-item].active {
        background: var(--paper-soft);
        border-color: var(--paper-mute);
        color: var(--ink);
    }
    [data-rail-item].done {
        color: var(--ink-mute);
    }

    /* Rail kanji states */
    .rail-kanji.done {
        color: var(--ink-mute);
    }

    /* Back/primary button disabled states */
    .btn-back:disabled {
        color: var(--ink-soft);
        cursor: default;
    }
    .btn-primary:hover:not(:disabled) {
        opacity: 0.9;
    }
    .btn-primary:disabled {
        background: var(--paper-mute);
        color: var(--ink-soft);
        cursor: default;
    }
</style>
