<script lang="ts">
  // The grip a `reorderable` list's rows drag by (see `reorder.ts` for the row
  // contract). It is the only part of a row a drag starts from, so clicks
  // and text selection elsewhere in the row stay what they were. The drag
  // itself is started and read by the action; this element only marks where
  // it may begin. `touch-none` keeps a touch from scrolling instead.
  import GripVerticalIcon from '@lucide/svelte/icons/grip-vertical'

  let { class: className = '' }: { class?: string } = $props()
</script>

<button
  type="button"
  data-reorder-handle
  aria-label="Drag to reorder"
  class="
    inline-flex size-6 cursor-grab touch-none items-center justify-center rounded-sm
    text-muted-foreground
    hover:bg-accent hover:text-foreground
    active:cursor-grabbing
    {className}
  "
>
  <GripVerticalIcon class="size-4" />
</button>
