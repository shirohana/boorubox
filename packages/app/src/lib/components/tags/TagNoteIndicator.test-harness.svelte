<script lang="ts">
  // Test support only, for `TagNoteIndicator.test.ts`: `Tooltip.Root`
  // (bits-ui 2.19) reads its provider from context and throws without one, and
  // in the running app that provider is the frame's
  // (`ui/sidebar/sidebar-provider.svelte`), an ancestor no unit test mounts.
  // This stands in for it — the same reason `tag-notes` design D8 gives for
  // not mounting a second provider inside `TagNoteIndicator` itself: one
  // already covers every real mount point, just not this one.
  import * as Tooltip from '$lib/components/ui/tooltip'
  import TagNoteIndicator from './TagNoteIndicator.svelte'

  interface Props {
    note: string | null
    portalTo?: Element
  }

  let { note, portalTo }: Props = $props()
</script>

<Tooltip.Provider delayDuration={0}>
  <TagNoteIndicator {note} {portalTo} />
</Tooltip.Provider>
