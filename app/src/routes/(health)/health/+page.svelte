<script lang="ts">
    import { onMount } from "svelte";
    import { goto } from "$app/navigation";
    import { healthState } from "$lib/health-state.svelte.js";
    import HealthView from "./HealthView.svelte";

    // Boot-timing — remove with the rest of the marks once boot loader ships.
    (window as any).__senseiMark?.("health-page-script");

    let firstUpdateLogged = false;
    $effect(() => {
        // Touch a reactive field so this $effect re-fires on health updates.
        const _ = healthState.components.length;
        if (!firstUpdateLogged && _ > 0) {
            firstUpdateLogged = true;
            (window as any).__senseiMark?.("health-first-update");
        }
    });

    onMount(() => {
        (window as any).__senseiMark?.("health-page-mount");
        healthState.init();
        (window as any).__senseiMark?.("health-init-returned");
    });

    // Auto-leave the health page once the gate is green. The reroute hook
    // then decides whether to land at /setup/welcome (setup not complete)
    // or / (observatory). Without this, a successful resolve leaves the
    // user staring at the "all green" view waiting for a manual click.
    $effect(() => {
        if (healthState.isOk) {
            goto("/", { replaceState: true });
        }
    });

    function onEnter() {
        goto("/", { replaceState: true });
    }
    function onVerify() {
        healthState.verify();
    }
    function onCopyScript() {
        if (healthState.remedy)
            navigator.clipboard?.writeText(healthState.remedy.script);
    }
</script>

hello
<HealthView state={healthState} {onEnter} {onVerify} {onCopyScript} />
