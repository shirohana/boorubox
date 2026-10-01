<script lang="ts">
  // The note's own reader (`tag-notes` design D8): one small glyph, wherever
  // a tag with a note is drawn — the sidebar row, an inspector badge, a
  // pinned chip. Renders nothing for a tag with no note. Takes `note` itself
  // rather than a tag name, so the component stays free of the vocabulary
  // store, which is what this component's own test mounts without one.
  import StickyNoteIcon from '@lucide/svelte/icons/sticky-note'
  import HoverHint from './HoverHint.svelte'

  interface Props {
    note: string | null
    /**
     * Where the tooltip portals (`browse-fixes` design D3): the sidebar row
     * never passes this — it is never inside the viewer — the inspector's
     * badges and chips do.
     */
    portalTo?: Element
  }

  let { note, portalTo }: Props = $props()
</script>

{#if note}
  <!--
    `block … whitespace-pre-wrap wrap-break-word` over the upstream
    `inline-flex … max-w-xs` (20rem, kept): a note is prose, not a label, so
    it wraps as written instead of staying on one line.
  -->
  <HoverHint
    text={note}
    {portalTo}
    contentClass="block text-left wrap-break-word whitespace-pre-wrap"
  >
    <StickyNoteIcon class="size-3 shrink-0 text-muted-foreground" aria-label="Tag note" />
  </HoverHint>
{/if}
