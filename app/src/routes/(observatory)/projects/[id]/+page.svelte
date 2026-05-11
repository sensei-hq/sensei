<script lang="ts">
    import { page } from "$app/state";
    import { onMount } from "svelte";
    import { appState } from "$lib/appstate.svelte.js";
    import { senseiApi } from "$lib/api.js";
    import type { ProjectListItem } from "$lib/types.js";
    import TabBar from "$lib/components/TabBar.svelte";

    let projectId = $derived(page.params.id);

    type Repo = {
        repo_id: string;
        name: string;
        path: string;
        role?: string;
        language?: string;
    };

    let project = $state<ProjectListItem | null>(null);
    let repos = $state<Repo[]>([]);
    let loading = $state(true);
    let tab = $state("overview");

    const projectTabs: [string, string][] = [
        ["overview", "Overview"],
        ["graph", "Graph"],
        ["patterns", "Patterns"],
        ["sessions", "Sessions"],
        ["settings", "Settings"],
    ];

    onMount(async () => {
        const api = senseiApi(appState.port);
        const projects = await api.listProjects();
        project = projects.find((p) => p.id === projectId) ?? null;
        const allRepos = await api.getRepos();
        repos = allRepos.filter((r: any) => r.project_id === projectId);
        loading = false;
    });

    let kanji = $derived(project?.icon?.value ?? "場");
    let stackTags = $derived([
        ...(project?.stack?.languages ?? []),
        ...(project?.stack?.frameworks ?? []),
    ]);
</script>

<div class="max-w-[820px] mx-auto px-12 py-12 pb-16">
    {#if loading}
        <p class="text-ui text-surface-z6">Loading project...</p>
    {:else if !project}
        <div class="flex flex-col items-center text-center py-20 gap-4">
            <span class="kanji text-6xl text-primary-z5 opacity-30">場</span>
            <p class="display text-xl font-normal m-0">Project not found.</p>
        </div>
    {:else}
        <!-- Project header -->
        <header class="mb-7">
            <div class="flex items-center gap-4 mb-2">
                <span class="kanji text-4xl text-primary-z5 opacity-70"
                    >{kanji}</span
                >
                <div class="flex-1">
                    <h1 class="display text-2xl font-normal m-0">
                        {project.name}
                    </h1>
                    {#if project.client}
                        <span class="text-xs text-surface-z6"
                            >{project.client}</span
                        >
                    {/if}
                </div>
                <span
                    class="px-3 py-1 rounded-full text-2xs bg-surface-z3 text-surface-z6 capitalize"
                    >{project.maturity}</span
                >
            </div>
            {#if project.goal}
                <p class="text-sm text-surface-z7 m-0 mb-3 leading-normal">
                    {project.goal}
                </p>
            {/if}
            {#if stackTags.length > 0}
                <div class="flex gap-1.5 flex-wrap">
                    {#each stackTags as tag}
                        <span
                            class="px-2.5 py-0.75 rounded-full text-2xs bg-surface-z3 text-surface-z6"
                            >{tag}</span
                        >
                    {/each}
                </div>
            {/if}
        </header>

        <TabBar tabs={projectTabs} bind:active={tab} class="mb-7" />

        <!-- Tab content -->
        {#if tab === "overview"}
            <div class="flex flex-col gap-7">
                <div>
                    <h3 class="text-sm font-medium m-0 mb-3.5 text-surface-z9">
                        Repositories
                    </h3>
                    {#if repos.length === 0}
                        <p class="text-ui text-surface-z6">
                            No repositories linked to this project.
                        </p>
                    {:else}
                        <div class="flex flex-col gap-0.5">
                            {#each repos as repo (repo.repo_id)}
                                <div
                                    class="repo-row flex items-center gap-3 px-3.5 py-2.5 rounded-md transition-colors duration-100"
                                >
                                    <span
                                        class="text-ui font-medium text-surface-z9"
                                        >{repo.name}</span
                                    >
                                    <span
                                        class="text-xs text-surface-z6 font-mono flex-1"
                                        >{repo.path}</span
                                    >
                                    {#if repo.role}
                                        <span
                                            class="text-3xs uppercase tracking-widest text-surface-z5 px-2 py-0.5 rounded-full bg-surface-z3"
                                            >{repo.role}</span
                                        >
                                    {/if}
                                </div>
                            {/each}
                        </div>
                    {/if}
                </div>

                <div>
                    <h3 class="text-sm font-medium m-0 mb-3.5 text-surface-z9">
                        Recommendations
                    </h3>
                    <div
                        class="flex flex-col items-center gap-2 px-5 py-10 bg-surface-z2 border border-surface-z3 rounded-lg text-center"
                    >
                        <span class="kanji text-2xl text-primary-z5 opacity-40"
                            >薦</span
                        >
                        <p
                            class="text-ui text-surface-z6 leading-relaxed max-w-[420px] m-0"
                        >
                            Recommendations will appear once sensei has observed
                            enough sessions in this project.
                        </p>
                    </div>
                </div>
            </div>
        {:else if tab === "graph"}
            <div
                class="flex flex-col items-center gap-4 px-5 py-20 bg-surface-z2 border border-surface-z3 rounded-lg text-center"
            >
                <span class="kanji text-5xl text-primary-z5 opacity-30">紋</span
                >
                <p class="display text-lg font-normal m-0">Code graph</p>
                <p
                    class="text-ui text-surface-z6 leading-relaxed max-w-[420px] m-0"
                >
                    Interactive code graph with three lenses: Complexity,
                    Rework, Staleness. Nodes sized by fan-in, colored by
                    overlay.
                </p>
            </div>
        {:else if tab === "patterns"}
            <div class="flex flex-col gap-7">
                {#each [{ title: "Followed patterns", hint: "Detected patterns (Adapter, Observer, Factory, Repository) will appear once the indexing pipeline has analyzed this project's code." }, { title: "Anti-patterns", hint: "Anti-patterns (duplication, god-nodes, dead code) with severity and suggested fixes will appear here after indexing." }] as section}
                    <div>
                        <h3
                            class="text-sm font-medium m-0 mb-3.5 text-surface-z9"
                        >
                            {section.title}
                        </h3>
                        <p class="text-ui text-surface-z6 leading-relaxed">
                            {section.hint}
                        </p>
                    </div>
                {/each}
            </div>
        {:else if tab === "sessions"}
            <div
                class="flex flex-col items-center gap-2 px-5 py-10 bg-surface-z2 border border-surface-z3 rounded-lg text-center"
            >
                <span class="kanji text-3xl text-primary-z5 opacity-40">刻</span
                >
                <p
                    class="text-ui text-surface-z6 leading-relaxed max-w-[420px] m-0"
                >
                    Sessions scoped to this project. Same format as the
                    observatory sessions view, filtered to show only sessions in {project.name}.
                </p>
            </div>
        {:else if tab === "settings"}
            <div>
                <h3 class="text-sm font-medium m-0 mb-3.5 text-surface-z9">
                    Identity
                </h3>
                <div class="grid grid-cols-[120px_1fr] gap-2 gap-x-4">
                    {#each [["Name", project.name], ["Client", project.client ?? "—"], ["Goal", project.goal ?? "—"], ["Preferred ACP", project.preferred_acp ?? "—"], ["Maturity", project.maturity]] as [label, val]}
                        <span class="text-xs text-surface-z6">{label}</span>
                        <span class="text-ui text-surface-z9">{val}</span>
                    {/each}
                </div>
            </div>
        {/if}
    {/if}
</div>

<style>
    .repo-row:hover {
        background: oklch(var(--color-surface-z2) / 1);
    }
</style>
