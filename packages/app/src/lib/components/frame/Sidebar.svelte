<script lang="ts">
  import ChevronsUpDownIcon from '@lucide/svelte/icons/chevrons-up-down'
  import ImagesIcon from '@lucide/svelte/icons/images'
  import LibraryIcon from '@lucide/svelte/icons/library'
  import SettingsIcon from '@lucide/svelte/icons/settings'
  import Trash2Icon from '@lucide/svelte/icons/trash-2'
  import { resolve } from '$app/paths'
  import { page } from '$app/state'
  import { library, trash } from '$lib/api'
  import LibraryMenu from '$lib/components/frame/LibraryMenu.svelte'
  import { libraryName } from '$lib/components/frame/library-name'
  import NotesPanel from '$lib/components/notes/NotesPanel.svelte'
  import * as Sidebar from '$lib/components/ui/sidebar'
  import { frame } from './frame.svelte'

  interface NavItem {
    /** `resolve()`'s own type: a plain string is not a route this app has. */
    href: ReturnType<typeof resolve>
    label: string
    icon: typeof ImagesIcon
    /** Read at render, so the badge follows the count (`trash` design D11). */
    count?: () => number
  }

  // Only screens that exist (spec `app-frame`): Import and Rules are added by
  // the changes that build them, not reserved here.
  const nav: NavItem[] = [
    { href: resolve('/'), label: 'Library', icon: ImagesIcon },
    { href: resolve('/trash'), label: 'Trash', icon: Trash2Icon, count: () => trash.count },
    { href: resolve('/settings'), label: 'Settings', icon: SettingsIcon },
  ]

  const path = $derived(library.status?.libraryPath ?? null)
  const name = $derived(libraryName(path))
  const imageCount = $derived(library.status?.imageCount ?? 0)

  let error = $state<string | null>(null)

  // The badge's one read that no action made (`trash` design D11): the count
  // belongs to the library that is open, so opening another one asks again.
  // Every write to the trash refreshes it from where it was made.
  //
  // The path, not the status: a capture and every trash write replace the whole
  // status object, and this effect would then re-read the count on each of
  // them — a round trip per capture, for a number none of them changed.
  $effect(() => {
    void path
    void trash.refresh()
  })
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
      <!--
        `data-sidebar="filters"` is the hook app.css scrolls this region by: its
        rules sit beside the copy-in content wrapper's, which cannot be reached
        from a class here. Dropping the attribute drops the scrollbar gutter and
        the sideways-scroll clip with it.
      -->
      <div
        data-sidebar="filters"
        class="min-h-0 flex-1 overflow-y-auto group-data-[collapsible=icon]:hidden"
      >
        {@render frame.filters()}
      </div>
      <Sidebar.Separator class="my-2 group-data-[collapsible=icon]:hidden" />
    {/if}

    <!--
      Slot Sidebar · filters, below the tag list (`notes` design D15). Mounted
      by the frame rather than by the library screen: the note is the library's,
      not a description of the current result set, so it stays while the user is
      on /settings or /trash. There is no note without a library, and no frame
      either, so nothing here guards for one. Hidden on the icon rail for the
      reason the filters are: a text area clipped to 3rem is not one.
    -->
    <div class="group-data-[collapsible=icon]:hidden">
      <NotesPanel />
    </div>
    <Sidebar.Separator class="my-2 group-data-[collapsible=icon]:hidden" />

    <!--
      Nav sits at the bottom, directly above the library footer: the filters are
      used on every search and the two nav items a few times a session, so the
      filters start at the top. `mt-auto` pins the nav there on routes with no
      filters to render.
    -->
    <Sidebar.Menu class="mt-auto gap-1">
      {#each nav as item (item.href)}
        {@const count = item.count?.() ?? 0}
        <Sidebar.MenuItem>
          <Sidebar.MenuButton isActive={page.url.pathname === item.href}>
            {#snippet child({ props })}
              <a href={item.href} {...props}>
                <item.icon />
                <span>{item.label}</span>
              </a>
            {/snippet}
          </Sidebar.MenuButton>
          <!--
            An empty trash gets no badge: a permanent `0` beside the entry is
            noise, and the entry itself already says the trash is there.
          -->
          {#if count > 0}
            <Sidebar.MenuBadge>{count.toLocaleString()}</Sidebar.MenuBadge>
          {/if}
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
  <!--
    The rail collapses and expands the sidebar; nothing in the frame is
    resizable (design D10), so its edge must not offer a resize cursor. The
    copy-in's cursors are variant-scoped (`in-data-[side=left]:`, and a second
    pair for the collapsed state), which a plain utility does not outrank —
    hence `!`, and hence the override from here rather than an edit to the
    copy-in, which `shadcn-svelte add sidebar` would overwrite.
  -->
  <Sidebar.Rail class="cursor-pointer!" />
</Sidebar.Root>
