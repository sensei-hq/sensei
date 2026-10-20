<script lang="ts">
    import { onMount } from 'svelte';
    import { page } from '$app/state';
    import { goto } from '$app/navigation';
    import { commands } from '@rokkit/states';
    import { shortcuts } from '@rokkit/actions';
    import { CommandPalette } from '@rokkit/ui';
    import { appState } from '$lib/appstate.svelte.js';
    import { senseiApi, failureReason } from '$lib/api.js';
    import { openProjectWindow } from '$lib/stores/windows.svelte.js';
    import ObservatorySidebar from './ObservatorySidebar.svelte';
    import { glyphMode } from '$lib/glyphs';
    import { buildNavItems } from './observatory-nav.js';

    let { children } = $props();

    /** ⌘K command palette open state. */
    let paletteOpen = $state(false);

    onMount(() => {
        // Static commands: open the palette (mod+k, fires even while a field is
        // focused) + jump to each observatory section (reuses the rail nav).
        const navCommands = buildNavItems()
            .flatMap((e) => ('children' in e ? e.children : 'href' in e ? [e] : []))
            .map((link) => ({
                id: `nav:${link.href}`,
                label: `Go to ${link.text}`,
                group: 'Navigate',
                run: () => { void goto(link.href); },
            }));
        const unregister = commands.registerMany([
            {
                id: 'palette.open',
                label: 'Open command palette',
                shortcut: 'mod+k',
                global: true,
                group: 'General',
                run: () => { paletteOpen = true; },
            },
            ...navCommands,
        ]);

        // Per-project "jump to project" commands load asynchronously.
        let unregisterProjects: (() => void) | undefined;
        void (async () => {
            await appState.load();
            // Cache the project count once for the rail badge (projects rarely change).
            await appState.loadProjectCount();
            // A failed read registers no project commands and says why in the
            // log — the palette offers actions, it asserts nothing (#288).
            const read = await senseiApi(appState.port).tryListProjects();
            if (!read.ok) {
                console.error('[observatory] project command load failed', failureReason(read.error));
                return;
            }
            unregisterProjects = commands.registerMany(
                read.data.map((p) => ({
                    id: `project.open:${p.id}`,
                    label: `Open project · ${p.name}`,
                    group: 'Projects',
                    keywords: p.client ? [p.client] : undefined,
                    run: () => { void openProjectWindow(p.id, p.name); },
                })),
            );
        })();

        return () => {
            unregister();
            unregisterProjects?.();
        };
    });
</script>

<div class="w-full h-screen flex flex-col bg-paper-soft text-ink overflow-hidden" use:shortcuts={commands}>
    <div class="drag-region h-8 shrink-0"></div>

    <div class="flex-1 flex min-h-0">
        <ObservatorySidebar
            port={appState.port}
            pathname={page.url.pathname}
            projectCount={appState.projectCount ?? undefined}
            glyphs={glyphMode(appState.config)}
        />

        <!-- Not a scroller: each screen's body is, and takes focus itself (#245, #293). -->
        <main data-component="observatory-main" class="flex-1 min-w-0 min-h-0 overflow-hidden">
            {@render children()}
        </main>
    </div>
</div>

<CommandPalette bind:open={paletteOpen} />
