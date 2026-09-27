<script lang="ts">
  // Slot Settings · Artists (`artist-entries` design D7): this file draws the
  // list and the delete confirm and mounts the form; the form itself is
  // `ArtistForm.svelte`, the two fields shared between Add and Edit
  // (`artists_upsert`'s own refusals decide what a valid entry is either
  // way) — unlike Rules' three-way split, there is no separate list
  // component here, since the list is only a handful of rows drawn inline.
  import type { ArtistApplyPreview, ArtistApplyReport, ArtistEntry } from '@boorubox/shared'
  import PencilIcon from '@lucide/svelte/icons/pencil'
  import PlayIcon from '@lucide/svelte/icons/play'
  import PlusIcon from '@lucide/svelte/icons/plus'
  import Trash2Icon from '@lucide/svelte/icons/trash-2'
  import {
    artistRevision,
    artistsApply,
    artistsApplyPreview,
    artistsDelete,
    artistsList,
    errorText,
    library,
    vocabulary,
  } from '$lib/api'
  import ConfirmDialog from '$lib/components/common/ConfirmDialog.svelte'
  import { CATEGORY_TEXT_CLASS } from '$lib/components/tags/categories'
  import { Button } from '$lib/components/ui/button'
  import { Input } from '$lib/components/ui/input'
  import { filterArtists } from './artist-filter'
  import ArtistForm from './ArtistForm.svelte'

  let entries = $state<ArtistEntry[]>([])
  /** Whether the form is open, and on which entry — `null` is a new one. */
  let formOpen = $state(false)
  let editing = $state<ArtistEntry | null>(null)
  let confirming = $state<ArtistEntry | null>(null)
  let error = $state<string | null>(null)

  /**
   * Apply, per entry (`artist-workflow` design D7).
   * `confirmingApply` is only set once `artistsApplyPreview` has answered —
   * the confirmation's own count is what the button reads, so there is
   * nothing to confirm before it is known. `applyPending` names which
   * entry's preview is in flight, so a second click on the same row while it
   * loads is a no-op rather than a second fetch. `applyResult` is shown
   * under the entry it answers for and cleared the next time that entry's
   * Apply is chosen.
   */
  let applyPending = $state<string | null>(null)
  let confirmingApply = $state<{ entry: ArtistEntry, preview: ArtistApplyPreview } | null>(null)
  /**
   * What the confirmation draws: the last `confirmingApply`, kept after it is
   * cleared so the dialog's close animation still shows the entry and counts
   * it was opened for rather than "undefined" and zeros.
   */
  let applyShown = $state<{ entry: ArtistEntry, preview: ArtistApplyPreview } | null>(null)
  const applyImages = $derived(applyShown?.preview.images ?? 0)
  const applyUntagged = $derived(applyShown?.preview.untagged ?? 0)
  const applyDescription = $derived(
    [
      `${applyImages.toLocaleString()} ${applyImages === 1 ? 'image comes' : 'images come'}`,
      `from these URLs; ${applyUntagged.toLocaleString()} of ${applyImages === 1 ? 'it' : 'them'}`,
      `${applyUntagged === 1 ? 'carries' : 'carry'} no artist tag.`,
      `Applying adds “${applyShown?.entry.tag ?? ''}” to`,
      `${applyImages === 1 ? 'it' : `all ${applyImages.toLocaleString()}`}; no tag is removed.`,
    ].join(' '),
  )
  let applyResult = $state<{ tag: string, report: ArtistApplyReport } | null>(null)
  let applyError = $state<string | null>(null)
  // Not remembered and not cleared by a save (`settings-pages` design D8): the field is the
  // user's own filter at work, not view state worth persisting.
  let query = $state('')
  const shown = $derived(filterArtists(entries, query))

  // Re-read when the open library changes: `RulesSection`'s own reasoning —
  // the switch menu on this screen can swap the library out from under the
  // list, and entries belong to the library. The path, not the status: a
  // capture replaces the whole status object, and depending on it would
  // re-read the whole list on every one of them.
  const libraryPath = $derived(library.status?.libraryPath ?? null)
  $effect(() => {
    void libraryPath
    void load()
  })

  async function load() {
    try {
      entries = await artistsList()
      error = null
    } catch (cause) {
      error = errorText(cause)
    }
  }

  function deleteConfirmed() {
    const doomed = confirming
    confirming = null
    if (!doomed) return
    void (async () => {
      try {
        entries = await artistsDelete(doomed.tag)
        artistRevision.bump()
        error = null
      } catch (cause) {
        error = errorText(cause)
      }
    })()
  }

  async function startApply(entry: ArtistEntry) {
    if (applyPending) return
    applyError = null
    applyResult = null
    applyPending = entry.tag
    try {
      const preview = await artistsApplyPreview(entry.urls)
      confirmingApply = { entry, preview }
      applyShown = confirmingApply
    } catch (cause) {
      applyError = errorText(cause)
    } finally {
      applyPending = null
    }
  }

  function applyConfirmed() {
    const target = confirmingApply
    confirmingApply = null
    if (!target) return
    void (async () => {
      try {
        const report = await artistsApply(target.entry.tag)
        artistRevision.bump()
        applyResult = { tag: target.entry.tag, report }
        await vocabulary.refresh()
      } catch (cause) {
        applyError = errorText(cause)
      }
    })()
  }
