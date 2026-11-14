<script lang="ts">
    import { personaList } from '$lib/personas.svelte.js';
    import { onMount } from 'svelte';
    import { page } from '$app/state';
    import { beforeNavigate, goto } from '$app/navigation';
    import { commands } from '@rokkit/states';
    import { shortcuts } from '@rokkit/actions';
    import { CommandPalette } from '@rokkit/ui';
    import { appState } from '$lib/appstate.svelte.js';
    import { senseiApi, failureReason } from '$lib/api.js';
    import { isSettingsPath, openSettingsWindow, openWorkspaceWindow } from '$lib/stores/windows.svelte.js';
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

        // Per-workspace "jump to workspace" commands load asynchronously.
        let unregisterWorkspaces: (() => void) | undefined;
        void (async () => {
            await appState.load();
            // Cache the workspace count once for the rail badge (workspaces rarely change).
            await appState.loadWorkspaceCount();
            // A failed read registers no workspace commands and says why in the
            // log — the palette offers actions, it asserts nothing (#288).
            const read = await senseiApi(appState.port).tryListWorkspaces();
            if (!read.ok) {
                console.error('[observatory] workspace command load failed', failureReason(read.error));
                return;
            }
            unregisterWorkspaces = commands.registerMany(
                read.data.map((p) => ({
                    id: `workspace.open:${p.id}`,
                    label: `Open workspace · ${p.name}`,
                    group: 'Workspaces',
                    keywords: p.client ? [p.client] : undefined,
                    run: () => { void openWorkspaceWindow(p.id, p.name); },
                })),
            );
        })();

        return () => {
            unregister();
            unregisterWorkspaces?.();
        };
    });

    // Settings is its own window (user, 2026-10-27, as a Mac app's preferences
    // are): a link to a settings page — the rail's Settings, Dōjō · Connections'
    // link — opens that window at the page, and this window stays where it is.
    // An e2e build drives this window alone and opens settings here instead
    // (`__SENSEI_E2E__`, set only by the e2e-testing build).
    beforeNavigate((nav) => {
        const to = nav.to?.url.pathname;
        if ((window as { __SENSEI_E2E__?: boolean }).__SENSEI_E2E__) return;
        if (!to || !isSettingsPath(to) || nav.type === 'leave') return;
        nav.cancel();
        void openSettingsWindow(to === '/settings' ? '/settings/general' : to);
    });
</script>

<div class="w-full h-screen flex flex-col bg-paper-soft text-ink overflow-hidden" use:shortcuts={commands}>
    <div class="drag-region h-8 shrink-0"></div>

    <div class="flex-1 flex min-h-0">
        <ObservatorySidebar
            port={appState.port}
            pathname={page.url.pathname}
            workspaceCount={appState.workspaceCount ?? undefined}
            glyphs={glyphMode(appState.config)}
            attention={personaList.attentionLine}
            onAttention={() => void openSettingsWindow('/settings/dojo')}
        />

        <!-- Not a scroller: each screen's body is, and takes focus itself (#245, #293). -->
        <main data-component="observatory-main" class="flex-1 min-w-0 min-h-0 overflow-hidden">
            {@render children()}
        </main>
    </div>
</div>

<CommandPalette bind:open={paletteOpen} />
