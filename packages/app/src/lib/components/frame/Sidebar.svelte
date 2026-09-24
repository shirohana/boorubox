<script lang="ts">
  import ChevronsUpDownIcon from '@lucide/svelte/icons/chevrons-up-down'
  import ImagesIcon from '@lucide/svelte/icons/images'
  import ImportIcon from '@lucide/svelte/icons/import'
  import LibraryIcon from '@lucide/svelte/icons/library'
  import SettingsIcon from '@lucide/svelte/icons/settings'
  import Trash2Icon from '@lucide/svelte/icons/trash-2'
  import { resolve } from '$app/paths'
  import { page } from '$app/state'
  import {
    collections,
    imports,
    library,
    libraryCounts,
    sidecarsBackfill,
    stamps,
    trash,
    vocabulary,
  } from '$lib/api'
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

  // Only screens that exist (spec `app-frame`): Rules is added by the change
  // that builds it, not reserved here.
  const nav: NavItem[] = [
    { href: resolve('/'), label: 'Library', icon: ImagesIcon },
    { href: resolve('/trash'), label: 'Trash', icon: Trash2Icon, count: () => trash.count },
    { href: resolve('/import'), label: 'Import', icon: ImportIcon },
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
  //
  // Also where a library switch resets the import queue's reports and reloads
  // `libraryCounts` (`legacy-bundle-import` design D7): the sidebar is mounted
  // for every route with a library open, unlike `LibraryScreen` or `/import`,
  // so this is the one place the reset does not depend on which screen the
  // switch was made from. The sidecar catch-up tile is reset here too
  // (`library-sidecars` design D13, `pending-work` spec "Switching libraries
  // mid-pass"): Rust stops writing into the folder that was left on its own,
  // but the webview has no further tick to tell it the tile it was showing no
  // longer describes the library now open.
  $effect(() => {
    void path
    void trash.refresh()
    imports.dismissAll()
    sidecarsBackfill.reset()
    void libraryCounts.refresh()
    // `collections` design D7: the menus and the sidebar section need the
    // list itself following a library switch, the same round trip the trash
    // and per-source counts already make here.
    void collections.refresh()
    // `tag-vocabulary` design D5: the vocabulary is per library, like the
    // collections just above, so it re-reads on the same path change — the
    // badges, the sidebar's tag list and the suggestion popover all need it.
    void vocabulary.refresh()
    // `stamps` design D3: stamps are per library too — the settings screen's
    // list and the library screen's stamp bar both read this store.
    void stamps.refresh()
  })

  // The per-source counts also follow every import, library switch or not,
  // wherever the run was started from — `/import` and `/settings` both read
  // `libraryCounts` rather than each polling on their own trigger.
  $effect(() => imports.onfinished(() => void libraryCounts.refresh()))
</script>

<!--
  Below the frame's top bar (which holds the toggle and the drag region, D13):
  nav, then the library footer. Collapsed, the shadcn icon rail keeps the nav
  icons and the library icon in view, so the sidebar never vanishes; the
  toggle in the bar and Cmd+B (bound by the provider) bring it back. The `!`
  overrides move the fixed container under the bar — the copy-in pins it to
  the window's top edge.
-->
<Sidebar.Root
  collapsible="icon"
  class="top-14! h-[calc(100svh-3.5rem)]! border-e border-sidebar-border"
>
  <Sidebar.Content class="p-2">
    <!--
      The sidebar region, filled by whichever route has a result set to
      describe (`frame.sidebar`). Collapsed to the icon rail there is no room
      for a list of names, so it is hidden rather than clipped — the same rule
      the library footer follows. A plain flex column with no scroll of its own
      (`sidebar-layout` design D1): search, rating and filter sections are
      natural height, and only `TagSidebar`'s own section — the one that can
      outgrow the panel — scrolls, via the `data-sidebar="tags"` hook in
      app.css. `overflow-y-auto` on the column is for a window too short for
      even that (`browse-feedback`): the tag list keeps a floor and the
      collections box a height of its own, so past a point the column itself
      has to scroll — without it the filter section was painted under the
      navigation below.
    -->
    {#if frame.sidebar}
      <div
        class="flex min-h-0 flex-1 flex-col overflow-y-auto group-data-[collapsible=icon]:hidden"
      >
        {@render frame.sidebar()}
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

      Nav sits directly below, at the bottom above the library footer: the
      filters are used on every search and the two nav items a few times a
      session, so the filters start at the top. `mt-auto` pins the note (and
      the nav below it) there on a route with no filters to render — the
      library route needs nothing extra, since `TagSidebar` is already
      `flex-1` (`sidebar-inspector-polish` design D5).
    -->
    <div class="mt-auto group-data-[collapsible=icon]:hidden">
      <NotesPanel />
    </div>
    <Sidebar.Separator class="my-2 group-data-[collapsible=icon]:hidden" />

    <!-- The Notes wrapper and the filter div above are both hidden on the icon
         rail, so nothing else pushes the nav down there. -->
    <Sidebar.Menu class="gap-1 group-data-[collapsible=icon]:mt-auto">
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
</Sidebar.Root>
