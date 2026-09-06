<script lang="ts">
  // The one switch menu (spec `library-switching`). The sidebar footer and the
  // Library section of /settings both open it, so it takes its trigger as a
  // snippet rather than being written twice with two sets of actions that would
  // drift.
  import type { RecentLibrary } from '@boorubox/shared'
  import type { Snippet } from 'svelte'
  import FolderOpenIcon from '@lucide/svelte/icons/folder-open'
  import FolderSearchIcon from '@lucide/svelte/icons/folder-search'
  import XIcon from '@lucide/svelte/icons/x'
  import {
    closeLibrary,
    errorText,
    library,
    openLibrary,
    pickLibrary,
    recentLibraries,
    revealLibrary,
  } from '$lib/api'
  import * as DropdownMenu from '$lib/components/ui/dropdown-menu'

  interface Props {
    trigger: Snippet<[{ props: Record<string, unknown> }]>
    align?: 'start' | 'center' | 'end'
    side?: 'top' | 'right' | 'bottom' | 'left'
    onerror: (message: string) => void
  }

  let { trigger, align = 'start', side = 'top', onerror }: Props = $props()

  let open = $state(false)
  let recent = $state<RecentLibrary[]>([])

  // `data-platform` is stamped on <html> in app.html (design D13); the file
  // manager's name is the other thing that answer decides.
  const revealLabel
    = typeof document !== 'undefined' && document.documentElement.dataset.platform === 'macos'
      ? 'Reveal in Finder'
      : 'Show in Explorer'

  // Only the folders that can be opened, and never the one already open: an
  // entry that would do nothing is not an entry (spec `app-frame`). The full
  // list, with its unavailable entries and Forget, is the start screen's.
  const switchable = $derived(
    recent.filter((entry) => entry.available && entry.path !== library.status?.libraryPath),
  )

  async function loadRecent() {
    try {
      recent = await recentLibraries()
    } catch (cause) {
      onerror(errorText(cause))
    }
  }

  async function run(action: () => Promise<void>) {
    try {
      await action()
    } catch (cause) {
      onerror(errorText(cause))
    }
  }

  // Every one of these answers with the new `LibraryStatus`, so the store takes
  // it straight (no second `library_status` call), and the layout's gate is what
  // sends a close on to /start.
  const chooseFolder = () => run(async () => library.set(await pickLibrary()))
  const switchTo = (path: string) => run(async () => library.set(await openLibrary(path)))
  const close = () => run(async () => library.set(await closeLibrary()))
  const reveal = () => run(revealLibrary)
</script>

<DropdownMenu.Root
  bind:open
  onOpenChange={(opened) => {
    // Asked for when the menu opens, not held: availability is checked at call
    // time (design D4) and a folder can be unmounted while the app runs.
    if (opened) void loadRecent()
  }}
>
  <DropdownMenu.Trigger>
    {#snippet child({ props })}
      {@render trigger({ props })}
    {/snippet}
  </DropdownMenu.Trigger>

  <DropdownMenu.Content {align} {side} class="w-64">
    {#if switchable.length > 0}
      <DropdownMenu.Group>
        <DropdownMenu.GroupHeading class="text-xs font-medium text-muted-foreground">
          Switch to
        </DropdownMenu.GroupHeading>
        {#each switchable as entry (entry.path)}
          <DropdownMenu.Item onSelect={() => switchTo(entry.path)}>
            <span class="truncate" title={entry.path}>{entry.name}</span>
          </DropdownMenu.Item>
        {/each}
      </DropdownMenu.Group>
      <DropdownMenu.Separator />
    {/if}

    <DropdownMenu.Item onSelect={chooseFolder}>
      <FolderOpenIcon />
      Choose folder…
    </DropdownMenu.Item>
    <DropdownMenu.Item onSelect={reveal}>
      <FolderSearchIcon />
      {revealLabel}
    </DropdownMenu.Item>
    <DropdownMenu.Separator />
    <DropdownMenu.Item onSelect={close}>
      <XIcon />
      Close library
    </DropdownMenu.Item>
  </DropdownMenu.Content>
</DropdownMenu.Root>
