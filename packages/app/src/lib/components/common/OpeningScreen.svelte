<script lang="ts">
  // Covers both places the layout has nothing else to show (`launch-screen`
  // design D2): before the first `library_status`/`app_settings` answer, and
  // while a remembered library's launch-time open runs on its blocking
  // thread. The first has no folder to name yet; the second does, once
  // `LibraryStatus.opening` carries it.
  import { libraryName } from '$lib/components/frame/library-name'
  import { windowDragRegion } from '$lib/platform'

  interface Props {
    /** `LibraryStatus.opening` — the folder a launch-time open is running
     * against. `null` for the plain pre-answer state. */
    path?: string | null
  }

  let { path = null }: Props = $props()
</script>

<!-- No frame yet, so the window's background is the drag region, same as
     /start (D13). -->
<main
  data-tauri-drag-region={windowDragRegion}
  class="flex min-h-svh items-center justify-center p-8"
>
  <p class="text-sm text-muted-foreground">
    {path ? `Opening ${libraryName(path)}…` : 'Opening your library…'}
  </p>
</main>
