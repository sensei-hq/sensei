<script lang="ts">
    import { onMount } from "svelte";
    import { goto } from "$app/navigation";
    import { healthState } from "$lib/health-state.svelte.js";
    import HealthView from "./HealthView.svelte";

    // `auto` controls whether the page navigates away on its own when health
    // reaches ok. Default true (cold-start bootstrap flow). `?auto=false` —
    // emitted by the View → Health menu entry — keeps the user on the screen
    // and surfaces a Continue button so they navigate forward explicitly.
    let auto = $state(true);

    onMount(() => {
        if (typeof window !== "undefined") {
            const params = new URLSearchParams(window.location.search);
            if (params.get("auto") === "false") auto = false;
        }
        healthState.init();
    });

    // Auto-leave the health page once the gate is green. The reroute hook
    // then decides whether to land at /setup/welcome (setup not complete)
    // or / (observatory). Only fires when `auto` is true.
    $effect(() => {
        if (!auto) return;
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
    // Clipboard copy is owned by Remedy.svelte now — it tracks success/failure
    // state internally and shows feedback in the button label. The parent
    // no longer needs to drive it.
</script>

<HealthView state={healthState} {auto} {onEnter} {onVerify} />
