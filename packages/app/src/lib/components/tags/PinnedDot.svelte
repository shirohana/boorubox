<script lang="ts">
  // A pinned tag's mark, wherever the tag is read as a row: a dot rather than
  // a glyph because it costs 4px plus the row's gap, and muted at 70% because
  // the search's tint and weight are the only loud marks a tag may carry — a
  // louder dot would read as a second kind of "active". Takes the group's
  // label itself so the component stays free of the vocabulary store.
  import HoverHint from './HoverHint.svelte'

  interface Props {
    /** The label of the group the tag is pinned in; `null` for a tag that is not pinned. */
    group: string | null
    /** Where the tooltip portals: set inside the viewer, as `TagNoteIndicator`'s is. */
    portalTo?: Element
  }

  let { group, portalTo }: Props = $props()
</script>

{#if group !== null}
  <HoverHint text="Pinned in {group}" {portalTo}>
    <span aria-hidden="true" class="size-1 shrink-0 rounded-full bg-muted-foreground/70"></span>
    <span class="sr-only">Pinned in {group}</span>
  </HoverHint>
{/if}
