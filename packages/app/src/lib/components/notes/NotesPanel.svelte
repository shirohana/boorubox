<script lang="ts">
  // Slot Sidebar · filters, below the tag list (`notes` design D14, D15): the
  // library's one scratchpad. Mounted by the sidebar rather than by the library
  // screen so the note is there on every screen a library is open on, and
  // absent on /start, where the frame itself is not drawn.
  import ChevronDownIcon from '@lucide/svelte/icons/chevron-down'
  import { library, notes, settings } from '$lib/api'
  import * as Collapsible from '$lib/components/ui/collapsible'
  import { Textarea } from '$lib/components/ui/textarea'

  // The fold is a stored preference, not local state (design D13), so the panel
  // reads it from the settings store and writes it back through the command —
  // a local copy would be a second answer to open the app on.
  const expanded = $derived(!(settings.current?.notesCollapsed ?? false))

  // The note belongs to the library, so opening another one reads its note.
  // `load` drops anything the debounce still owes — `LibraryMenu` flushes
  // before it switches, which is where that text is saved (design D14).
  //
  // The path, not the status: every capture replaces the whole status object,
  // and depending on it would re-read the note on each one — over the sentence
  // being typed.
  const path = $derived(library.status?.libraryPath ?? null)
  $effect(() => {
    void notes.load(path)
  })

  // Leaving the panel is one of the three ways to lose a pause-less edit that
  // the spec names; the window closing is the second, the library switch the
  // third.
  $effect(() => () => void notes.flush())
</script>

<!--
  Fire-and-forget: `beforeunload` cannot wait for a promise, so this posts the
  write and lets the window go. Removing it loses the last unpaused sentence of
  every session that ends by closing the window.
-->
<svelte:window onbeforeunload={() => void notes.flush()} />

<Collapsible.Root
  open={expanded}
  onOpenChange={(open) => void settings.setNotesCollapsed(!open)}
  class="flex flex-col gap-1 p-2"
>
  <Collapsible.Trigger
    class="
      flex items-center gap-1 rounded-md px-1 py-0.5 text-xs font-medium text-muted-foreground
      hover:bg-sidebar-accent
    "
  >
    <ChevronDownIcon class="size-3 transition-transform {expanded ? '' : '-rotate-90'}" />
    Notes
  </Collapsible.Trigger>

  <Collapsible.Content class="flex flex-col gap-1">
    <Textarea
      class="min-h-24 resize-y bg-sidebar text-xs"
      placeholder="Anything to remember about this library"
      aria-label="Library note"
      value={notes.content}
      oninput={(event) => notes.edit(event.currentTarget.value)}
    />
    {#if notes.error}
      <p class="px-1 text-xs text-destructive">{notes.error}</p>
    {/if}
  </Collapsible.Content>
</Collapsible.Root>
