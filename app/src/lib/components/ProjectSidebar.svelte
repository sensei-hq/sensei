<script lang="ts">
  import { page } from '$app/state';

  interface Section {
    id: string;
    kanji: string;
    label: string;
    badge?: number;
  }

  interface Props {
    projectId: string;
    projectName: string;
    projectKanji?: string;
    client?: string | null;
    sections: Section[];
    onSwitchProject?: () => void;
  }

  const {
    projectId,
    projectName,
    projectKanji = '場',
    client = null,
    sections,
    onSwitchProject,
  }: Props = $props();

  // Match the sidebar entry against the current pathname's trailing segment
  // so navigating between /projects/[id]/overview, /sessions, /libraries …
  // reflects the active section without a runtime prop.
  const activeSection = $derived.by(() => {
    const parts = page.url.pathname.split('/').filter(Boolean);
    return parts[parts.length - 1] ?? sections[0]?.id;
  });
</script>

<aside class="flex flex-col gap-4 h-full py-5 px-3 border-r border-paper-edge bg-paper-soft box-border overflow-auto">
  <div class="px-1">
    <div class="flex items-center gap-3">
      <span class="kanji text-2xl text-accent opacity-70">{projectKanji}</span>
      <div class="flex-1 min-w-0">
        <div class="text-xs uppercase tracking-wide text-ink-mute">Project</div>
        <div class="text-base font-medium text-ink truncate">{projectName}</div>
      </div>
    </div>
    {#if client}
      <div class="mono mt-2 text-xs text-ink-mute">{client}</div>
    {/if}
    {#if onSwitchProject}
      <button
        class="mt-2 py-1 px-2 text-xs text-ink-mute border border-paper-edge rounded bg-transparent cursor-pointer"
        onclick={onSwitchProject}
      >
        ⇆ switch project
      </button>
    {/if}
  </div>

  <div>
    <div class="pt-0 pb-2 px-2 text-xs uppercase tracking-wide text-ink-mute">
      This project
    </div>
    <nav class="flex flex-col gap-1">
      {#each sections as section (section.id)}
        {@const active = activeSection === section.id}
        <a
          href={`/projects/${projectId}/${section.id}`}
          class="section-row flex items-center gap-3 py-2 px-2.5 rounded-md no-underline transition-colors duration-fast"
          class:bg-paper-mute={active}
          class:text-ink={active}
          class:text-ink-soft={!active}
          aria-current={active ? 'page' : undefined}
        >
          <span class="kanji text-lg text-accent opacity-80 w-5 text-center">{section.kanji}</span>
          <span class="flex-1 text-sm">{section.label}</span>
          {#if section.badge != null && section.badge > 0}
            <span
              class="px-1.5 py-0.5 rounded-full text-xs bg-paper-mute text-ink-mute"
              class:bg-primary-soft={active}
              class:text-primary={active}
            >{section.badge}</span>
          {/if}
        </a>
      {/each}
    </nav>
  </div>
</aside>

<style>
  .section-row:hover {
    background: var(--paper-mute);
  }
</style>
