<script lang="ts">
  // A pinned tag's mark, wherever the tag is read as a row: a dot rather than
  // a glyph because it costs 4px plus the row's gap, and muted at 70% because
  // the search's tint and weight are the only loud marks a tag may carry — a
  // louder dot would read as a second kind of "active". Takes `pinned` itself
  // so the component stays free of the vocabulary store.
  interface Props {
    pinned: boolean
  }

  let { pinned }: Props = $props()
</script>

{#if pinned}
  <span aria-hidden="true" class="size-1 shrink-0 rounded-full bg-muted-foreground/70"></span>
  <span class="sr-only">pinned</span>
{/if}
