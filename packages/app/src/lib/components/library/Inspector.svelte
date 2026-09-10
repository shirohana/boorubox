<script lang="ts">
  // One component, two placements (design D9): the library route's right column
  // and the lightbox's inspect mode. Both edit — the tag editor and the rating
  // control are the same instances in both, which is the point of there being
  // one component (slots Inspector · tags and Inspector · rating, design D17).
  import type { ImageRecord, Rating } from '@boorubox/shared'
  import type { SearchResults, Selection } from '$lib/api'
  import { errorText } from '$lib/api'
  import PostedLabel from '$lib/components/booru/PostedLabel.svelte'
  import UploadAction from '$lib/components/booru/UploadAction.svelte'
  import RatingControl from '$lib/components/tags/RatingControl.svelte'
  import TagInput from '$lib/components/tags/TagInput.svelte'
  import { Badge } from '$lib/components/ui/badge'
  import { Button } from '$lib/components/ui/button'
  import * as ContextMenu from '$lib/components/ui/context-menu'
  import { formatBytes, formatTimestamp } from '$lib/domain/format'
  import { excludeTagFromQuery, sortTags, tagList, toggleTagInQuery } from '$lib/domain/tag-utils'
  import SelectionThumbs from './SelectionThumbs.svelte'
  import type { TrashActions } from './trash-actions'

  /**
   * How many of a multi-selection the header draws. One constant, no spec:
   * design D7 leaves the number to be judged against a real selection.
   */
  const PREVIEW_LIMIT = 12

  interface Props {
    /** The focused card. The selected image wins over it when exactly one is selected. */
    image: ImageRecord | null
    /** Where an edit is written and the changed record is put back (design D10). */
    results: SearchResults
    /**
     * Slot Inspector · header: two or more selected replaces the identity block
     * with the count and a thumbnail strip (design D7). One selected shows that
     * image's full panel (spec `selection`), which is not always the focused one:
     * a multi-select click that deselects leaves the focus on the card it cleared.
     *
     * Absent inside the viewer, which shows one image and neither reads nor
     * writes the selection (`selection-and-bulk` Non-Goals).
     */
    selection?: Selection
    /**
     * Slot Inspector · actions (`app-shell` D9): the pair the tile menu offers,
     * for the one image this panel is showing, in both of its placements.
     */
    actions: TrashActions
    /** The toolbar's tag query, so a click on a tag can rewrite it (design D14). */
    tagQuery: string
    onquery: (next: string) => void
    /**
     * A rating was chosen here. Only the viewer's placement listens: it takes
     * the keyboard focus back off the choice so Space still closes it (design
     * D4, amended). Beside the grid nobody wants the focus moved.
     */
    onrated?: () => void
    /**
     * Open the viewer at a row — the screen's own way in, so a thumbnail in the
     * selection strip opens the one viewer rather than a second one
     * (`selection-and-bulk` design D7, amended). Absent inside the viewer,
     * which is already showing an image.
     */
    onactivate?: (index: number) => void
  }

  let {
    image: focused,
    results,
    selection,
    actions,
    tagQuery,
    onquery,
    onrated,
    onactivate,
  }: Props = $props()

  const multi = $derived(selection !== undefined && selection.count >= 2)
  const preview = $derived(
    selection && multi ? selection.previewIds(PREVIEW_LIMIT, (index) => results.at(index)?.id) : [],
  )

  /** The one selected id, whichever representation the selection is in (design D2). */
  const only = $derived(
    selection?.count === 1
      ? selection.previewIds(1, (index) => results.at(index)?.id)[0]
      : undefined,
  )
  const image = $derived(
    only === undefined || only === focused?.id ? focused : results.at(rowWithId(only)) ?? null,
  )

  /**
   * `SearchResults` answers by row, so a record is found by the same walk its
   * own `replace` does; `-1` is a row whose page has not been loaded. It stops
   * at the selected row, and that row is one whose tile was drawn — the walk is
   * short in the gestures that reach here.
   *
   * The row and not the record, because both callers want the index: the panel
   * to read the record back, the strip to open the viewer at it.
   */
  function rowWithId(id: string): number {
    for (let index = 0; index < results.total; index++) {
      if (results.at(index)?.id === id) return index
    }
    return -1
  }

  /**
   * A thumbnail in the strip is a control, not a picture (design D7, amended):
   * pressing it opens the image, and its own button takes it back out of the
   * selection. The row is looked up here because the strip has ids and the
   * viewer opens at a row.
   */
  function openThumb(id: string) {
    const index = rowWithId(id)
    if (index >= 0) onactivate?.(index)
  }

  const title = $derived(image?.pageTitle || image?.imageUrl || image?.id || '')
  const origin = $derived(
    image ? (image.sourceRef ? `${image.source} · ${image.sourceRef}` : image.source) : '',
  )
  /** `sortTags` is the only tag order, applied at render (design D4). */
  const tags = $derived(image ? sortTags(image.tags) : [])
  const saved = $derived(tags.join(' '))

  let tagInput = $state<TagInput | null>(null)
  let draft = $state('')
  let saving = $state(false)
  let error = $state<string | null>(null)
  const dirty = $derived(draft.trim() !== saved)

  // The editor follows the record: another image, or the same one after a write.
  // `updatedAt` is what a save moves, so the text comes back sorted from the row
  // that was stored rather than from what was typed (design D4).
  let shown = ''
  $effect(() => {
    const key = image ? `${image.id}:${image.updatedAt}` : ''
    if (key === shown) return
    shown = key
    draft = saved
    error = null
  })

  async function write(tags: string[]) {
    if (!image || saving) return
    saving = true
    error = null
    try {
      await results.saveTags(image.id, tags)
    } catch (cause) {
      error = errorText(cause)
    } finally {
      saving = false
    }
  }

  /** The action row acts on the image on screen, whatever the grid's focus is. */
  function act(run: (ids: string[]) => void) {
    if (!image) return
    run([image.id])
  }

  /**
   * A write that happens now also lets its image go: the panel was describing
   * one image and the user has just taken it out of this view, so there is
   * nothing left here to describe. The grid's own `Delete` keeps its card
   * instead (`trash` design D12) — those keys are meant to be pressed again.
   *
   * Only for the writes that happen now. "Delete forever…" merely opens the
   * confirmation, and emptying the panel while it is up moves the screen under
   * a question the user may still decline; its reset comes with the write
   * itself, from the screen's `afterTrashWrite`.
   */
  function actAndRelease(run: (ids: string[]) => void) {
    act(run)
    selection?.reset()
  }

  async function rate(rating: Rating | null) {
    if (!image) return
    await results.saveRating(image.id, rating)
  }

  /** Design D2: the editor holds the whole set, so the whole set is sent. */
  const save = () => write(tagList(draft))

  const remove = (tag: string) => write(tags.filter((other) => other !== tag))

  /**
   * A keyboard confirmation in the editor saves and then, once the write has
   * gone through, blurs it so the grid's keyboard map is live again. The blur is
   * conditional on success: a failed save keeps the focus so the text can be
   * fixed and confirmed again without reaching for the field.
   */
  async function submitFromEditor() {
    await save()
    if (!error) tagInput?.blur()
  }
