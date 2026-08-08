<script lang="ts">
    import { page } from '$app/state';
    import { Eyebrow } from '$lib/components';

    interface Props {
        projectId: string;
        ftr14d: number; // 0..1
    }
    let { projectId, ftr14d }: Props = $props();

    const SECTIONS = [
        { id: 'intake',       kanji: '門', label: 'Intake' },
        { id: 'overview',     kanji: '見', label: 'Overview' },
        { id: 'sessions',     kanji: '録', label: 'Sessions' },
        { id: 'memories',     kanji: '憶', label: 'Memories' },
        { id: 'traceability', kanji: '跡', label: 'Traceability' },
        { id: 'atlas',        kanji: '図', label: 'Atlas' },
        { id: 'libraries',    kanji: '蔵', label: 'Libraries' },
        { id: 'instruments',  kanji: '器', label: 'Instruments' },
        { id: 'patterns',     kanji: '型', label: 'Patterns' },
        { id: 'impact',       kanji: '響', label: 'Impact' },
        { id: 'about',        kanji: '情', label: 'About' },
    ];

    const ftrPct = $derived(Math.round(ftr14d * 100));

    function isActive(sectionId: string): boolean {
        return page.url.pathname.startsWith(`/project/${projectId}/${sectionId}`);
    }
</script>

<aside
    data-component="project-sidebar"
    class="w-[180px] shrink-0 border-r border-paper-edge flex flex-col py-3"
>
    <div class="px-4 pb-4 pt-2">
        <span class="text-2xl font-bold block">{ftrPct}%</span>
        <span class="text-xs text-ink-soft">FTR 14d</span>
    </div>

    <div class="px-4 pb-2"><Eyebrow>This project</Eyebrow></div>

    <nav class="flex flex-col" aria-label="Project sections">
        {#each SECTIONS as section (section.id)}
            {@const active = isActive(section.id)}
            <a
                href="/project/{projectId}/{section.id}"
                class="proj-nav-item flex items-center gap-2.5 px-4 py-2 no-underline text-inherit text-sm transition-colors duration-fast"
                class:active
            >
                <span class="kanji w-4 text-center" aria-hidden="true">{section.kanji}</span>
                <span>{section.label}</span>
            </a>
        {/each}
    </nav>
</aside>

<style>
    .proj-nav-item:hover {
        background: var(--paper-mute);
    }
    .proj-nav-item.active {
        background: var(--paper);
        color: var(--accent);
    }
</style>
