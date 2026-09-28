<script lang="ts">
  // The note's own reader (`tag-notes` design D8): one small glyph, wherever
  // a tag with a note is drawn — the sidebar row, an inspector badge, a
  // pinned chip. Renders nothing for a tag with no note. Takes `note` itself
  // rather than a tag name, so the component stays free of the vocabulary
  // store, which is what this component's own test mounts without one.
  import StickyNoteIcon from '@lucide/svelte/icons/sticky-note'
  import * as Tooltip from '$lib/components/ui/tooltip'

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
    `delayDuration={150}` overrides the frame's own `Tooltip.Provider
    delayDuration={0}` (`ui/sidebar/sidebar-provider.svelte`, which suits the
    sidebar's icon buttons and not a text that pops over a dense list); no new
    provider is mounted here — the frame's already wraps every mount point.
    150 ms, down from 400 (owner, 2026-09-28: too slow to read a note in
    passing) — enough that a pointer crossing the list does not pop every
    note it passes, short enough that a pause on one reads at once.
  -->
  <Tooltip.Root delayDuration={150}>
    <Tooltip.Trigger tabindex={-1}>
      {#snippet child({ props })}
        <!--
          A `<span>`, not the trigger's default `<button>`: the badge and the
          chip this mounts inside of are buttons of their own, and a button
          cannot nest inside one. `tabindex={-1}` above keeps this span out of
          the Tab order too: bits-ui defaults the trigger's tabindex to 0,
          which would otherwise both add a stop `tag-notes` design D8 never
          asked for (hover only) and nest a focusable element inside the host
          button. `shrink-0`: in a host's flex row the glyph keeps its width
          rather than squeezing under the tag name.
        -->
        <span {...props} class="inline-flex shrink-0">
          <StickyNoteIcon class="size-3 shrink-0 text-muted-foreground" aria-label="Tag note" />
        </span>
      {/snippet}
    </Tooltip.Trigger>
    <!--
      `block … whitespace-pre-wrap wrap-break-word` over the upstream
      `inline-flex … max-w-xs` (20rem, kept): a note is prose, not a label, so
      it wraps as written instead of staying on one line.
    -->
    <Tooltip.Content
      portalProps={{ to: portalTo }}
      class="block text-left wrap-break-word whitespace-pre-wrap"
    >
      {note}
    </Tooltip.Content>
  </Tooltip.Root>
{/if}
