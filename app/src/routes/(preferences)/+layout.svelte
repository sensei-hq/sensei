<script lang="ts">
    import { beforeNavigate } from '$app/navigation';
    import { emitTo } from '@tauri-apps/api/event';
    import { WebviewWindow, getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
    import { isSettingsPath } from '$lib/settings-path.js';

    let { children } = $props();

    // A link out of settings (Dōjō · Connections, say) belongs to the main
    // window: open it there and bring that window forward, rather than drawing
    // the main app inside the settings window.
    // Only from the settings window itself: were the main window showing a
    // settings page, sending its links to "main" would send them to itself.
    const inMain = (() => {
        try {
            return getCurrentWebviewWindow().label === 'main';
        } catch {
            return true; // no Tauri (a browser, a test): nothing to hand off to
        }
    })();
    beforeNavigate((nav) => {
        const to = nav.to?.url.pathname;
        if (inMain || !to || isSettingsPath(to) || nav.type === 'leave') return;
        nav.cancel();
        void emitTo('main', 'dev-navigate', to);
        void WebviewWindow.getByLabel('main').then((w) => w?.setFocus());
    });
</script>

<!-- The settings window's shell: the overlay titlebar's drag strip, then the
     settings sidebar and page (settings/+layout.svelte). -->
<div class="w-full h-screen flex flex-col bg-paper-soft text-ink overflow-hidden" data-component="settings-window">
    <div class="drag-region h-8 shrink-0"></div>
    <div class="flex-1 min-h-0">
        {@render children()}
    </div>
</div>
