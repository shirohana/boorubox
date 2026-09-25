<script lang="ts">
  // Slot Settings · Artists (`artist-entries` design D7): above Rules — an
  // entry is configuration of the library applied as an image enters it, the
  // same reasoning `RulesSection`'s own placement gives, not a route of its
  // own. This file draws the list and the delete confirm and mounts the form;
  // the form itself is `ArtistForm.svelte`, the two fields shared between
  // Add and Edit (`artists_upsert`'s own refusals decide what a valid entry
  // is either way) — unlike Rules' three-way split, there is no separate list
  // component here, since the list is only a handful of rows drawn inline.
  import type { ArtistEntry } from '@boorubox/shared'
  import PencilIcon from '@lucide/svelte/icons/pencil'
  import PlusIcon from '@lucide/svelte/icons/plus'
  import Trash2Icon from '@lucide/svelte/icons/trash-2'
  import { artistsDelete, artistsList, errorText, library } from '$lib/api'
  import ConfirmDialog from '$lib/components/common/ConfirmDialog.svelte'
  import { CATEGORY_TEXT_CLASS } from '$lib/components/tags/categories'
  import { Button } from '$lib/components/ui/button'
  import ArtistForm from './ArtistForm.svelte'

  let entries = $state<ArtistEntry[]>([])
  /** Whether the form is open, and on which entry — `null` is a new one. */
  let formOpen = $state(false)
  let editing = $state<ArtistEntry | null>(null)
  let confirming = $state<ArtistEntry | null>(null)
  let error = $state<string | null>(null)

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
        error = null
      } catch (cause) {
        error = errorText(cause)
      }
    })()
  }
</script>

<section class="flex flex-col gap-4">
  <h2 class="text-sm font-semibold">Artists</h2>

  <p class="text-sm text-muted-foreground">
    An artist entry is a tag that owns a list of profile URLs, matched against a capture's
    source so the right name is used from then on. Changing URLs here applies to future
    captures only — images already in the library keep their tags. To retag images, rename the
    artist from its tag in the image's inspector.
  </p>

  <!--
    FIXME(`artist-entries` D9): correcting an entry's URLs here does not
    re-derive any image already stored — only a rename from the inspector
    retags, because that is the case the proposal covers today. A "run over
    existing images" pass, the shape `auto-tag-rules` design D9's `rules.rs`
    `run` already has, is the missing way back for an entry corrected here
    instead.
  -->

  {#if formOpen}
    <ArtistForm
      entry={editing}
      onsaved={(saved) => {
        entries = saved
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

  {#if entries.length === 0}
    <p class="text-sm text-muted-foreground">
      No artist entries yet. Renaming an artist from its tag in the inspector creates one too.
    </p>
  {:else}
    <ul class="flex flex-col gap-2">
      {#each entries as entry (entry.tag)}
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

          <ul class="mt-2 flex flex-col gap-0.5">
            {#each entry.urls as url (url)}
              <li class="font-mono text-xs break-all text-muted-foreground">{url}</li>
            {/each}
          </ul>
        </li>
      {/each}
    </ul>
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
