<script lang="ts">
  import ChevronsUpDownIcon from '@lucide/svelte/icons/chevrons-up-down'
  import ImagesIcon from '@lucide/svelte/icons/images'
  import LibraryIcon from '@lucide/svelte/icons/library'
  import SettingsIcon from '@lucide/svelte/icons/settings'
  import { resolve } from '$app/paths'
  import { page } from '$app/state'
  import { library } from '$lib/api'
  import LibraryMenu from '$lib/components/frame/LibraryMenu.svelte'
  import { libraryName } from '$lib/components/frame/library-name'
  import * as Sidebar from '$lib/components/ui/sidebar'
  import { frame } from './frame.svelte'

  // Only screens that exist (spec `app-frame`): Trash, Import and Rules are
  // added by the changes that build them, not reserved here.
  const nav = [
    { href: resolve('/'), label: 'Library', icon: ImagesIcon },
    { href: resolve('/settings'), label: 'Settings', icon: SettingsIcon },
  ]

  const name = $derived(libraryName(library.status?.libraryPath ?? null))
  const imageCount = $derived(library.status?.imageCount ?? 0)

  let error = $state<string | null>(null)
</script>

<!--
  Below the frame's top bar (which holds the toggle and the drag region, D13):
  nav, then the library footer. Collapsed, the shadcn icon rail keeps the nav
  icons and the library icon in view, so the sidebar never vanishes; the
  toggle in the bar, the rail's edge strip and Cmd+B (bound by the provider)
  bring it back. The `!` overrides move the fixed container under the bar —
  the copy-in pins it to the window's top edge.
-->
<Sidebar.Root
  collapsible="icon"
  class="top-14! h-[calc(100svh-3.5rem)]! border-e border-sidebar-border"
>
  <Sidebar.Content class="p-2">
    <!--
      The filters region, filled by whichever route has a result set to describe
      (`frame.filters`). Collapsed to the icon rail there is no room for a list
      of names, so it is hidden rather than clipped — the same rule the library
      footer follows.
    -->
    {#if frame.filters}
      <div class="min-h-0 flex-1 overflow-y-auto group-data-[collapsible=icon]:hidden">
        {@render frame.filters()}
      </div>
      <Sidebar.Separator class="my-2 group-data-[collapsible=icon]:hidden" />
    {/if}

    <!--
      Nav sits at the bottom, directly above the library footer: the filters are
      used on every search and the two nav items a few times a session, so the
      filters start at the top. `mt-auto` pins the nav there on routes with no
      filters to render.
    -->
    <Sidebar.Menu class="mt-auto gap-1">
      {#each nav as item (item.href)}
        <Sidebar.MenuItem>
          <Sidebar.MenuButton isActive={page.url.pathname === item.href}>
            {#snippet child({ props })}
              <a href={item.href} {...props}>
                <item.icon />
                <span>{item.label}</span>
              </a>
            {/snippet}
          </Sidebar.MenuButton>
        </Sidebar.MenuItem>
      {/each}
    </Sidebar.Menu>
  </Sidebar.Content>

  <!--
    The running total lives here, not on the library screen: it answers "did
    that capture land" without a trip, while the per-source breakdown that only
    matters during a migration is on /settings (design D7).
  -->
  <Sidebar.Footer class="border-t border-sidebar-border p-2">
    <Sidebar.Menu>
      <Sidebar.MenuItem>
        <LibraryMenu align="start" side="top" onerror={(message) => (error = message)}>
          {#snippet trigger({ props })}
            <!--
              `lg` drops its padding when collapsed, which pins the icon to the left edge.
            -->
            <Sidebar.MenuButton
              size="lg"
              class="group-data-[collapsible=icon]:justify-center"
              {...props}
            >
              <LibraryIcon class="shrink-0" />
              <!--
                Collapsed, the rail shows the icon alone: a clipped count reads as a wrong one.
              -->
              <div class="min-w-0 flex-1 text-left group-data-[collapsible=icon]:hidden">
                <p class="truncate text-sm font-medium" title={library.status?.libraryPath ?? ''}>
                  {name}
                </p>
                <p class="text-xs text-muted-foreground tabular-nums">
                  {imageCount.toLocaleString()}
                  {imageCount === 1 ? 'image' : 'images'}
                </p>
              </div>
              <ChevronsUpDownIcon class="group-data-[collapsible=icon]:hidden" />
            </Sidebar.MenuButton>
          {/snippet}
        </LibraryMenu>
      </Sidebar.MenuItem>
    </Sidebar.Menu>
    {#if error}
      <p class="px-2 text-xs text-destructive">{error}</p>
    {/if}
  </Sidebar.Footer>
  <Sidebar.Rail />
</Sidebar.Root>
