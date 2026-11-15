<script lang="ts">
  // The three tags a member card carries (COMPONENTS.md §3a): the family (the
  // assistant's name), the model (mono), and the host ("in Cursor", with a title
  // that says what the editor is). It draws only its props.
  import { tagClass, tagText, tagTitle, type TagKind } from './primitives.js';

  let {
    kind = 'family',
    label = '',
    editor = '',
  }: { kind?: TagKind; label?: string; editor?: string } = $props();
</script>

<span data-tag={kind} title={tagTitle(kind, editor)} class={tagClass(kind)}>{tagText(kind, label, editor)}</span>
