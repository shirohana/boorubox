<script lang="ts">
  // `app-shell` design D16: no requirement lists what the settings screen
  // holds; each page here is the screen half of a fact another capability
  // owns.
  import { resolve } from '$app/paths'
  import { page } from '$app/state'
  import { isCurrentPath } from '$lib/components/frame/current-path'
  import { rememberSettingsPage, SETTINGS_PAGES } from '$lib/components/settings/settings-pages'
  import { windowDragRegion } from '$lib/platform'

  let { children } = $props()

  // `settings-pages` design D3: written on every page shown, from a
  // `$derived` slug — `page.url` is replaced on every navigation, so an
  // effect reading a field off it directly would re-run on every one of them
  // (repo CLAUDE.md's `$effect` rule); this depends on the derived slug
  // instead.
  const currentPage = $derived(
    SETTINGS_PAGES.find((item) => isCurrentPath(page.url.pathname, resolve(item.path))),
  )
  const currentSlug = $derived(currentPage?.slug ?? null)
  $effect(() => {
    if (currentSlug) rememberSettingsPage(currentSlug)
  })
</script>

<!-- No toolbar band on this screen, so its background is the drag region (D13). -->
<div
  data-tauri-drag-region={windowDragRegion}
  class="@container min-h-0 flex-1 overflow-y-auto"
>
  <div class="mx-auto flex w-full max-w-4xl flex-col @3xl:flex-row">
    <nav class="
      shrink-0 px-8 pt-8
      @3xl:sticky @3xl:top-0 @3xl:w-44 @3xl:self-start @3xl:pr-0 @3xl:pb-8
    ">
      <h1 class="text-xl font-semibold">Settings</h1>
      <ul class="mt-4 flex flex-wrap gap-1 @3xl:flex-col">
        {#each SETTINGS_PAGES as item (item.slug)}
          {@const href = resolve(item.path)}
          {@const current = isCurrentPath(page.url.pathname, href)}
          <li>
            <a
              {href}
              aria-current={current ? 'page' : undefined}
              class="
                block rounded-md px-2 py-1.5 text-sm text-muted-foreground
                hover:bg-accent hover:text-accent-foreground
              "
              class:bg-accent={current}
              class:text-accent-foreground={current}
              class:font-medium={current}
            >
              {item.label}
            </a>
          </li>
        {/each}
      </ul>
    </nav>

    <div class="flex max-w-2xl min-w-0 flex-1 flex-col gap-10 p-8">
      {@render children()}
    </div>
  </div>
</div>
