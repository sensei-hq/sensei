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
    import WizardRail from "./WizardRail.svelte";
    import StageHeader from "./StageHeader.svelte";
    import StageNav from "./StageNav.svelte";

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
        <WizardRail {stages} />

        <!-- Content -->
        <div class="flex flex-col min-h-0">
            {#if stage?.id !== "welcome" && stage?.id !== "done"}
                <StageHeader {stage} />
            {/if}

            <!-- key= ensures the scroll container remounts on stage change so
                 scroll position resets to top instead of carrying over -->
            {#key stage?.id}
            <div
                class="flex-1 overflow-y-auto px-16 relative"
                class:py-8={stage?.id !== "welcome" && stage?.id !== "done"}
                class:pt-11={stage?.id === "welcome" || stage?.id === "done"}
                class:pb-8={stage?.id === "welcome" || stage?.id === "done"}
            >
                {#if stage?.watermark}
                    <span
                        class="watermark kanji absolute right-16 bottom-8 text-4xl text-accent opacity-[0.035] leading-none select-none pointer-events-none z-0"
                        >{stage.icon}</span
                    >
                {/if}
                {@render children()}
            </div>
            {/key}

            {#if commitError}
                <div class="mx-16 mb-2 p-3 rounded-md border border-danger bg-danger-soft text-xs text-danger select-text">
                    Could not finish: {commitError} — fix and try Continue again.
                </div>
            {/if}

            <StageNav
                {stage}
                {currentIdx}
                {total}
                {canAdvance}
                {committing}
                {isFirst}
                {isLast}
                onBack={back}
                onNext={next}
            />
        </div>
    </div>
</div>