</script>

<section class="flex flex-col gap-4">
  <h2 class="text-sm font-semibold">Artists</h2>

  <p class="text-sm text-muted-foreground">
    An artist entry is a tag that owns a list of profile URLs, matched against a capture's
    source so the right name is used from then on. Changing URLs applies to future captures
    only. To tag images already in the library, apply the artist; applying only adds its tag.
    To rename an artist and retag its images, use Edit artist… on its tag.
  </p>

  {#if formOpen}
    <ArtistForm
      entry={editing}
      onsaved={(saved) => {
        entries = saved
        artistRevision.bump()
        formOpen = false
        editing = null
      }}
      oncancel={() => {
        formOpen = false
        editing = null
      }}
    />
  {:else}
    <div>
      <Button size="sm" onclick={() => (formOpen = true)}>
        <PlusIcon />
        Add artist
      </Button>
    </div>
  {/if}

  {#if entries.length > 0}
    <Input
      type="search"
      placeholder="Filter by tag or URL"
      aria-label="Filter artists"
      bind:value={query}
    />
  {/if}

  {#if entries.length === 0}
    <p class="text-sm text-muted-foreground">
      No artist entries yet. Editing or creating an artist from its tag elsewhere in the app
      creates one too.
    </p>
  {:else if shown.length === 0}
    <p class="text-sm text-muted-foreground">No artist matches “{query}”.</p>
  {:else}
    <ul class="flex flex-col gap-2">
      {#each shown as entry (entry.tag)}
        <li class="rounded-lg border border-border p-3">
          <div class="flex flex-wrap items-center gap-2">
            <span
              class="min-w-0 text-sm font-medium wrap-break-word {CATEGORY_TEXT_CLASS.artist}"
            >
              {entry.tag}
            </span>
            <div class="ml-auto flex items-center gap-1">
              <Button
                size="icon-xs"
                variant="ghost"
                aria-label="Apply {entry.tag} to stored images"
                title="Apply to stored images"
                disabled={applyPending === entry.tag}
                onclick={() => void startApply(entry)}
              >
                <PlayIcon />
              </Button>
              <Button
                size="icon-xs"
                variant="ghost"
                aria-label="Edit {entry.tag}"
                onclick={() => {
                  editing = entry
                  formOpen = true
                }}
              >
                <PencilIcon />
              </Button>
              <Button
                size="icon-xs"
                variant="ghost"
                aria-label="Delete {entry.tag}"
                onclick={() => (confirming = entry)}
              >
                <Trash2Icon />
              </Button>
            </div>
          </div>

          <!--
            The note between the tag and its URLs (`tag-notes` design D9): the
            store is refreshed by the frame on library open (`frame/Sidebar.svelte`),
            so it is live here too, edited from the tag's own context menu, not
            from this list (`tag-notes` spec `artist-entries`).
          -->
          {#if vocabulary.noteOf(entry.tag)}
            <p class="mt-1 text-xs wrap-break-word whitespace-pre-wrap text-muted-foreground">
              {vocabulary.noteOf(entry.tag)}
            </p>
          {/if}

          <ul class="mt-2 flex flex-col gap-0.5">
            {#each entry.urls as url (url)}
              <li class="font-mono text-xs break-all text-muted-foreground">{url}</li>
            {/each}
          </ul>

          {#if applyResult?.tag === entry.tag}
            <p class="mt-1 text-xs text-muted-foreground">
              Tagged {applyResult.report.tagged.toLocaleString()}
              {applyResult.report.tagged === 1 ? 'image' : 'images'};
              {applyResult.report.skipped.toLocaleString()} already carried it.
            </p>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}

  {#if applyError}
    <p class="text-sm text-destructive">{applyError}</p>
  {/if}

  {#if error}
    <p class="text-sm text-destructive">{error}</p>
  {/if}
</section>

<!--
  An entry is not recoverable once deleted (spec "Deleting"), so this asks —
  the seventh caller of `ConfirmDialog` (its own doc comment names the bar for
  adding one). It clears that bar the way the trash and rebuild confirmations
  do, on its own ground: naming what deleting does not do (no image changes)
  against what it does silently change (the next matching capture).
-->
<ConfirmDialog
  title="Delete “{confirming?.tag}”?"
  description="Future captures from these URLs fall back to the handle or display name; no
    image changes."
  confirmLabel="Delete"
  open={confirming !== null}
  onclose={() => (confirming = null)}
  onconfirm={deleteConfirmed}
/>

<!--
  The eighth caller (`artist-workflow` design D7, the doc comment above names
  the bar): not destructive, but a bulk write over however many images the
  entry owns, made from one button — the same ground the rating write clears
  it on. `destructive={false}` for the same reason the rating write's own
  call is: nothing here is destroyed. `confirmDisabled` at 0 images: there is
  nothing that button would do.
-->
<ConfirmDialog
  title="Apply “{applyShown?.entry.tag}”?"
  description={applyDescription}
  confirmLabel="Apply to {applyImages.toLocaleString()} {applyImages === 1 ? 'image' : 'images'}"
  destructive={false}
  confirmDisabled={applyImages === 0}
  open={confirmingApply !== null}
  onclose={() => (confirmingApply = null)}
  onconfirm={applyConfirmed}
/>
