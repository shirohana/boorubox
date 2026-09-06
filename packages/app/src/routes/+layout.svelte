<script lang="ts">
  import { goto } from '$app/navigation'
  import { resolve } from '$app/paths'
  import { page } from '$app/state'
  import { library } from '$lib/api'
  import '../app.css'

  let { children } = $props()

  const onSetup = $derived(page.url.pathname === '/setup')
  const libraryOpen = $derived(library.status?.opened === true)

  void library.load()

  // The gate from spec `library-folder`: with no library open, `/setup` is the
  // only reachable route. The children stay unrendered until the redirect has
  // landed — rendering them first flashes the grid, search and import.
  $effect(() => {
    if (library.status && !libraryOpen && !onSetup) void goto(resolve('/setup'))
  })
</script>

{#if library.error}
  <main class="p-6">
    <h1 class="text-lg font-semibold">BooruBox could not read its settings</h1>
    <p class="mt-2 text-sm text-muted-foreground">{library.error}</p>
  </main>
{:else if library.status === null}
  <p class="p-6 text-sm text-muted-foreground">Opening your library…</p>
{:else if libraryOpen || onSetup}
  {@render children()}
{/if}
