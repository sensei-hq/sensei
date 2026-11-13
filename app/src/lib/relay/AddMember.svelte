<script lang="ts">
  // Set the crew up, or add to it (COMPONENTS §3e AddMember; CONTRACT §1h): the title and
  // a line on how it works, the role (Coordinator disabled once taken), the step and the
  // line to paste with Copy, and what the screen is listening for. The catalogue picker
  // (assistants and editors seen on this machine) arrives when the daemon serves it.
  import type { RelayAddMemberView, RelayRole } from '@sensei/state';

  let {
    view,
    oncopy,
    onrole,
  }: { view: RelayAddMemberView; oncopy: (text: string) => void; onrole: (role: RelayRole) => void } = $props();

  const ROLES: Array<{ value: RelayRole; label: string }> = [
    { value: 'coordinator', label: 'Coordinator' },
    { value: 'contributor', label: 'Contributor' },
  ];
</script>

<section data-add-member aria-label={view.title} class="flex flex-col gap-3.5 p-4 rounded-xl border border-solid border-paper-edge bg-paper max-w-[980px]">
  <div class="flex flex-col gap-1">
    <h3 data-add-member-title class="m-0 display text-lg font-medium">{view.title}</h3>
    <p class="m-0 text-sm text-ink-soft">{view.lead}</p>
  </div>
  <div class="flex flex-wrap items-center gap-3">
    <span class="text-sm text-ink-soft">As</span>
    <span role="group" aria-label="Role" class="inline-flex p-0.5 rounded-[9px] bg-paper-mute">
      {#each ROLES as r (r.value)}
        <button
          type="button"
          data-role-option={r.value}
          aria-pressed={view.role === r.value}
          disabled={view.roleLocked && r.value === 'coordinator'}
          onclick={() => onrole(r.value)}
          class={[
            'min-h-[30px] px-3 rounded-[7px] border-0 text-sm cursor-pointer disabled:cursor-default',
            view.role === r.value ? 'bg-paper font-semibold text-ink ring-1 ring-paper-edge' : 'bg-transparent text-ink-soft disabled:text-ink-mute',
          ]}
        >{r.label}</button>
      {/each}
    </span>
    {#if view.roleLocked}<span class="text-sm text-ink-soft">The coordinator is set; contributors join from here.</span>{/if}
  </div>
  <div class="flex flex-col gap-2 p-3 rounded-[10px] bg-paper-soft">
    <p class="m-0 text-sm">
      {#if view.command.step}<strong data-command-step>{view.command.step}</strong>{/if}
      Paste this into an assistant session.
    </p>
    <div data-paste-line class="flex items-center gap-2 py-1.5 pl-3 pr-1.5 rounded-lg border border-solid border-paper-edge bg-paper">
      <code data-paste-text class="flex-1 min-w-0 font-mono text-xs truncate">{view.command.text}</code>
      <button
        type="button"
        data-paste-copy
        onclick={() => oncopy(view.command.text)}
        class="min-h-[30px] px-2.5 rounded-[7px] border border-solid border-paper-edge bg-paper text-ink text-xs cursor-pointer"
      >Copy</button>
    </div>
    <p class="m-0 text-xs text-ink-soft">{view.command.note}</p>
  </div>
  <p data-listening class="m-0 flex items-center gap-2 text-xs text-ink-soft">
    <span aria-hidden="true" class="inline-block w-2 h-2 rounded-full bg-success"></span>{view.listening}
  </p>
</section>
