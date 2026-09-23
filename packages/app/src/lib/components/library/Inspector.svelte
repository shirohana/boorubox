<script lang="ts">
  // One component, two placements (design D9): the library route's right column
  // and the lightbox's inspect mode. Both edit — the tag editor and the rating
  // control are the same instances in both, which is the point of there being
  // one component (slots Inspector · tags and Inspector · rating, design D17).
  import type { ImageRecord, Rating, TagCount } from '@boorubox/shared'
  import type { SearchResults, Selection } from '$lib/api'
  import { tick } from 'svelte'
  import { collectionRemove, collections, errorText, selectionTagCounts, vocabulary } from '$lib/api'
  import PostedLabel from '$lib/components/booru/PostedLabel.svelte'
  import UploadAction from '$lib/components/booru/UploadAction.svelte'
  import PencilIcon from '@lucide/svelte/icons/pencil'
  import PinIcon from '@lucide/svelte/icons/pin'
  import CollectionNameDialog from '$lib/components/common/CollectionNameDialog.svelte'
  import ExternalLink from '$lib/components/common/ExternalLink.svelte'
  import RatingControl from '$lib/components/tags/RatingControl.svelte'
  import { CATEGORY_TEXT_CLASS, searchMark, searchMarkClass, SEARCH_MARK_CLASS } from '$lib/components/tags/categories'
  import TagInput from '$lib/components/tags/TagInput.svelte'
  import TagVocabularyMenuItems from '$lib/components/tags/TagVocabularyMenuItems.svelte'
  import { Badge } from '$lib/components/ui/badge'
  import { Button } from '$lib/components/ui/button'
  import * as ContextMenu from '$lib/components/ui/context-menu'
  import * as DropdownMenu from '$lib/components/ui/dropdown-menu'
  import { Input } from '$lib/components/ui/input'
  import { formatBytes, formatTimestamp } from '$lib/domain/format'
  import { groupByCategory } from '$lib/domain/tag-categories'
  import { editorText } from '$lib/domain/tag-input'
  import {
    activeTerms,
    excludeTagFromQuery,
    sortTags,
    tagList,
    toggleAccountInQuery,
    toggleCollectionInQuery,
    toggleTagInQuery,
  } from '$lib/domain/tag-utils'
  import { KEY_ENTER, KEY_ESCAPE } from '$lib/keyboard'
  import { portalTarget } from '$lib/portal'
  import type { CollectionTarget } from './collection-actions'
  import { addToCreated } from './collection-actions'
  import CollectionMenuItems from './CollectionMenuItems.svelte'
  import { fillState, type FillState, toggledSelection, toggledTag } from './pinned-state'
  import SelectionThumbs from './SelectionThumbs.svelte'
  import type { TrashActions } from './trash-actions'

  /**
   * How many of a multi-selection the header draws. One constant, no spec:
   * `selection-and-bulk` design D7 leaves the number to be judged against a real selection.
   */
  const PREVIEW_LIMIT = 12

  interface Props {
    /** The focused card. The selected image wins over it when exactly one is selected. */
    image: ImageRecord | null
    /** Where an edit is written and the changed record is put back (design D10). */
    results: SearchResults
    /**
     * Slot Inspector · header: two or more selected replaces the identity block
     * with the count and a thumbnail strip (`selection-and-bulk` design D7). One selected
     * shows that image's full panel (spec `selection`), which is not always the focused one:
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
    /**
     * A tag or account was acted on as a search term. `id` is the image this
     * panel describes — `image` below, not always the focused card (design
     * D2) — so the screen can keep it current in the new result.
     */
    onquery: (next: string, id: string) => void
    /**
     * A completed action in this panel: a rating chosen, a tag saved or
     * removed, a tag or account acted on as a search term (`app-frame` design
     * D1, amended from `onrated`). One case among many now that `browse-fixes`
     * design D4 widens the rule to any click that leaves no control focused —
     * this one still needed on its own because an action can be confirmed
     * from the keyboard, where no click ever reaches the screen's handler.
     * Each placement decides where the focus goes back to — the viewer's own
     * surface, or the grid's current card. Never fired after a failed save;
     * the editor keeps the focus so the text can be fixed.
     */
    onrelease?: () => void
    /**
     * Open the viewer at a row — the screen's own way in, so a thumbnail in the
     * selection strip opens the one viewer rather than a second one
     * (`selection-and-bulk` design D7, amended). Absent inside the viewer,
     * which is already showing an image.
     */
    onactivate?: (index: number) => void
    /**
     * A pinned chip activated over a selection (`tag-vocabulary` design D8):
     * the screen's `editSelectionTags`, which asks first past one image. The
     * one-image chip never calls this — it writes through `results.saveTags`
     * directly, the same path the tag editor's own save uses.
     */
    onedit?: (ids: string[], add: string[], remove: string[]) => void
  }

  let {
    image: focused,
    results,
    selection,
    actions,
    tagQuery,
    onquery,
    onrelease,
    onactivate,
    onedit,
  }: Props = $props()

  /**
   * The panel's own root, so every menu and dialog it mounts (design D3 of
   * `browse-fixes`) portals into the viewer's dialog when it is shown there
   * and into `<body>` beside the grid.
   */
  let root = $state<HTMLDivElement | null>(null)
  const portalTo = $derived(portalTarget(root))

  const multi = $derived(selection !== undefined && selection.count >= 2)
  const preview = $derived(
    selection && multi ? selection.previewIds(PREVIEW_LIMIT, (index) => results.at(index)?.id) : [],
  )

  /**
   * The one selected id, whichever representation the selection is in
   * (`inspector-polish` design D2).
   */
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
   * A thumbnail in the strip is a control, not a picture (`selection-and-bulk` design D7, amended):
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
  /** `sortTags` is the only tag order, applied at render (`tags-and-ratings` design D4). */
  const tags = $derived(image ? sortTags(image.tags) : [])
  /**
   * The read-mode tag list's own grouping (`tag-vocabulary` design D7),
   * ordered per `tag-panel-polish` design D1: `groupByCategory` — the same
   * grouping `editorText` lines the editor with, over the tag list instead
   * of a string.
   */
  const groupedTags = $derived(
    groupByCategory(tags, (tag) => tag, vocabulary.categoryOf).flatMap((group) => group.items),
  )
  /** Design D3: the same reader the sidebar uses, so a tag's marking agrees. */
  const terms = $derived(activeTerms(tagQuery))

  /**
   * A tag or account acted on as a search term (spec `tag-editing`): rewrites
   * the query for the image this panel describes, then hands the keyboard
   * back (`app-frame` design D1) — before the rewrite settles, because this is
   * about the keyboard, not the result (the same rule `RatingControl`'s
   * `onchosen` follows). Inside the viewer that is the whole hand-back. Beside
   * the grid the screen has already emptied the selection by the time this
   * returns, so there is no card to hand back to and the screen's own
   * `focusCard` takes it, on the row the image lands at in the new result.
   */
  function query(next: string) {
    if (!image) return
    onquery(next, image.id)
    onrelease?.()
  }

  let tagInput = $state<TagInput | null>(null)
  let draft = $state('')
  let saving = $state(false)
  let error = $state<string | null>(null)

  // Read-first (`tag-vocabulary` design D7): the tag area opens on the tag list, and the
  // field appears only once Edit is used — the facts form's own pattern.
  let editingTags = $state(false)

  // The editor follows the record: another image, or the same one after a write.
  // `updatedAt` is what a save moves, so the text comes back sorted from the row
  // that was stored rather than from what was typed (`tags-and-ratings` design D4). Gated on
  // `editingTags` so a capture arriving mid-edit does not overwrite the typed
  // text (`tag-vocabulary` design D7) — the field's own draft is seeded fresh by `startEditTags`
  // on every open instead, which reads the current tags regardless of `shown`.
  let shown = ''
  $effect(() => {
    if (editingTags) return
    const key = image ? `${image.id}:${image.updatedAt}` : ''
    if (key === shown) return
    shown = key
    draft = editorText(tags, vocabulary.categoryOf)
    error = null
  })

  function startEditTags() {
    if (!image) return
    draft = editorText(tags, vocabulary.categoryOf)
    error = null
    editingTags = true
    // The field mounts on the flag above, so it is not there yet to focus.
    void tick().then(() => tagInput?.focusEnd())
  }

  function cancelEditTags() {
    editingTags = false
    error = null
  }

  /**
   * `Escape` with the suggestion list already closed (`TagInput`'s own
   * `onescape`): the facts form's own rule, `preventDefault` and
   * `stopPropagation` for the same reason (`onFactsKeydown`'s doc comment).
   */
  function onTagsEscape(event: KeyboardEvent) {
    event.preventDefault()
    event.stopPropagation()
    cancelEditTags()
  }

  /**
   * Escape from anywhere else in the editor block — a refused save leaves
   * the focus on the Save button, and an editor that only cancels from its
   * textarea reads as stuck there. The textarea's own Escape is left to
   * `TagInput` (`onescape` above), which closes the suggestion list first.
   */
  function onEditorKeydown(event: KeyboardEvent) {
    if (event.key !== KEY_ESCAPE || event.target instanceof HTMLTextAreaElement) return
    onTagsEscape(event)
  }

  // The facts form (`editable-info` design D3): editing state, one field per row. Kept apart
  // from the tag editor's `draft` above — a facts save does not touch tags and
  // must not reset that editor's dirty text.
  let editingFacts = $state(false)
  let draftTitle = $state('')
  let draftPageUrl = $state('')
  let draftImageUrl = $state('')
  let factsSaving = $state(false)
  let factsError = $state<string | null>(null)

  // Changing the described image discards the facts draft (`editable-info` design D3): the
  // image it was for is gone from the panel. Keyed on the id alone, not
  // `updatedAt` — a save's own record replacement must not blow away the
  // "leave editing mode" that just happened, and no other write touches facts.
  let shownFactsId: string | undefined
  $effect(() => {
    const id = image?.id
    if (id === shownFactsId) return
    shownFactsId = id
    editingFacts = false
    factsError = null
  })

  function startEditFacts() {
    if (!image) return
    draftTitle = image.pageTitle ?? ''
    draftPageUrl = image.pageUrl ?? ''
    draftImageUrl = image.imageUrl ?? ''
    factsError = null
    editingFacts = true
  }

  function cancelEditFacts() {
    editingFacts = false
    factsError = null
  }

  /**
   * Saves the title and the two addresses. `onrelease` fires only on success
   * (`editable-info` design D3, the same rule `write` follows below): a refused address keeps
   * the form open with the typed text so it can be fixed.
   */
  async function saveFacts() {
    if (!image || factsSaving) return
    factsSaving = true
    factsError = null
    try {
      await results.saveFacts(image.id, {
        pageTitle: draftTitle,
        pageUrl: draftPageUrl,
        imageUrl: draftImageUrl,
      })
      editingFacts = false
      onrelease?.()
    } catch (cause) {
      factsError = errorText(cause)
    } finally {
      factsSaving = false
    }
  }

  /**
   * Enter saves, Escape cancels (`editable-info` design D3) — both `preventDefault`, since
   * inside the viewer a native `<dialog>` would otherwise close on the same
   * Escape (`Lightbox.svelte`'s `onkeydown` stands down once the key event
   * already has a default prevented). `stopPropagation` on Escape too, so nothing
   * else in the panel or the frame reads the same key press a second time.
   */
  function onFactsKeydown(event: KeyboardEvent) {
    if (event.key === KEY_ENTER) {
      event.preventDefault()
      void saveFacts()
    } else if (event.key === KEY_ESCAPE) {
      event.preventDefault()
      event.stopPropagation()
      cancelEditFacts()
    }
  }

  /**
   * A tag save or removal, from the editor, a tag's context menu or a pinned chip.
   * `onrelease` fires only on success (`inspector-polish` design D1): a failed save leaves the
   * editor open with the reason (`tag-vocabulary` design D7) rather than closing over it —
   * `submitFromEditor`'s own conditional blur below reads the same success.
   */
  async function write(tags: string[]) {
    if (!image || saving) return
    saving = true
    error = null
    try {
      await results.saveTags(image.id, tags)
      editingTags = false
      onrelease?.()
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
   * itself, from the screen's `afterWrite`.
   */
  function actAndRelease(run: (ids: string[]) => void) {
    act(run)
    selection?.reset()
  }

  async function rate(rating: Rating | null) {
    if (!image) return
    await results.saveRating(image.id, rating)
  }

  /** This image's own collections, for the "Add to…" menu's checkmark (`collections` design D8). */
  const ownCollections = $derived(image ? new Set(image.collections) : null)
  let collectionsError = $state<string | null>(null)

  async function resolveOwnId(): Promise<string[]> {
    return image ? [image.id] : []
  }

  /**
   * A collection write from either the "Add to…" menu or one badge's "Remove
   * from this collection" (`collections` design D8): the written records replace their
   * rows, the same `replace` a tag or rating save uses, and the keyboard goes
   * back the way every other panel write hands it back.
   */
  function collectionsWritten(records: ImageRecord[]): void {
    results.replaceMany(records)
    collectionsError = null
    onrelease?.()
  }

  const collectionTarget: CollectionTarget = {
    resolveIds: resolveOwnId,
    memberships: () => ownCollections,
    onwritten: collectionsWritten,
    onerror: (message) => (collectionsError = message),
  }

  /** Mounted outside the dropdown: a closed menu's content is unmounted. */
  let creatingCollection = $state(false)

  async function removeFromCollection(collectionId: string): Promise<void> {
    if (!image) return
    try {
      collectionsWritten(await collectionRemove([image.id], collectionId))
    } catch (cause) {
      collectionsError = errorText(cause)
    }
  }

  /** Design D2: the editor holds the whole set, so the whole set is sent. */
  const save = () => write(tagList(draft))

  const remove = (tag: string) => write(tags.filter((other) => other !== tag))

  /**
   * A pinned chip over one image (`tag-vocabulary` design D8): the same whole-set write the
   * editor's save makes, so the tag list, the sidebar and `updatedAt`
   * follow exactly as they do for a save.
   */
  const togglePinned = (tag: string) => write(toggledTag(tags, tag))

  /**
   * The chip's tri-state over a selection (`tag-vocabulary` design D8), read from
   * `selectionTagCounts` asked for exactly the pinned names — the bulk
   * dialog's own command, narrowed by the `names` filter it gained for this.
   *
   * Reads `selection.peekIds()`, never `selection.ids()`: `ids()` promotes a
   * range to id mode as a side effect, which reassigns the state this effect
   * tracks and reruns it mid-flight — twice the fetch per gesture — and would
   * resolve a plain select-all's ids at selection time, which is exactly what
   * `selection-and-bulk` D3 restricts to "only when an action needs ids".
   * `peekIds()` resolves the same range and does not write anything back.
   *
   * FIXME: the fetch itself is still the cost D3 warns against — resolving a
   * range through `search_ids` just to draw a chip. The honest fix is a Rust
   * `selection_tag_counts` that takes the search request plus a row range and
   * counts over the plan's rows directly, so no ids cross the wire for
   * something that is only ever drawn, not acted on, until it is clicked. Not
   * built in this pass: a Rust unit was in flight on the same files
   * (`tags.rs`, `commands.rs`).
   *
   * `pinnedCounts` is cleared, and `pinnedCountsKnown` set false, the moment
   * the effect decides to refetch — not left holding the outgoing
   * selection's answer. While unknown every chip draws `none`
   * (`fillState`'s own empty-counts fallback) and `toggleSelectionPinned`
   * ignores a click, rather than sending `add`/`remove` over images the
   * counts never described.
   *
   * `vocabulary.pinned` is a fresh array identity on every vocabulary
   * refresh even when the names themselves are unchanged, so depending on it
   * directly would refetch on every refresh; `pinnedKey` is the names'
   * stable text, and `selection.generation` — bumped by every selection
   * write — is the cheap synchronous stand-in for "the selection changed"
   * that does not require resolving anything to answer. The effect bails
   * without a round trip when none of the key, the selection's generation or
   * `results.generation` moved since the last run.
   */
  let pinnedCounts = $state<TagCount[]>([])
  let pinnedCountsError = $state<string | null>(null)
  let pinnedCountsKnown = $state(false)

  let lastPinnedKey: string | undefined
  let lastSelectionGeneration: number | undefined
  let lastResultsGeneration: number | undefined

  $effect(() => {
    const names = vocabulary.pinned
    const pinnedKey = names.join('\n')

    if (!selection || !multi || names.length === 0) {
      pinnedCounts = []
      pinnedCountsKnown = false
      return
    }

    const selectionGeneration = selection.generation
    const resultsGeneration = results.generation
    if (
      pinnedKey === lastPinnedKey
      && selectionGeneration === lastSelectionGeneration
      && resultsGeneration === lastResultsGeneration
    ) {
      return
    }
    lastPinnedKey = pinnedKey
    lastSelectionGeneration = selectionGeneration
    lastResultsGeneration = resultsGeneration

    pinnedCounts = []
    pinnedCountsKnown = false

    let current = true
    selection.peekIds()
      .then((ids) => selectionTagCounts(ids, 0, names))
      .then((counts) => {
        if (!current) return
        pinnedCounts = counts
        pinnedCountsKnown = true
        pinnedCountsError = null
      })
      .catch((cause) => {
        if (current) pinnedCountsError = errorText(cause)
      })
    return () => {
      current = false
    }
  })

  /**
   * A pinned chip's activation over a selection (`tag-vocabulary` design D8): add unless every
   * selected image already carries the tag, else remove it from all of them —
   * `onedit` is the screen's `editSelectionTags`, which asks first past one
   * image (`pending-write.ts`'s `edit` kind). Ignored while `pinnedCounts` is
   * unknown (the fetch above is mid-flight or has not started): a click that
   * lands then would otherwise act on counts left over from a different
   * selection.
   */
  async function toggleSelectionPinned(tag: string) {
    if (!selection || !pinnedCountsKnown) return
    const { add, remove: removeTag } = toggledSelection(
      fillState(tag, pinnedCounts, selection.count),
      tag,
    )
    onedit?.(await selection.ids(), add, removeTag)
  }

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

<!--
  A pinned tag's chip (`tag-vocabulary` design D8), one image's membership or a selection's
  tri-state alike — `state` is `'all' | 'some' | 'none'` either way, `'some'`
  only ever reached from a selection. Always drawn `secondary`: `default`'s
  `bg-primary` is near-white in dark mode, and `CATEGORY_TEXT_CLASS`'s
  amber/violet/red/green/blue text loses its contrast against it. The fill
  state is the pin itself: solid (`fill-current`) while the image carries
  the tag, an outline while it does not, half-solid over a selection that
  is split — a ring alone was unreadable at 1× (smoke run, 2026-09-24). A
  faint foreground tint and a ring (`all`) or a dashed outline (`some`,
  Tailwind's `ring-*` utilities have no dashed style, so `some` uses
  `outline-*` instead) back the pin up without competing with the category
  colour, which stays the text's alone on the muted background either way.
  `aria-pressed` carries the same tri-state for assistive tech, since a
  plain boolean cannot say "some". The pin glyph (`tag-vocabulary` design
  D6, `tag-panel-polish` design D6) is what marks this a control rather than
  one of the image's tags, now that the image's own tags are plain text
  (design D3) and this chip is the only pill-shaped thing left in the
  section. Its menu is `TagVocabularyMenuItems` unchanged: a pinned chip's
  tag reads `vocabulary.isPinned` true by construction, so only Unpin
  renders, never a second Pin item to suppress.
-->
{#snippet pinnedChip(tag: string, state: FillState, onactivate: () => void)}
  <li>
    <ContextMenu.Root>
      <ContextMenu.Trigger>
        {#snippet child({ props })}
          <button
            type="button"
            {...props}
            aria-pressed={state === 'all' ? 'true' : state === 'some' ? 'mixed' : 'false'}
            onclick={onactivate}
          >
            <Badge
              variant="secondary"
              class="
                flex items-center gap-1
                {state === 'all' ? 'bg-foreground/10 ring-1 ring-foreground/60' : ''}
                {state === 'some' ? 'outline-1 outline-foreground/40 outline-dashed' : ''}
                {CATEGORY_TEXT_CLASS[vocabulary.categoryOf(tag)]}
              "
            >
              <PinIcon
                class="
                  size-3 shrink-0
                  {state === 'all' ? 'fill-current' : ''}
                  {state === 'some' ? 'fill-current [fill-opacity:0.4]' : ''}
                "
              />
              {tag}
            </Badge>
          </button>
        {/snippet}
      </ContextMenu.Trigger>
      <ContextMenu.Content portalProps={{ to: portalTo }}>
        <TagVocabularyMenuItems name={tag} />
      </ContextMenu.Content>
    </ContextMenu.Root>
  </li>
{/snippet}

<div bind:this={root} class="flex h-full flex-col overflow-y-auto">
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

    <!--
      Design D8: the same chip row the single-image panel draws, tri-state
      over the selection instead of one image's membership. Absent along with
      its heading while nothing is pinned, same as the single-image panel.
    -->
    {#if vocabulary.pinned.length > 0}
      <section class="border-t border-border px-4 py-3">
        <h3 class="mb-2 text-xs font-medium text-muted-foreground">Tags</h3>
        <ul class="flex flex-wrap gap-1">
          {#each vocabulary.pinned as tag (tag)}
            {@render pinnedChip(
              tag,
              fillState(tag, pinnedCounts, selection?.count ?? 0),
              () => void toggleSelectionPinned(tag),
            )}
          {/each}
        </ul>
        {#if pinnedCountsError}
          <p class="mt-1 text-xs text-destructive">{pinnedCountsError}</p>
        {/if}
        {#if vocabulary.error}
          <p class="mt-1 text-xs text-destructive">{vocabulary.error}</p>
        {/if}
      </section>
    {/if}
  {:else if !image}
    <p class="p-4 text-sm text-muted-foreground">No image selected</p>
  {:else}
    <header class="flex items-start justify-between gap-2 border-b border-border px-4 py-3">
      <h2 class="text-sm font-medium wrap-break-word">{title}</h2>
      {#if !editingFacts}
        <Button
          type="button"
          size="icon-xs"
          variant="ghost"
          class="shrink-0"
          aria-label="Edit title and addresses"
          title="Edit title and addresses"
          onclick={startEditFacts}
        >
          <PencilIcon class="size-3" />
        </Button>
      {/if}
    </header>

    <dl class="grid grid-cols-[auto_minmax(0,1fr)] gap-x-3 gap-y-1.5 px-4 py-3 text-xs">
      <!--
        The header falls back to the image URL and then the id so it is never
        blank; this row is the stored page title itself, which is often absent.
      -->
      <dt class="text-muted-foreground">Title</dt>
      {#if editingFacts}
        <dd>
          <Input
            bind:value={draftTitle}
            class="h-6 px-1.5 text-xs"
            aria-label="Title"
            onkeydown={onFactsKeydown}
          />
        </dd>
      {:else}
        <dd class="wrap-break-word">{image.pageTitle ?? '—'}</dd>
      {/if}

      <dt class="text-muted-foreground">Source</dt>
      <dd class="wrap-break-word">{origin}</dd>

      {#if image.account}
        {@const account = image.account}
        <!--
          Spec `tag-editing`, "An X account on screen is a search term": a
          fact of the page address, beside Page, not among the tags — with
          artist tags in the vocabulary a handle there read as a second
          artist (design D4). Read the same in both `editingFacts` states:
          the address being edited is the draft, but the account is derived
          from the stored one, so this row does not flip with the form. No
          colour of its own (blue is general's now, design D2) — the padded,
          hoverable box and the pointer cursor are what say it is a control,
          not a fact to merely read (owner, 2026-09-24: the row did not look
          clickable). `searchMarkClass` (design D3, amended again
          2026-09-24) supplies `hover:bg-accent` only while the row carries
          no tint, so hovering an active account keeps its mark instead of
          losing it to a competing neutral hover.
        -->
        <dt class="text-muted-foreground">Account</dt>
        <dd>
          <button
            type="button"
            class="
              cursor-pointer rounded-md px-1 py-0.5 text-foreground
              {searchMarkClass(searchMark(account, terms.accounts, terms.excludedAccounts), 'hover:bg-accent')}
            "
            title="Search for this account"
            onclick={() => query(toggleAccountInQuery(tagQuery, account))}
          >
            @{account}
          </button>
        </dd>
      {/if}

      <dt class="text-muted-foreground">Page</dt>
      {#if editingFacts}
        <dd>
          <Input
            bind:value={draftPageUrl}
            class="h-6 px-1.5 text-xs"
            aria-label="Page address"
            placeholder="https://…"
            onkeydown={onFactsKeydown}
          />
        </dd>
      {:else}
        <!-- Wrapping, so `ExternalLink`'s failure line gets a row of its own. -->
        <dd class="flex flex-wrap items-start gap-1 wrap-break-word">
          <span class="min-w-0 flex-1 wrap-break-word">{image.pageUrl ?? '—'}</span>
          <ExternalLink url={image.pageUrl} />
        </dd>
      {/if}

      <dt class="text-muted-foreground">Image</dt>
      {#if editingFacts}
        <dd>
          <Input
            bind:value={draftImageUrl}
            class="h-6 px-1.5 text-xs"
            aria-label="Image address"
            placeholder="https://…"
            onkeydown={onFactsKeydown}
          />
        </dd>
      {:else}
        <dd class="flex flex-wrap items-start gap-1 wrap-break-word">
          <span class="min-w-0 flex-1 wrap-break-word">{image.imageUrl ?? '—'}</span>
          <ExternalLink url={image.imageUrl} />
        </dd>
      {/if}

      {#if editingFacts}
        <div class="col-span-2 flex items-center justify-end gap-2 pt-0.5">
          <Button size="xs" variant="outline" disabled={factsSaving} onclick={cancelEditFacts}>
            Cancel
          </Button>
          <Button size="xs" disabled={factsSaving} onclick={saveFacts}>Save</Button>
        </div>
        {#if factsError}
          <p class="col-span-2 text-destructive">{factsError}</p>
        {/if}
      {/if}

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
      <RatingControl value={image.rating} onchoose={rate} onchosen={onrelease} />
    </section>

    <section class="border-t border-border px-4 py-3">
      <div class="flex items-center justify-between gap-2">
        <h3 class="text-xs font-medium text-muted-foreground">
          Tags {#if tags.length > 0}({tags.length}){/if}
        </h3>
        {#if !editingTags}
          <Button
            type="button"
            size="icon-xs"
            variant="ghost"
            class="shrink-0"
            aria-label="Edit tags"
            title="Edit tags"
            onclick={startEditTags}
          >
            <PencilIcon class="size-3" />
          </Button>
        {/if}
      </div>

      {#if editingTags}
        <!--
          A textarea, capped so a heavily tagged image does not push the rest
          of the panel off screen; past the cap it scrolls. Save and Cancel
          sit under it, not beside: beside a field that grows they would hang
          in the margin (`tag-vocabulary` design D7, the facts form's own layout).
        -->
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div class="mt-2 flex flex-col gap-2" onkeydown={onEditorKeydown}>
          <TagInput
            bind:this={tagInput}
            bind:value={draft}
            multiline
            label="Tags of this image"
            placeholder="Tags, separated by spaces"
            class="max-h-64 min-h-16 min-w-0"
            onsubmit={submitFromEditor}
            onescape={onTagsEscape}
          />
          <div class="flex justify-end gap-2">
            <Button size="xs" variant="outline" disabled={saving} onclick={cancelEditTags}>
              Cancel
            </Button>
            <Button size="xs" disabled={saving} onclick={save}>Save</Button>
          </div>
        </div>
      {:else if vocabulary.pinned.length > 0}
        <!--
          Design D8: one activation writes the whole toggled set through
          `write`, the same path the editor's own save makes — so the tag
          list, the sidebar and `updatedAt` follow exactly as they do for a
          save. Only in read mode: a chip write mid-edit would fight the
          open draft, replacing tags the field has not saved yet.
        -->
        <ul class="mt-2 flex flex-wrap gap-1">
          {#each vocabulary.pinned as tag (tag)}
            {@render pinnedChip(tag, tags.includes(tag) ? 'all' : 'none', () => togglePinned(tag))}
          {/each}
        </ul>
      {/if}

      {#if error}
        <p class="mt-2 text-xs text-destructive">{error}</p>
      {/if}
      {#if vocabulary.error}
        <p class="mt-2 text-xs text-destructive">{vocabulary.error}</p>
      {/if}

      {#if !editingTags}
        {#if tags.length === 0}
          <p class="mt-2 text-xs text-muted-foreground">No tags</p>
        {:else}
          <!--
            Every tag on screen is a search term (spec `tag-editing`): a click
            puts it in the query or takes it out, and the menu offers the
            other two things one can do to a tag, plus the vocabulary group
            (`tag-vocabulary` design D9). Plain text, not a pill (design D3):
            a pill's own fill competed with the marking that says a tag is in
            the search. `rounded-md px-1 py-0.5` give the text a real box, so
            `SEARCH_MARK_CLASS`'s background (design D3, amended
            `tag-panel-polish` 2026-09-24 — no underline, no strike-through)
            has something to tint beside `CATEGORY_TEXT_CLASS`'s category
            colour, rather than replacing it. `searchMarkClass` (amended
            again 2026-09-24) supplies `hover:bg-accent` only for a tag with
            no mark, so an active or excluded tag keeps its own hover instead
            of losing it to a competing neutral one. `cursor-pointer`
            (`Button`'s own default, Tailwind 4 preflight otherwise leaves a
            `<button>` at `cursor: default`): every clickable text in the
            panel admits it. Grouped by category (`tag-vocabulary` design
            D7): `groupedTags` above.
          -->
          <ul class="mt-2 flex flex-wrap gap-x-2">
            {#each groupedTags as tag (tag)}
              <li>
                <ContextMenu.Root>
                  <ContextMenu.Trigger>
                    {#snippet child({ props })}
                      <button
                        type="button"
                        {...props}
                        class="
                          cursor-pointer rounded-md px-1 py-0.5 text-xs
                          {CATEGORY_TEXT_CLASS[vocabulary.categoryOf(tag)]}
                          {searchMarkClass(searchMark(tag, terms.included, terms.excluded), 'hover:bg-accent')}
                        "
                        onclick={() => query(toggleTagInQuery(tagQuery, tag))}
                      >
                        {tag}
                      </button>
                    {/snippet}
                  </ContextMenu.Trigger>
                  <ContextMenu.Content portalProps={{ to: portalTo }}>
                    <ContextMenu.Item onSelect={() => query(toggleTagInQuery(tagQuery, tag))}>
                      Search for this tag
                    </ContextMenu.Item>
                    <ContextMenu.Item onSelect={() => query(excludeTagFromQuery(tagQuery, tag))}>
                      Exclude from the search
                    </ContextMenu.Item>
                    <ContextMenu.Separator />
                    <ContextMenu.Item variant="destructive" onSelect={() => remove(tag)}>
                      Remove from this image
                    </ContextMenu.Item>
                    <ContextMenu.Separator />
                    <TagVocabularyMenuItems name={tag} />
                  </ContextMenu.Content>
                </ContextMenu.Root>
              </li>
            {/each}
          </ul>
        {/if}
      {/if}
    </section>

    <!--
      `collections` design D8: absent when the image is in none and there is
      nothing to add to — impossible while Favorites exists; the guard is for
      a library whose user deleted every collection.
    -->
    {#if image.collections.length > 0 || collections.list.length > 0}
      <section class="border-t border-border px-4 py-3">
        <h3 class="mb-2 text-xs font-medium text-muted-foreground">
          Collections {#if image.collections.length > 0}({image.collections.length}){/if}
        </h3>

        {#if image.collections.length > 0}
          <!--
            Each acts as a search term with the one marking style every
            search-term reader uses (design D3: `SEARCH_MARK_CLASS` over
            `searchMark`'s result), so a collection reads the same "in the
            search" cue as a tag or the account row rather than its own
            inline colours — `terms` is `activeTerms`'s own reader, shared
            with the sidebar section.
          -->
          <ul class="flex flex-wrap gap-1">
            {#each image.collections as collectionId (collectionId)}
              {@const collection = collections.byId(collectionId)}
              {#if collection}
                <li>
                  <ContextMenu.Root>
                    <ContextMenu.Trigger>
                      {#snippet child({ props })}
                        {@const collectionSlug = collection.slug}
                        <button
                          type="button"
                          {...props}
                          onclick={() => query(toggleCollectionInQuery(tagQuery, collectionSlug))}
                        >
                          <Badge
                            variant="secondary"
                            class={SEARCH_MARK_CLASS[
                              searchMark(
                                collectionSlug,
                                terms.collections,
                                terms.excludedCollections,
                              )
                            ]}
                          >
                            {collection.name}
                          </Badge>
                        </button>
                      {/snippet}
                    </ContextMenu.Trigger>
                    <ContextMenu.Content portalProps={{ to: portalTo }}>
                      <ContextMenu.Item onSelect={() => void removeFromCollection(collection.id)}>
                        Remove from this collection
                      </ContextMenu.Item>
                    </ContextMenu.Content>
                  </ContextMenu.Root>
                </li>
              {/if}
            {/each}
          </ul>
        {:else}
          <p class="text-xs text-muted-foreground">In no collections</p>
        {/if}

        <DropdownMenu.Root>
          <DropdownMenu.Trigger>
            {#snippet child({ props })}
              <Button size="xs" variant="outline" class="mt-2" {...props}>Add to…</Button>
            {/snippet}
          </DropdownMenu.Trigger>
          <DropdownMenu.Content
            align="start"
            portalProps={{ to: portalTo }}
            class="max-h-(--bits-floating-available-height) overflow-y-auto"
          >
            <CollectionMenuItems
              target={collectionTarget}
              onnew={() => (creatingCollection = true)}
            >
              {#snippet checkboxItem({ label, checked, onSelect })}
                <DropdownMenu.CheckboxItem {checked} onCheckedChange={onSelect}>
                  {label}
                </DropdownMenu.CheckboxItem>
              {/snippet}
              {#snippet item({ label, onSelect })}
                <DropdownMenu.Item {onSelect}>{label}</DropdownMenu.Item>
              {/snippet}
              {#snippet separator()}
                <DropdownMenu.Separator />
              {/snippet}
            </CollectionMenuItems>
          </DropdownMenu.Content>
        </DropdownMenu.Root>

        {#if collectionsError}
          <p class="mt-2 text-xs text-destructive">{collectionsError}</p>
        {/if}
      </section>
    {/if}

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
          {portalTo}
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

<!--
  Outside the menu above — see `collection-actions.ts` for why a dialog cannot
  live inside one, and `collections` design D8 for why creating here also adds.
-->
<CollectionNameDialog
  collection={null}
  open={creatingCollection}
  {portalTo}
  onclose={() => (creatingCollection = false)}
  onsaved={(collection) => {
    creatingCollection = false
    void addToCreated(collectionTarget, collection)
  }}
/>
