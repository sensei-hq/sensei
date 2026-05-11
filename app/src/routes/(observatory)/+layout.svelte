<script lang="ts">
    import { page } from "$app/state";
    import { onMount } from "svelte";
    import { appState } from "$lib/appstate.svelte.js";
    import { senseiApi } from "$lib/api.js";
    import { openProjectWindow } from "$lib/stores/windows.svelte.js";

    let { children } = $props();

    const NAV_ITEMS = [
        { href: "/observatory", kanji: "家", label: "Today" },
        { href: "/projects", kanji: "場", label: "Projects" },
        { href: "/sessions", kanji: "刻", label: "Sessions" },
        { href: "/insights", kanji: "學", label: "Insights" },
        { href: "/libraries", kanji: "書", label: "Libraries" },
        { href: "/instruments", kanji: "具", label: "Instruments" },
    ];

    const BOTTOM_ITEMS = [
        { href: "/logs", kanji: "録", label: "Logs" },
        { href: "/settings", kanji: "設", label: "Settings" },
    ];

    type SidebarProject = { id: string; name: string; kanji: string };

    let projects = $state<SidebarProject[]>([]);
    let sidebarCollapsed = $state(false);

    onMount(async () => {
        await appState.load();
        const api = senseiApi(appState.port);
        const raw = await api.listProjects();
        projects = raw.map((p: any) => ({
            id: p.id,
            name: p.name,
            kanji: p.icon?.value ?? "場",
        }));
    });

    function isActive(href: string): boolean {
        return (
            page.url.pathname === href ||
            page.url.pathname.startsWith(href + "/")
        );
    }
</script>

<div
    class="w-full h-screen flex flex-col bg-surface-z1 text-surface-z9 overflow-hidden"
>
    <div class="drag-region h-8 shrink-0"></div>

    <div
        class="app-body flex-1 grid grid-cols-[220px_1fr] min-h-0 transition-[grid-template-columns] duration-150"
        class:collapsed={sidebarCollapsed}
    >
        <!-- Sidebar -->
        <aside
            class="border-r border-surface-z2 px-3.5 py-5.5 bg-surface-z2 flex flex-col gap-5 overflow-auto"
        >
            <div class="flex items-baseline gap-2 px-1.5">
                <span class="kanji text-xl text-primary-z5">先</span>
                {#if !sidebarCollapsed}
                    <span class="display text-base">Sensei</span>
                    <button
                        class="collapse-btn ml-auto bg-none border-none text-surface-z6 cursor-pointer text-sm px-1.5 py-0.5 rounded-md"
                        onclick={() => (sidebarCollapsed = true)}>‹</button
                    >
                {/if}
            </div>

            {#snippet navItem(item: typeof NAV_ITEMS[number], collapsed: boolean)}
                {@const active = isActive(item.href)}
                <a
                    href={item.href}
                    class="nav-item flex items-center py-1.75 rounded-md text-ui text-surface-z7 no-underline transition-colors duration-120 hover:bg-surface-z3"
                    class:justify-center={collapsed}
                    class:gap-2.5={!collapsed}
                    class:px-2.5={!collapsed}
                    class:active
                    title={collapsed ? item.label : undefined}
                >
                    <span
                        class="kanji text-ui w-3.5 text-surface-z6"
                        class:nav-kanji-active={active}
                        >{item.kanji}</span
                    >
                    {#if !collapsed}
                        <span>{item.label}</span>
                    {/if}
                </a>
            {/snippet}

            {#snippet projectItem(proj: SidebarProject, collapsed: boolean)}
                {@const active = !collapsed && isActive(`/projects/${proj.id}`)}
                <button
                    type="button"
                    class="nav-item flex items-center py-1.75 rounded-md text-ui text-surface-z7 no-underline transition-colors duration-120 hover:bg-surface-z3 bg-none border-none cursor-pointer w-full"
                    class:justify-center={collapsed}
                    class:gap-2.5={!collapsed}
                    class:px-2.5={!collapsed}
                    class:text-left={!collapsed}
                    onclick={() =>
                        openProjectWindow(proj.id, proj.name).catch(
                            console.error,
                        )}
                    title={collapsed ? `${proj.name} ↗` : `${proj.name} ↗ opens in its own window`}
                >
                    <span
                        class="kanji text-ui w-3.5 text-surface-z6"
                        class:nav-kanji-active={active}
                        >{proj.kanji}</span
                    >
                    {#if !collapsed}
                        <span class="nav-label">{proj.name}</span>
                        <span class="text-3xs opacity-40 ml-auto">↗</span>
                    {/if}
                </button>
            {/snippet}

            {#if sidebarCollapsed}
                <nav class="flex flex-col gap-px">
                    {#each NAV_ITEMS as item (item.href)}
                        {@render navItem(item, true)}
                    {/each}
                </nav>

                {#if projects.length > 0}
                    <div class="h-px bg-surface-z3 mx-2.5"></div>
                    <nav class="flex flex-col gap-px">
                        {#each projects as proj (proj.id)}
                            {@render projectItem(proj, true)}
                        {/each}
                    </nav>
                {/if}

                <div class="mt-auto pt-2.5 border-t border-surface-z2">
                    <button
                        class="collapse-btn bg-none border-none text-surface-z6 cursor-pointer text-sm px-1.5 py-0.5 rounded-md"
                        onclick={() => (sidebarCollapsed = false)}>›</button
                    >
                </div>
            {:else}
                <div class="flex flex-col gap-0.5">
                    <p
                        class="text-micro tracking-label uppercase text-surface-z6 px-2.5 pb-2 m-0"
                    >
                        Observatory
                    </p>
                    <nav class="flex flex-col gap-px">
                        {#each NAV_ITEMS as item (item.href)}
                            {@render navItem(item, false)}
                        {/each}
                    </nav>
                </div>

                {#if projects.length > 0}
                    <div class="flex flex-col gap-0.5">
                        <p
                            class="text-micro tracking-label uppercase text-surface-z6 px-2.5 pb-2 m-0"
                        >
                            Projects
                        </p>
                        <nav class="flex flex-col gap-px">
                            {#each projects as proj (proj.id)}
                                {@render projectItem(proj, false)}
                            {/each}
                        </nav>
                    </div>
                {/if}

                <div class="flex flex-col gap-0.5 mt-auto">
                    <nav class="flex flex-col gap-px">
                        {#each BOTTOM_ITEMS as item (item.href)}
                            {@render navItem(item, false)}
                        {/each}
                    </nav>
                </div>

                <div class="pt-2.5 border-t border-surface-z2">
                    <span class="mono text-3xs text-surface-z6"
                        >daemon · port {appState.port}</span
                    >
                </div>
            {/if}
        </aside>

        <!-- Main content -->
        <main class="overflow-auto">
            {@render children()}
        </main>
    </div>
</div>

<style>
    /* Collapsed grid width */
    .app-body.collapsed {
        grid-template-columns: 52px 1fr;
    }

    /* Active nav item */
    .nav-item.active {
        background: oklch(var(--color-surface-z3) / 1);
        color: oklch(var(--color-surface-z9) / 1);
    }

    /* Active nav kanji accent */
    .nav-kanji-active {
        color: oklch(var(--color-primary-z5) / 1);
    }

    /* Collapse button hover */
    .collapse-btn:hover {
        background: oklch(var(--color-surface-z3) / 1);
        color: oklch(var(--color-surface-z7) / 1);
    }
</style>