</script>

<div class="flex h-full flex-col overflow-y-auto">
  {#if multi}
    <header class="border-b border-border px-4 py-3">
      <h2 class="text-sm font-medium">{selection?.count.toLocaleString()} images selected</h2>
    </header>

    <section class="px-4 py-3">
      <SelectionThumbs
        ids={preview}
        more={(selection?.count ?? 0) - preview.length}
        onopen={openThumb}
        onremove={(id) => void selection?.remove(id)}
      />
    </section>
  {:else if !image}
    <p class="p-4 text-sm text-muted-foreground">No image selected</p>
  {:else}
    <header class="border-b border-border px-4 py-3">
      <h2 class="text-sm font-medium wrap-break-word">{title}</h2>
    </header>

    <dl class="grid grid-cols-[auto_minmax(0,1fr)] gap-x-3 gap-y-1.5 px-4 py-3 text-xs">
      <!--
        The header falls back to the image URL and then the id so it is never
        blank; this row is the stored page title itself, which is often absent.
      -->
      <dt class="text-muted-foreground">Title</dt>
      <dd class="wrap-break-word">{image.pageTitle ?? '—'}</dd>

      <dt class="text-muted-foreground">Source</dt>
      <dd class="wrap-break-word">{origin}</dd>

      <dt class="text-muted-foreground">Page</dt>
      <dd class="wrap-break-word">{image.pageUrl ?? '—'}</dd>

      <dt class="text-muted-foreground">Image</dt>
      <dd class="wrap-break-word">{image.imageUrl ?? '—'}</dd>

      <dt class="text-muted-foreground">Dimensions</dt>
      <dd>{image.width} × {image.height}</dd>

      <dt class="text-muted-foreground">Size</dt>
      <dd>{formatBytes(image.size)}</dd>

      <dt class="text-muted-foreground">Type</dt>
      <dd>{image.mime}</dd>

      <dt class="text-muted-foreground">Captured</dt>
      <dd>{formatTimestamp(image.capturedAt)}</dd>

      <dt class="text-muted-foreground">Imported</dt>
      <dd>{formatTimestamp(image.createdAt)}</dd>

      <!--
        `formatTimestamp` already reads a non-finite number as `—`; passing
        `NaN` for an image with no file behind it reuses that fallback instead
        of a second one written here (design D11).
      -->
      <dt class="text-muted-foreground">File modified</dt>
      <dd>{formatTimestamp(image.fileModifiedAt ?? Number.NaN)}</dd>

      <dt class="text-muted-foreground">ID</dt>
      <dd class="font-mono wrap-break-word">{image.id}</dd>
    </dl>

    <section class="border-t border-border px-4 py-3">
      <h3 class="mb-2 text-xs font-medium text-muted-foreground">Rating</h3>
      <RatingControl value={image.rating} onchoose={rate} onchosen={onrated} />
    </section>

    <section class="border-t border-border px-4 py-3">
      <h3 class="mb-2 text-xs font-medium text-muted-foreground">
        Tags {#if tags.length > 0}({tags.length}){/if}
      </h3>

      <div class="flex items-center gap-2">
        <TagInput
          bind:this={tagInput}
          bind:value={draft}
          label="Tags of this image"
          placeholder="Tags, separated by spaces"
          class="h-8 min-w-0 flex-1"
          onsubmit={submitFromEditor}
        />
        {#if dirty}
          <Button size="xs" disabled={saving} onclick={save}>Save</Button>
        {/if}
      </div>

      {#if error}
        <p class="mt-2 text-xs text-destructive">{error}</p>
      {/if}

      {#if tags.length === 0}
        <p class="mt-2 text-xs text-muted-foreground">No tags</p>
      {:else}
        <!--
          Every tag on screen is a search term (spec `tag-editing`): a click puts
          it in the query or takes it out, and the menu offers the other two
          things one can do to a tag.
        -->
        <ul class="mt-2 flex flex-wrap gap-1">
          {#each tags as tag (tag)}
            <li>
              <ContextMenu.Root>
                <ContextMenu.Trigger>
                  {#snippet child({ props })}
                    <button
                      type="button"
                      {...props}
                      onclick={() => onquery(toggleTagInQuery(tagQuery, tag))}
                    >
                      <Badge variant="secondary">{tag}</Badge>
                    </button>
                  {/snippet}
                </ContextMenu.Trigger>
                <ContextMenu.Content>
                  <ContextMenu.Item onSelect={() => onquery(toggleTagInQuery(tagQuery, tag))}>
                    Search for this tag
                  </ContextMenu.Item>
                  <ContextMenu.Item onSelect={() => onquery(excludeTagFromQuery(tagQuery, tag))}>
                    Exclude from the search
                  </ContextMenu.Item>
                  <ContextMenu.Separator />
                  <ContextMenu.Item variant="destructive" onSelect={() => remove(tag)}>
                    Remove from this image
                  </ContextMenu.Item>
                </ContextMenu.Content>
              </ContextMenu.Root>
            </li>
          {/each}
        </ul>
      {/if}
    </section>

    <!--
      `posted-label`: absent, not an empty placeholder, for an image that has
      never been posted. Shown in the trash too — where an image has been is a
      fact about it, not an action on it.
    -->
    {#if image.posts.length > 0}
      <section class="border-t border-border px-4 py-3">
        <h3 class="mb-2 text-xs font-medium text-muted-foreground">Posted</h3>
        <PostedLabel posts={image.posts} />
      </section>
    {/if}

    <!--
      Slot Inspector · actions (`trash` design D13, `booru-upload` design D9).
      `mt-auto` keeps it at the foot of the panel rather than floating under a
      short tag list, and it is inside this branch, so a panel showing no image
      has no action row at all.
    -->
    <section class="mt-auto flex flex-col gap-2 border-t border-border px-4 py-3">
      <!--
        Not in the trash: an image on its way out of the library is not one to
        publish, and the row it would write claims the library holds the file.
      -->
      {#if results.view === 'library'}
        <!--
          The post is the only thing that changed about the image, and Rust
          answered with it, so the loaded record is edited where it sits —
          `tags-and-ratings` design D10: a write replaces the record, the search
          is never re-run. Re-running it here would clear the rows under the
          dialog that is still reporting the post it just made.
        -->
        <UploadAction
          {image}
          onposted={(post) => results.replace({ ...image, posts: [...image.posts, post] })}
        />
      {/if}

      <div class="flex gap-2">
        {#if results.view === 'trash'}
          <Button size="xs" variant="outline" onclick={() => actAndRelease(actions.restore)}>
            Restore
          </Button>
          <Button size="xs" variant="destructive" onclick={() => act(actions.deleteForever)}>
            Delete forever…
          </Button>
        {:else}
          <Button size="xs" variant="outline" onclick={() => actAndRelease(actions.trash)}>
            Move to trash
          </Button>
        {/if}
      </div>
    </section>
  {/if}
</div>
