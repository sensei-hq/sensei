<script lang="ts">
  // Set the crew up, or add to it (COMPONENTS §3e AddMember; CONTRACT §1h): the title and
  // a line on how it works, the role (Coordinator disabled once taken), the step and the
  // line to paste with Copy, and what the screen is listening for. The catalogue picker
  // (#455): the assistants and editors this machine has seen, as pressed-state lists;
  // picking one words the note under the line, never the line itself. A failed read is
  // a Notice with Retry, and the line stays usable.
  import { Notice } from '@sensei/ui';
  import type { RelayAddMemberView, RelayCatalogueItemView, RelayRole } from '@sensei/state';

  let {
    view,
    oncopy,
    onrole,
    onpick = () => {},
    onretrycatalogue = () => {},
  }: {
    view: RelayAddMemberView;
    oncopy: (text: string) => void;
    onrole: (role: RelayRole) => void;
    onpick?: (id: string) => void;
    onretrycatalogue?: () => void;
  } = $props();

  const groups = $derived(
    view.catalogue
      ? [
          { id: 'assistants', label: 'Assistants', items: view.catalogue.assistants },
          { id: 'editors', label: 'Editors', items: view.catalogue.editors },
        ]
      : [],
  );

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
  {#snippet tool(item: RelayCatalogueItemView)}
    <button
      type="button"
      data-catalogue-item={item.id}
      aria-pressed={item.pressed}
      onclick={() => onpick(item.id)}
      class={[
        'flex flex-col items-start gap-0.5 min-w-[160px] px-3 py-2 rounded-lg border border-solid text-left cursor-pointer',
        item.pressed ? 'border-primary bg-primary-soft text-ink' : 'border-paper-edge bg-paper text-ink',
      ]}
    >
      <span class="text-sm font-semibold">{item.name}</span>
      <span class="text-xs text-ink-soft">{item.used}</span>
      {#each item.details as detail, i (i)}
        <span class="text-xs text-ink-soft [overflow-wrap:anywhere]">{detail}</span>{' '}
      {/each}
    </button>
  {/snippet}
  {#if view.catalogueError}
    <Notice
      data-catalogue-error
      tone="fault"
      size="inline"
      title="Couldn't read the assistants"
      detail={view.catalogueError}
      primary={{ label: 'Retry', name: 'retry-catalogue', onclick: onretrycatalogue }}
    />
  {:else if view.catalogue}
    <div class="flex flex-col gap-3">
      {#each groups as group (group.id)}
        {#if group.items.length}
          <div data-catalogue-group={group.id} role="group" aria-label={group.label} class="flex flex-col gap-1.5">
            <span class="text-xs font-semibold text-ink-soft uppercase tracking-wide">{group.label}</span>
            <div class="flex flex-wrap gap-2">
              {#each group.items as item (item.id)}{@render tool(item)}{/each}
            </div>
          </div>
        {/if}
      {/each}
    </div>
  {/if}
  <div class="flex flex-col gap-2 p-3 rounded-[10px] bg-paper-soft">
    <p class="m-0 text-sm">
      {#if view.command.step}<strong data-command-step>{view.command.step}</strong>{/if}
      Paste this into an assistant session.
    </p>
    <div data-paste-line class="flex items-center gap-2 py-1.5 pl-3 pr-1.5 rounded-lg border border-solid border-paper-edge bg-paper">
      <code data-paste-text class="flex-1 min-w-0 font-mono text-xs [overflow-wrap:anywhere]">{view.command.text}</code>
      <button
        type="button"
        data-paste-copy
        onclick={() => oncopy(view.command.text)}
        class="min-h-[30px] px-2.5 rounded-[7px] border border-solid border-paper-edge bg-paper text-ink text-xs cursor-pointer"
      >Copy</button>
    </div>
    <p data-command-note class="m-0 text-xs text-ink-soft [overflow-wrap:anywhere]">{view.command.note}</p>
  </div>
  <p data-listening class="m-0 flex items-center gap-2 text-xs text-ink-soft">
    <span aria-hidden="true" class="inline-block w-2 h-2 rounded-full bg-success"></span>{view.listening}
  </p>
</section>
