<script lang="ts">
    import { page } from "$app/state";

    let { data, children } = $props();

    const SECTIONS = [
        {
            id: "overview",
            kanji: "見",
            label: "Overview",
            href: () => `/project/${data.projectId}/overview`,
        },
        {
            id: "sessions",
            kanji: "録",
            label: "Sessions",
            href: () => `/project/${data.projectId}/sessions`,
        },
        {
            id: "memories",
            kanji: "憶",
            label: "Memories",
            href: () => `/project/${data.projectId}/memories`,
        },
        {
            id: "traceability",
            kanji: "跡",
            label: "Traceability",
            href: () => `/project/${data.projectId}/traceability`,
        },
        {
            id: "libraries",
            kanji: "蔵",
            label: "Libraries",
            href: () => `/project/${data.projectId}/libraries`,
        },
        {
            id: "instruments",
            kanji: "器",
            label: "Instruments",
            href: () => `/project/${data.projectId}/instruments`,
        },
        {
            id: "patterns",
            kanji: "型",
            label: "Patterns",
            href: () => `/project/${data.projectId}/patterns`,
        },
        {
            id: "impact",
            kanji: "響",
            label: "Impact",
            href: () => `/project/${data.projectId}/impact`,
        },
        {
            id: "about",
            kanji: "情",
            label: "About",
            href: () => `/project/${data.projectId}/about`,
        },
    ];

    function isActive(sectionId: string): boolean {
        return page.url.pathname.startsWith(
            `/project/${data.projectId}/${sectionId}`,
        );
    }

    let kanji = $derived(data.project?.icon?.value ?? "場");
    let ftr = $derived(Math.round((data.ftrMetrics?.ftr14d ?? 0) * 100));
</script>

<div
    class="flex flex-col h-screen overflow-hidden bg-surface-z1 text-surface-z9"
>
    <!-- Primary accent stripe -->
    <div class="h-0.5 bg-primary-z5 shrink-0"></div>

    <!-- Titlebar / drag region -->
    <div class="drag-region h-9 flex items-center gap-2 px-4 shrink-0">
        <span class="kanji text-lg text-primary-z5">{kanji}</span>
        <span class="text-sm font-semibold">{data.project?.name ?? "…"}</span>
        <span class="text-xs opacity-50">· project window</span>
    </div>

    <div class="flex flex-1 overflow-hidden">
        <aside
            class="w-[180px] shrink-0 border-r border-surface-z2 flex flex-col py-3"
        >
            <div class="px-4 pb-4 pt-2">
                <span class="text-2xl font-bold block">{ftr}%</span>
                <span class="text-xs opacity-50">FTR 14d</span>
            </div>

            <nav class="flex flex-col" aria-label="Project sections">
                {#each SECTIONS as section (section.id)}
                    {@const active = isActive(section.id)}
                    <a
                        href={section.href()}
                        class="proj-nav-item flex items-center gap-2.5 px-4 py-2 no-underline text-inherit text-sm transition-colors duration-fast"
                        class:active
                    >
                        <span class="kanji w-4.5 text-center" aria-hidden="true"
                            >{section.kanji}</span
                        >
                        <span>{section.label}</span>
                    </a>
                {/each}
            </nav>
        </aside>

        <main class="flex-1 overflow-y-auto">
            {@render children()}
        </main>
    </div>
</div>

<style>
    .proj-nav-item:hover {
        background: oklch(var(--color-surface-z2) / 1);
    }
    .proj-nav-item.active {
        background: oklch(var(--color-surface-z2) / 1);
        color: oklch(var(--color-primary-z5) / 1);
    }
</style>
