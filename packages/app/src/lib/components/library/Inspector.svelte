<script lang="ts">
  // One component, two placements (design D9): the library route's right column
  // and the lightbox's inspect mode. Both edit — the tag editor and the rating
  // control are the same instances in both, which is the point of there being
  // one component (slots Inspector · tags and Inspector · rating, design D17).
  import type { ArtistMatch, Collection, CollectionCount, ImageRecord, Rating, TagCount, TagEditSpec } from '@boorubox/shared'
  import type { SearchResults, Selection } from '$lib/api'
  import { tick, untrack } from 'svelte'
  import {
    artistMatch as fetchArtistMatch,
    artistRevision,
    collectionRemove,
    collections,
    errorText,
    latestOnly,
    rowOf,
    selectionCollectionCounts,
    selectionTagCounts,
    vocabulary,
  } from '$lib/api'
  import type { ArtistDialogRequest } from '$lib/components/artists/artist-dialog'
  import ArtistDialog from '$lib/components/artists/ArtistDialog.svelte'
  import PostedLabel from '$lib/components/booru/PostedLabel.svelte'
  import UploadAction from '$lib/components/booru/UploadAction.svelte'
  import BookmarkIcon from '@lucide/svelte/icons/bookmark'
  import PencilIcon from '@lucide/svelte/icons/pencil'
  import PinIcon from '@lucide/svelte/icons/pin'
  import CollectionNameDialog from '$lib/components/common/CollectionNameDialog.svelte'
  import ExternalLink from '$lib/components/common/ExternalLink.svelte'
  import RatingControl from '$lib/components/tags/RatingControl.svelte'
  import { CATEGORY_TEXT_CLASS, searchMark, searchMarkClass, SEARCH_MARK_CLASS } from '$lib/components/tags/categories'
  import TagInput from '$lib/components/tags/TagInput.svelte'
  import CollectionPinMenuItem from '$lib/components/tags/CollectionPinMenuItem.svelte'
  import PinnedGroupsDialog from '$lib/components/tags/PinnedGroupsDialog.svelte'
  import TagNoteDialog from '$lib/components/tags/TagNoteDialog.svelte'
  import PinnedDot from '$lib/components/tags/PinnedDot.svelte'
  import TagNoteIndicator from '$lib/components/tags/TagNoteIndicator.svelte'
  import TagVocabularyMenuItems from '$lib/components/tags/TagVocabularyMenuItems.svelte'
  import { Badge } from '$lib/components/ui/badge'
  import { Button } from '$lib/components/ui/button'
  import * as ContextMenu from '$lib/components/ui/context-menu'
  import * as DropdownMenu from '$lib/components/ui/dropdown-menu'
  import { Input } from '$lib/components/ui/input'
  import * as Tooltip from '$lib/components/ui/tooltip'
  import { formatRelative, formatSizeLine, formatTimestamp } from '$lib/domain/format'
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
  import { addToCreated, toggleCollection } from './collection-actions'
  import CollectionMenuItems from './CollectionMenuItems.svelte'
  import { isOrphanedFocus } from './focus-handback'
  import { fillOf, fillState, type FillState, toggledSelection, toggledTag } from './pinned-state'
  import SelectionThumbs from './SelectionThumbs.svelte'
  import type { TrashActions } from './trash-actions'

  /**
   * How many of a multi-selection the header draws. One constant, no spec:
   * `selection-and-bulk` design D7 leaves the number to be judged against a real selection.
   */
  const PREVIEW_LIMIT = 12

  /**
   * What {@link pinnedChip} draws (`pinned-collections` design D7): a tag by
   * name, or a collection by its row. Named here rather than written inline
   * at the snippet's parameter, which `eslint`'s template-expression parser
   * does not accept for an inline object-literal union type.
   */
  type PinnedChipItem = { kind: 'tag', tag: string } | { kind: 'collection', collection: Collection }

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
    /**
     * `id` is the described image, for the screen to keep current across the
     * re-run; `undefined` over a selection with no focused card (a ⌘A after a
     * fresh search), where the pinned chips' search items still have to work.
     */
    onquery: (next: string, id: string | undefined) => void
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
     * A pinned chip activated over a selection (`pinned-collections` design
     * D10, widened from `tag-vocabulary` design D8's `(ids, add, remove)`):
     * the screen's `editSelection`, which asks first past one image. `label`
     * is the chip's own name — the tag, or the collection's name, which the
     * confirmation names since the spec only carries its slug. The one-image
     * chip never calls this — it writes through `results.saveTags` or
     * `collection-actions.ts`'s `toggleCollection` directly, the same paths
     * the tag editor's own save and the "Add to…" menu use.
     */
    onedit?: (ids: string[], spec: TagEditSpec, label: string) => void
    /**
     * An artist entry was saved — a rename, a create, or an edit's own
     * "Apply to existing images" (`artist-workflow` design D4): usually
     * retags other images too, so the screen's own write pattern runs,
     * `afterWrite` in `LibraryScreen.svelte` — vocabulary and search refresh,
     * then `keepMatching()` prunes the selection and refocuses, since a save
     * can move a row out of the current search, the same reason every other
     * selection-wide writer ends there. Required in both placements: inside
     * the viewer too the re-read has to keep the viewer on its image, which a
     * bare `results.refresh()` does not (`selection-by-id` design D7).
     */
    onartistsaved: () => void
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
    onartistsaved,
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
    only === undefined || only === focused?.id ? focused : results.at(rowOf(results, only)) ?? null,
  )

  /**
   * A thumbnail in the strip is a control, not a picture (`selection-and-bulk` design D7, amended):
   * pressing it opens the image, and its own button takes it back out of the
   * selection. The row is looked up here because the strip has ids and the
   * viewer opens at a row.
   */
  function openThumb(id: string) {
    const index = rowOf(results, id)
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
    onquery(next, image?.id)
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

  export function startEditTags() {
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

  /**
   * Both editors belong to the image they were opened for (spec `tag-editing`,
   * `app-frame`): changing the described image closes the tag editor and the
   * facts form and discards their drafts, since a save from either would
   * otherwise land on the image that replaced it (`tag-row-and-inspector-fixes`
   * design D6). Keyed on `imageId` alone, not `updatedAt` — a write to the
   * same image (this editor's own save, a rating, a facts save, a capture
   * refreshing the record) must not close an editor that is open for it. The
   * first run after mount only records the id: `e` opens the tag editor
   * through a `tick()` right after the panel mounts, and a first run that
   * reset would race it closed.
   *
   * An editor closed here while it held the focus hands it back through
   * `onrelease`: in the viewer `→` can change the image with the focus on the
   * editor's Cancel, and the unmounted button drops the focus to `<body>`,
   * where the viewer's own `keydown` never hears the next `←`/`→`. Not when
   * the panel loses its image: there is no image to hand the focus back to.
   */
  const imageId = $derived(image?.id)
  let lastImageId: string | undefined
  let sawFirstImageId = false
  $effect(() => {
    if (!sawFirstImageId) {
      sawFirstImageId = true
      lastImageId = imageId
      return
    }
    if (imageId === lastImageId) return
    lastImageId = imageId
    const editorHeldFocus = untrack(
      () => (editingTags || editingFacts) && isOrphanedFocus(document.activeElement, root),
    )
    editingTags = false
    error = null
    editingFacts = false
    factsError = null
    if (imageId !== undefined && editorHeldFocus) onrelease?.()
  })

  /**
   * The Artist row (`artist-workflow` design D5): who the library says the
   * image's author is, read once per image rather than carried on
   * `ImageRecord` — a per-row field would read the entries for every image
   * of every search page to draw a row only the inspector shows.
   *
   * `inspectedId` reuses {@link imageId} rather than a second `image?.id`
   * derivation, and the effect below is keyed on it and on
   * {@link artistEntriesRevision}, not on `image` or `adapter` directly:
   * `image` is reassigned wholesale on every refresh (CLAUDE.md's `$effect`
   * rule), so an effect that read `image.adapter` as a tracked dependency
   * would refetch on a capture landing mid-view even though the id on screen
   * has not moved. `adapter` and `pageUrl` are read through `untrack` inside
   * the effect for the same reason. The page URL goes along because an image
   * stored with no record — a legacy-bundle import — is matched by the X
   * account its page names, the account the Account row below shows.
   */
  const inspectedId = $derived(imageId ?? null)
  const inspectedAdapter = $derived(image?.adapter ?? null)
  const inspectedPageUrl = $derived(image?.pageUrl ?? null)

  /**
   * Moves on every artist entry write, from any host — this panel's dialog,
   * the sidebar's, Settings → Artists — so the row re-reads for the same image.
   */
  const artistEntriesRevision = $derived(artistRevision.current)
  let artistMatchAnswer = $state<ArtistMatch | null>(null)
  let artistMatchError = $state<string | null>(null)
  const artistMatchTicket = latestOnly<ArtistMatch | null>()

  $effect(() => {
    const id = inspectedId
    void artistEntriesRevision
    // Cleared before the fetch starts, not left holding the previous image's
    // answer: design D5's "the row is absent rather than a placeholder"
    // while pending reads the same as "no profile URL at all" — both hide
    // the row until an answer names one.
    artistMatchAnswer = null
    artistMatchError = null
    if (id === null) return
    const adapter = untrack(() => inspectedAdapter)
    const pageUrl = untrack(() => inspectedPageUrl)
    if (!adapter && pageUrl === null) return
    void (async () => {
      try {
        const result = await artistMatchTicket(fetchArtistMatch(adapter, pageUrl))
        if (result.current) {
          artistMatchAnswer = result.value
          artistMatchError = null
        }
      } catch (cause) {
        if (id === inspectedId) artistMatchError = errorText(cause)
      }
    })()
  })

  function startEditFacts() {
    if (!image) return
    draftTitle = image.pageTitle ?? ''
    draftPageUrl = image.pageUrl ?? ''
    draftImageUrl = image.imageUrl ?? ''
    factsError = null
    editingFacts = true
    // The form sits below Tags and Collections, out of view under a long tag
    // list, while its pencil is in the header: focusing the first field is
    // what scrolls the form to where the click's effect can be seen.
    void tick().then(() => root?.querySelector<HTMLInputElement>('input[aria-label="Title"]')?.focus())
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

  /**
   * The `ArtistDialog` request this panel opened, or `null` while none is
   * open (`artist-workflow` design D2, D4). A snapshot taken when the menu item or
   * "Create artist…" is chosen, not a guard on the tag plus the live `image`
   * above: a capture landing while the dialog is up runs `results.refresh()`,
   * which empties the results for a moment and turns `image` briefly `null`,
   * and reading the live `image` instead would change what the open dialog
   * reads out from under a save in progress. `ArtistDialog` itself stays
   * mounted (the `CollectionNameDialog` shape) and follows this via
   * `open`/`request` props, so it is only this snapshot's own reassignment,
   * not `results.refresh()`, that ever changes what it sees.
   */
  let editingArtist = $state<ArtistDialogRequest | null>(null)

  /**
   * The tag `TagNoteDialog` is open for — the name, snapshotted when "Edit
   * note…" is chosen (`tag-notes` design D10), one dialog here for the
   * chips of both strips and the badges, the `editingArtist` shape above:
   * the dialog cannot live inside `TagVocabularyMenuItems` (its content
   * unmounts on select).
   */
  let editingNote = $state<string | null>(null)

  /** The pinned-groups dialog is open; one instance serves every menu of both strips. */
  let managingGroups = $state(false)

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
   * The chips' tri-state over a selection, both kinds (`pinned-collections`
   * design D9, widened from `tag-vocabulary` design D8): one effect fetches
   * `selectionTagCounts` for the pinned names and `selectionCollectionCounts`
   * for the pinned collection ids together, over one `selection.peekIds()` —
   * not two effects, each with its own copy of the generation guard and the
   * {@link pinnedFetch} ticket, where a click could land with one set of
   * counts fresh and the other stale.
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
   * count that takes the search request plus a row range and counts over the
   * plan's rows directly, for both the tag and the collection call, so no ids
   * cross the wire for something that is only ever drawn, not acted on, until
   * it is clicked. Not built in this pass: a Rust unit was in flight on the
   * same files (`tags.rs`, `collections.rs`, `commands.rs`).
   *
   * The counts are cleared, and `pinnedCountsKnown` set false, the moment the
   * effect decides to refetch — not left holding the outgoing selection's
   * answer. While unknown every chip draws `none` (`fillState`'s and
   * `fillOf`'s own empty-counts fallback) and a chip's activation ignores a
   * click, rather than sending an edit over images the counts never
   * described.
   *
   * `vocabulary.pinned` and `collections.pinned` are fresh array identities on
   * every refresh even when the names or ids themselves are unchanged, so
   * depending on them directly would refetch on every refresh; `pinnedKey` is
   * their stable text together, and `selection.generation` — bumped by every
   * selection write — is the cheap synchronous stand-in for "the selection
   * changed" that does not require resolving anything to answer. The effect
   * bails without a round trip when none of the key, the selection's
   * generation or `results.generation` moved since the last run, and bails
   * out entirely while nothing of either kind is pinned.
   */
  let pinnedTagCounts = $state<TagCount[]>([])
  let pinnedCollectionCounts = $state<CollectionCount[]>([])
  let pinnedCountsError = $state<string | null>(null)
  let pinnedCountsKnown = $state(false)

  /**
   * Which fetch may still answer: bumped by every fetch the effect starts and
   * by every run that clears the counts, never by a run that bails out
   * unchanged. Not the effect's own teardown — that runs before every re-run,
   * the unchanged ones included, so a vocabulary refresh landing mid-fetch (a
   * new `vocabulary.pinned` identity over the same names) would drop the one
   * answer still owed and leave `pinnedCountsKnown` false, every chip `none`
   * and every click ignored, until the selection next changed.
   */
  let pinnedFetch = 0
  let lastPinnedKey: string | undefined
  let lastSelectionGeneration: number | undefined
  let lastResultsGeneration: number | undefined

  $effect(() => {
    const names = vocabulary.pinned
    const collectionIds = collections.pinned.map((collection) => collection.id)
    // Sorted: the counts depend on which names are pinned, not on their
    // order, and a move between groups reorders `pinned` without changing
    // the set — keyed on the order, every move refetched and blanked the chips.
    const pinnedKey = `${[...names].sort().join('\n')}\u0000${collectionIds.join('\n')}`

    if (!selection || !multi || (names.length === 0 && collectionIds.length === 0)) {
      pinnedFetch++
      pinnedTagCounts = []
      pinnedCollectionCounts = []
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

    pinnedTagCounts = []
    pinnedCollectionCounts = []
    pinnedCountsKnown = false

    const ticket = ++pinnedFetch
    void (async () => {
      try {
        const ids = await selection.peekIds()
        const tagCountsFetch: Promise<TagCount[]> = names.length > 0
          ? selectionTagCounts(ids, 0, names)
          : Promise.resolve([])
        const collectionCountsFetch: Promise<CollectionCount[]> = collectionIds.length > 0
          ? selectionCollectionCounts(ids, collectionIds)
          : Promise.resolve([])
        const [tagCounts, collectionCounts] = await Promise.all([
          tagCountsFetch,
          collectionCountsFetch,
        ])
        if (ticket !== pinnedFetch) return
        pinnedTagCounts = tagCounts
        pinnedCollectionCounts = collectionCounts
        pinnedCountsKnown = true
        pinnedCountsError = null
      } catch (cause) {
        if (ticket === pinnedFetch) pinnedCountsError = errorText(cause)
      }
    })()
  })

  /**
   * A tag chip's activation over a selection (`tag-vocabulary` design D8):
   * add unless every selected image already carries the tag, else remove it
   * from all of them — `onedit` is the screen's `editSelection`, which asks
   * first past one image (`pending-write.ts`'s `edit` kind). Ignored while
   * `pinnedCountsKnown` is false (the fetch above is mid-flight or has not
   * started): a click that lands then would otherwise act on counts left
   * over from a different selection.
   */
  async function toggleSelectionTag(tag: string) {
    if (!selection || !pinnedCountsKnown) return
    const { add, remove: removeTag } = toggledSelection(
      fillState(tag, pinnedTagCounts, selection.count),
      tag,
    )
    const spec: TagEditSpec = { add, remove: removeTag, addCollections: [], removeCollections: [] }
    onedit?.(await selection.ids(), spec, tag)
  }

  /**
   * How many of the selection are in `id`, from `pinnedCollectionCounts` —
   * a collection absent from the answer is 0 of them, `selectionCollectionCounts`'s
   * own doc comment. Read by the template too, so the chip's fill and its
   * own activation never disagree about the count a click acted on.
   */
  function collectionCountOf(id: string): number {
    return pinnedCollectionCounts.find((entry) => entry.id === id)?.count ?? 0
  }

  /**
   * A collection chip's activation over a selection (`pinned-collections`
   * design D10), the collection-keyed twin of {@link toggleSelectionTag}: the
   * slug, not the id, since `addCollections`/`removeCollections` are slugs
   * resolved to ids in Rust (`TagEditSpec`'s own doc comment).
   */
  async function toggleSelectionCollection(collection: Collection) {
    if (!selection || !pinnedCountsKnown) return
    const { add, remove: removeSlug } = toggledSelection(
      fillOf(collectionCountOf(collection.id), selection.count),
      collection.slug,
    )
    const spec: TagEditSpec = {
      add: [],
      remove: [],
      addCollections: add,
      removeCollections: removeSlug,
    }
    onedit?.(await selection.ids(), spec, collection.name)
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
  The two things one can do to a tag as a search term (spec `tag-editing`),
  shared by the tag list's own menu and a pinned chip's menu (design D3): a
  pinned chip is a tag on screen like any other, so it gets the same two
  items rather than a copy of them.
-->
{#snippet tagSearchItems(tag: string)}
  <ContextMenu.Item onSelect={() => query(toggleTagInQuery(tagQuery, tag))}>
    Search for this tag
  </ContextMenu.Item>
  <ContextMenu.Item onSelect={() => query(excludeTagFromQuery(tagQuery, tag))}>
    Exclude from the search
  </ContextMenu.Item>
{/snippet}

<!--
  A pinned chip, tag or collection alike (`pinned-collections` design D7,
  `tag-vocabulary` design D8): one image's membership or a selection's
  tri-state — `state` is `'all' | 'some' | 'none'` either way, `'some'`
  only ever reached from a selection. Always drawn `secondary`: `default`'s
  `bg-primary` is near-white in dark mode, and the text classes below lose
  their contrast against it. The fill state is the glyph itself: solid
  (`fill-current`) while the image carries the tag or is in the collection,
  an outline while it does not, half-solid over a selection that is split —
  a ring alone was unreadable at 1× (smoke run, 2026-09-23). A faint
  foreground tint and a ring (`all`) or a dashed outline (`some`, Tailwind's
  `ring-*` utilities have no dashed style, so `some` uses `outline-*`
  instead) back the glyph up without competing with the text colour. A tag's
  text is `CATEGORY_TEXT_CLASS[vocabulary.categoryOf(tag)]` with `PinIcon`,
  the glyph that marks this a control rather than one of the image's tags,
  now that the image's own tags are plain text (design D3) and this chip is
  the only pill-shaped thing left in the section. A collection's text is
  plain `text-foreground` with `BookmarkIcon` — the icon `ImageCard` already
  draws for "in a collection" — since every category hue is already a tag's
  (design D7's "why neutral text and not a sixth hue"). `aria-pressed`
  carries the same tri-state for assistive tech, since a plain boolean
  cannot say "some". The menu is `TagVocabularyMenuItems` for a tag —
  unchanged: a pinned chip's tag reads `vocabulary.isPinned` true by
  construction, so Unpin and the group moves render, never a Pin item to suppress —
  or `CollectionPinMenuItem` for a collection, true by construction the same
  way.
-->
{#snippet pinnedTagRows(fill: (tag: string) => FillState, onactivate: (tag: string) => void)}
  <!--
    One row per group, a hairline between rows from the second on. A group
    is called by its name, or `#n` when it has none (`vocabulary.labelOf`,
    the spelling "Move to" uses), and the label is the fold toggle: past one
    group, or for a named one, it ends the row; a single unnamed group has
    nothing to tell apart or to fold, so it draws no label. A folded row is
    its label and `· <count>` and no chips.
    The row is the wrap container and the label its last item (`order-last`,
    `ml-auto`, `self-end`): it takes the end of the chips' last line when
    there is room and a line of its own only when there is not, so no width is
    reserved for a name whose length nobody knows. The chips' `<ul>` is
    `contents` for that reason, its chips being the row's own flex items, and
    the label is a sibling of it, not a child — a `<button>` directly inside
    a `<ul>` is invalid, and Preflight's `list-style: none` makes WebKit drop
    the list role, taking the `aria-label` with it, hence the explicit
    `role="list"`. Shared by both strips so a category or group change cannot
    leave one placement's rows out of step with the other's.
  -->
  {#each vocabulary.pinnedGroups as group, index (index)}
    {@const position = index + 1}
    {@const label = vocabulary.labelOf(position)}
    {@const labelled = group.name !== '' || vocabulary.pinnedGroups.length > 1}
    <div class="flex flex-wrap gap-1 {index > 0 ? 'mt-1.5 border-t border-border pt-1.5' : ''}">
      {#if !group.collapsed}
        <ul
          role="list"
          class="contents"
          aria-label={labelled ? `Pinned group ${label}` : undefined}
        >
          {#each group.tags as tag (tag)}
            {@render pinnedChip({ kind: 'tag', tag }, fill(tag), () => onactivate(tag))}
          {/each}
        </ul>
      {/if}
      {#if labelled}
        <button
          type="button"
          aria-expanded={!group.collapsed}
          title={label}
          class="
            order-last ml-auto max-w-32 cursor-pointer self-end truncate text-[10px]
            text-muted-foreground tabular-nums
            hover:text-foreground
          "
          onclick={() => void vocabulary.setGroupCollapsed(position, !group.collapsed)}
        >
          {group.collapsed ? `${label} · ${group.tags.length}` : label}
        </button>
      {/if}
    </div>
  {/each}
{/snippet}

{#snippet pinnedChip(chip: PinnedChipItem, state: FillState, onactivate: () => void)}
  {@const Glyph = chip.kind === 'tag' ? PinIcon : BookmarkIcon}
  {@const label = chip.kind === 'tag' ? chip.tag : chip.collection.name}
  {@const textClass = chip.kind === 'tag'
    ? CATEGORY_TEXT_CLASS[vocabulary.categoryOf(chip.tag)]
    : 'text-foreground'}
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
                {textClass}
              "
            >
              <Glyph
                class="
                  size-3 shrink-0
                  {state === 'all' ? 'fill-current' : ''}
                  {state === 'some' ? 'fill-current [fill-opacity:0.4]' : ''}
                "
              />
              {label}
              {#if chip.kind === 'tag'}
                <TagNoteIndicator note={vocabulary.noteOf(chip.tag)} {portalTo} />
              {/if}
            </Badge>
          </button>
        {/snippet}
      </ContextMenu.Trigger>
      <ContextMenu.Content portalProps={{ to: portalTo }}>
        {#if chip.kind === 'tag'}
          {@render tagSearchItems(chip.tag)}
          <ContextMenu.Separator />
          <TagVocabularyMenuItems
            name={chip.tag}
            oneditnote={(name) => (editingNote = name)}
            onmanagegroups={() => (managingGroups = true)}
            oneditartist={(tag) => (editingArtist = { mode: 'edit', tag, adapter: image?.adapter ?? null, pageUrl: image?.pageUrl ?? null })}
          />
        {:else}
          <CollectionPinMenuItem collection={chip.collection} />
        {/if}
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
      The single-image panel's pinned tag strip, tri-state over the selection
      instead of one image's tags. Absent along with its heading while no tag
      is pinned.
    -->
    {#if vocabulary.pinnedGroups.length > 0}
      <section class="border-t border-border px-4 py-3">
        <h3 class="mb-2 text-xs font-medium text-muted-foreground">Tags</h3>
        {@render pinnedTagRows(
          (tag) => fillState(tag, pinnedTagCounts, selection?.count ?? 0),
          (tag) => void toggleSelectionTag(tag),
        )}
        {#if pinnedCountsError}
          <p class="mt-1 text-xs text-destructive">{pinnedCountsError}</p>
        {/if}
        {#if vocabulary.error}
          <p class="mt-1 text-xs text-destructive">{vocabulary.error}</p>
        {/if}
      </section>
    {/if}

    <!--
      The pinned collection chips, tri-state over the selection
      (`pinned-collections` design D7/D9).
      Absent along with its heading while no collection is pinned.
      `pinnedCountsError` is one error for both kinds' counts (D9), so
      it is shown here only when the Tags section above, which already shows
      it, is absent.
    -->
    {#if collections.pinned.length > 0}
      <section class="border-t border-border px-4 py-3">
        <h3 class="mb-2 text-xs font-medium text-muted-foreground">Collections</h3>
        <ul class="flex flex-wrap gap-1">
          {#each collections.pinned as collection (collection.id)}
            {@render pinnedChip(
              { kind: 'collection', collection },
              fillOf(collectionCountOf(collection.id), selection?.count ?? 0),
              () => void toggleSelectionCollection(collection),
            )}
          {/each}
        </ul>
        {#if pinnedCountsError && vocabulary.pinnedGroups.length === 0}
          <p class="mt-1 text-xs text-destructive">{pinnedCountsError}</p>
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

    <!-- No top border: the header above already draws the line between them. -->
    <section class="px-4 py-3">
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
      {:else if vocabulary.pinnedGroups.length > 0}
        <!--
          `tag-vocabulary` design D8: a chip's activation writes the whole
          toggled set through `write`, the same path the editor's own save
          makes, so the tag list, the sidebar and `updatedAt` follow exactly
          as they do for a save. Only in read mode: a chip write mid-edit
          would fight the open draft, replacing tags the field has not saved
          yet.
        -->
        <div class="mt-2">
          {@render pinnedTagRows(
            (tag) => (tags.includes(tag) ? 'all' : 'none'),
            (tag) => togglePinned(tag),
          )}
        </div>
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
            `tag-panel-polish` 2026-09-23 — no underline, no strike-through)
            has something to tint beside `CATEGORY_TEXT_CLASS`'s category
            colour, rather than replacing it. `searchMarkClass` (amended
            again 2026-09-23) supplies `hover:bg-accent` only for a tag with
            no mark, so an active or excluded tag keeps its own hover instead
            of losing it to a competing neutral one. `cursor-pointer`
            (`Button`'s own default, Tailwind 4 preflight otherwise leaves a
            `<button>` at `cursor: default`): every clickable text in the
            panel admits it. Grouped by category (`tag-vocabulary` design
            D7): `groupedTags` above. The badge stays one line beside its note
            mark, but a name wider than the panel breaks inside the box
            (`max-w-full`, the name's own `break-all`) rather than pushing the
            panel sideways; the name needs `whitespace-normal`, since the
            button's `nowrap` would otherwise suppress `break-all`.
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
                          inline-flex max-w-full cursor-pointer items-center gap-1 rounded-md px-1
                          py-0.5 text-xs whitespace-nowrap
                          {CATEGORY_TEXT_CLASS[vocabulary.categoryOf(tag)]}
                          {searchMarkClass(searchMark(tag, terms.included, terms.excluded), 'hover:bg-accent')}
                        "
                        onclick={() => query(toggleTagInQuery(tagQuery, tag))}
                      >
                        <span class="min-w-0 break-all whitespace-normal">{tag}</span>
                        <PinnedDot pinned={vocabulary.isPinned(tag)} />
                        <TagNoteIndicator note={vocabulary.noteOf(tag)} {portalTo} />
                      </button>
                    {/snippet}
                  </ContextMenu.Trigger>
                  <ContextMenu.Content portalProps={{ to: portalTo }}>
                    {@render tagSearchItems(tag)}
                    <ContextMenu.Separator />
                    <TagVocabularyMenuItems
                      name={tag}
                      oneditnote={(name) => (editingNote = name)}
                      onmanagegroups={() => (managingGroups = true)}
                      oneditartist={(name) => (editingArtist = { mode: 'edit', tag: name, adapter: image?.adapter ?? null, pageUrl: image?.pageUrl ?? null })}
                    />
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

        {#if collections.pinned.length > 0 && !editingTags}
          <!--
            `pinned-collections` design D7: a chip's activation goes through
            `toggleCollection` on the panel's own `collectionTarget`, the
            "Add to…" menu's door. Hidden while the tag editor is open:
            `collectionTarget.onwritten` calls `onrelease`, which on the grid
            refocuses the grid and would blur the editor mid-draft.
          -->
          <ul class="mb-2 flex flex-wrap gap-1">
            {#each collections.pinned as collection (collection.id)}
              {@render pinnedChip(
                { kind: 'collection', collection },
                ownCollections?.has(collection.id) ? 'all' : 'none',
                () => void toggleCollection(collectionTarget, collection.id),
              )}
            {/each}
          </ul>
        {/if}

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
                      <ContextMenu.Separator />
                      <CollectionPinMenuItem {collection} />
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
        part="action"
        {portalTo}
        onposted={(post) => results.replace({ ...image, posts: [...image.posts, post] })}
      />
    {/if}

    <!--
      A second edit action beside the block it opens (`tag-row-and-inspector-fixes` design D5):
      the title row's own pencil is a screen away, past a tag list that is
      often long. "Details" because the block has no name on screen otherwise —
      every other section here opens the same way, Tags' own pencil sitting in
      exactly this spot.
    -->
    <section class="border-t border-border px-4 py-3">
      <div class="flex items-center justify-between gap-2">
        <h3 class="text-xs font-medium text-muted-foreground">Details</h3>
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
      </div>

      <dl class="mt-2 grid grid-cols-[auto_minmax(0,1fr)] gap-x-3 gap-y-1.5 text-xs">
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

        <dt class="text-muted-foreground">Origin</dt>
        <dd class="wrap-break-word">{origin}</dd>

        {#if artistMatchAnswer}
          <!--
            `artist-workflow` design D5: who the library says the image's
            author is, from the adapter record (or, for an image stored with
            none, the X account its page URL names) and the artist entries — a
            different fact from Account below, which it sits above (spec
            `tag-editing`'s own "does not replace it"). Read the same in both
            `editingFacts` states, like Account.
          -->
          <dt class="text-muted-foreground">Artist</dt>
          <dd>
            {#if artistMatchAnswer.owner}
              {@const owner = artistMatchAnswer.owner}
              <button
                type="button"
                class="
                  cursor-pointer rounded-md px-1 py-0.5
                  {CATEGORY_TEXT_CLASS.artist}
                  {searchMarkClass(searchMark(owner, terms.included, terms.excluded), 'hover:bg-accent')}
                "
                onclick={() => query(toggleTagInQuery(tagQuery, owner))}
              >
                {owner}
              </button>
            {:else}
              {@const match = artistMatchAnswer}
              <Button
                type="button"
                size="xs"
                variant="ghost"
                class="h-auto px-1 py-0.5 text-xs"
                onclick={() => (editingArtist = { mode: 'create', tag: match.derived ?? '', url: match.url })}
              >
                Create artist…
              </Button>
            {/if}
          </dd>
        {/if}
        {#if artistMatchError}
          <dt class="text-muted-foreground">Artist</dt>
          <dd class="text-destructive">{artistMatchError}</dd>
        {/if}

        {#if image.account}
          {@const account = image.account}
          <!--
            Spec `tag-editing`, "An X account on screen is a search term": a
            fact of the page address, beside Page, not among the tags —
            `account:` searches the address, not the tags, and the handle
            reaches the tags only as the derived artist tag a capture creates
            from it (`auto-artist-tag`, spec `capture-ingest` "A capture is
            stored with its author as an artist tag"); this row never writes
            one. Read the same in both `editingFacts` states: the address
            being edited is the draft, but the account is derived from the
            stored one, so this row does not flip with the form. No colour of
            its own (blue is general's now, design D2) — the padded,
            hoverable box and the pointer cursor are what say it is a control,
            not a fact to merely read (owner, 2026-09-23: the row did not look
            clickable). `searchMarkClass` (design D3, amended again
            2026-09-23) supplies `hover:bg-accent` only while the row carries
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

        <dt class="text-muted-foreground">Source</dt>
        {#if editingFacts}
          <dd>
            <Input
              bind:value={draftPageUrl}
              class="h-6 px-1.5 text-xs"
              aria-label="Source address"
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

        <dt class="text-muted-foreground">Source file</dt>
        {#if editingFacts}
          <dd>
            <Input
              bind:value={draftImageUrl}
              class="h-6 px-1.5 text-xs"
              aria-label="Source file address"
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

        <dt class="text-muted-foreground">Size</dt>
        <dd>{formatSizeLine(image)}</dd>

        <dt class="text-muted-foreground">Date</dt>
        <dd>
          <!--
            A tooltip, not a `title` attribute (`inspector-facts-relabel`
            design D3): the native one is slow and unstyled, and the app
            already has this styled one — same mount `TagNoteIndicator` uses,
            so it portals into the viewer placement too. `formatTimestamp`
            already reads a non-finite number as `—`; passing `NaN` for an
            image with no file behind it reuses that fallback instead of a
            second one written here (design D11, moved from the old File
            modified row).
          -->
          <Tooltip.Root delayDuration={150}>
            <Tooltip.Trigger tabindex={-1}>
              {#snippet child({ props })}
                <span {...props}>{formatRelative(image.capturedAt)}</span>
              {/snippet}
            </Tooltip.Trigger>
            <Tooltip.Content portalProps={{ to: portalTo }} class="block text-left">
              <p>Captured {formatTimestamp(image.capturedAt)}</p>
              <p>Imported {formatTimestamp(image.createdAt)}</p>
              <p>File modified {formatTimestamp(image.fileModifiedAt ?? Number.NaN)}</p>
            </Tooltip.Content>
          </Tooltip.Root>
        </dd>

        <dt class="text-muted-foreground">ID</dt>
        <dd class="font-mono wrap-break-word">{image.id}</dd>
      </dl>
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
      Slot Inspector · actions (`trash` design D13). `mt-auto` keeps it at the
      foot of the panel rather than floating under a short tag list, and it is
      inside this branch, so a panel showing no image has no action row at all.
    -->
    <section class="mt-auto flex gap-2 border-t border-border px-4 py-3">
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
    </section>

    <!--
      `part="hint"` last (design D2 of `sidebar-inspector-polish`): the note
      that no booru is configured, kept out of the trash view like the action
      mount above.
    -->
    {#if results.view === 'library'}
      <UploadAction {image} part="hint" />
    {/if}
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

<!--
  Outside the menu above, the same reason as `CollectionNameDialog`, and
  mounted unconditionally like it: a bits-ui `Dialog` torn down while still
  open keeps running its own close effects against derived state that no
  longer exists (`derived_inert`). Reads `editingArtist`'s own snapshot, not
  `image` — see its doc comment above. `onartistsaved` is the screen's
  `afterWrite`, which refreshes the vocabulary and the search; the Artist row
  follows `artistRevision`, which the dialog bumps itself.
-->
<ArtistDialog
  open={editingArtist !== null}
  request={editingArtist}
  {portalTo}
  onclose={() => {
    editingArtist = null
    onrelease?.()
  }}
  onsaved={onartistsaved}
/>

<!--
  Outside the menus above, mounted unconditionally, the same reason as
  `ArtistDialog`: one dialog for the chips of both strips and the badges
  (`tag-notes` design D10), `{portalTo}` so it lands in the viewer's own
  `<dialog>` when this panel is shown there.
-->
<TagNoteDialog
  open={editingNote !== null}
  name={editingNote ?? ''}
  {portalTo}
  onclose={() => {
    editingNote = null
    onrelease?.()
  }}
/>

<PinnedGroupsDialog
  open={managingGroups}
  {portalTo}
  onclose={() => {
    managingGroups = false
    onrelease?.()
  }}
/>
