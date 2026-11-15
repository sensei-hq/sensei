<script lang="ts">
  // The selected identity: who it is, where it stands, and every action for it.
  import { Avatar, Button } from '@rokkit/ui';
  import type { IdentityHeader } from './dojo-connection-state.svelte.js';

  let {
    header,
    onprimary,
    onmakedefault,
    onsignout,
  }: { header: IdentityHeader; onprimary: () => void; onmakedefault: () => void; onsignout: () => void } = $props();
</script>

<div data-identity-header class="flex flex-wrap items-center justify-between gap-3">
  <div class="flex items-center gap-3 min-w-0">
    <Avatar name={header.name} size="md" />
    <div class="flex flex-col gap-0.5 min-w-0">
      <h2 class="m-0 font-display text-xl font-normal text-ink">{header.name}</h2>
      <span class="text-xs text-ink-mute">{header.login} · <span class={header.toneClass}>{header.standing}</span></span>
    </div>
  </div>
  <div class="flex flex-wrap items-center gap-2">
    {#if header.primary}
      <Button variant="primary" size="sm" label={header.primary.label} loading={header.primary.busy} onclick={onprimary} data-action="primary" />
    {/if}
    {#if header.canMakeDefault}
      <Button style="outline" size="sm" label="Make default" disabled={header.busy} onclick={onmakedefault} data-action="make-default" data-make-default />
    {/if}
    {#if header.canSignOut}
      <Button style="outline" size="sm" label="Sign out" disabled={header.busy} onclick={onsignout} data-action="sign-out" />
    {/if}
  </div>
</div>
