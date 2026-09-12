<script lang="ts">
  // The band under the window's top edge, spanning sidebar and content. It is
  // the frame's, not the route's: the sidebar toggle has to be in the same
  // place whether the sidebar is open or collapsed to its rail (the rail is
  // too narrow to hold it, and a control that disappears when it is needed
  // most is how the first build shipped), and with the macOS title bar
  // overlaid this is where the window is dragged from (design D13). The
  // route's own controls — search, view, import — come through `frame.toolbar`.
  //
  // This reverses the first build's page-rendered band, whose argument was
  // that a frame-drawn bar would be empty on /settings. With the toggle in it
  // the bar is never empty, so that argument no longer holds.
  import * as Sidebar from '$lib/components/ui/sidebar'
  import { windowDragRegion } from '$lib/platform'
  import { frame } from './frame.svelte'
</script>

<!--
  The traffic lights float over the top-left (D13): `ps-20` reserves their 80px,
  keyed off the attribute app.html stamps on <html>. Only the header element
  itself drags; the controls in it keep working.
-->
<header
  data-tauri-drag-region={windowDragRegion}
  class="
    flex h-14 shrink-0 items-center gap-3 border-b border-border px-3
    in-data-[platform=macos]:ps-20
  "
>
  <Sidebar.Trigger />
  {@render frame.toolbar?.()}
</header>
