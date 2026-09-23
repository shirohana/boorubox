<script lang="ts">
  import { tick, untrack } from 'svelte'
  // The library screen, drawn against one of two sets of images (`trash` design
  // D1). `/` renders it over the library and `/trash` over the trash; the grid,
  // the search, the viewer, the inspector and the selection are the same
  // components in both. The view decides the request that is searched, which
  // actions each control offers, and what the empty state says — nothing else.
  import type { DeleteReport, ExportReport, ImageRecord, Rating, TagEditSpec } from '@boorubox/shared'
  import {
    CLICK_ZOOM_CEILING_DEFAULT,
    GRID_TILE_DEFAULT,
    GRID_TILE_MAX,
    GRID_TILE_MIN,
  } from '@boorubox/shared'
  import PanelRightIcon from '@lucide/svelte/icons/panel-right'
  import StampIcon from '@lucide/svelte/icons/stamp'
  import TagsIcon from '@lucide/svelte/icons/tags'
  import {
    applyEdit,
    booruSites,
    browseSession,
    buildSearchRequest,
    bulkSetRating,
    deleteForever,
    emptyTrash,
    errorText,
    imports,
    library,
    matchingIds,
    onCaptureStored,
    onFileDrop,
    restoreImages,
    searchIds,
    searchPosition,
    settings,
    stamps,
    trash,
    trashImages,
    vocabulary,
    type SearchInputs,
  } from '$lib/api'
  import { Selection } from '$lib/api'
  import ConfirmDialog from '$lib/components/common/ConfirmDialog.svelte'
  import { frame } from '$lib/components/frame/frame.svelte'
  import CollectionsSection from '$lib/components/tags/CollectionsSection.svelte'
  import RatingPills from '$lib/components/tags/RatingPills.svelte'
  import TagSidebar from '$lib/components/tags/TagSidebar.svelte'
  import { parseStamp } from '$lib/domain/stamp'
  import DeleteReportCard from './DeleteReportCard.svelte'
  import EmptyState from './EmptyState.svelte'
  import { isOrphanedFocus } from './focus-handback'
  import ImportMenu from './ImportMenu.svelte'
  import ExportReportCard from './ExportReportCard.svelte'
  import ImportReportCard from './ImportReportCard.svelte'
  import Inspector from './Inspector.svelte'
  import LibraryGrid from './LibraryGrid.svelte'
  import Lightbox from './Lightbox.svelte'
  import PendingBand from './PendingBand.svelte'
  import SearchBar from './SearchBar.svelte'
  import AccountRail from './AccountRail.svelte'
  import type { ConfirmPrompt, PendingWrite } from './pending-write'
  import { confirmPrompt, needsConfirmation } from './pending-write'
  import SelectionToolbar from './SelectionToolbar.svelte'
  import StampBar from './StampBar.svelte'
  import type { TrashActions } from './trash-actions'
  import ViewControls from './ViewControls.svelte'
  import { Button } from '$lib/components/ui/button'
  import { Input } from '$lib/components/ui/input'
  import { Slider } from '$lib/components/ui/slider'
  import { useSidebar } from '$lib/components/ui/sidebar'
  import { Toggle } from '$lib/components/ui/toggle'
  import {
    blurOnEscape,
    isInDialog,
    isTypingTarget,
    KEY_ENTER,
    KEY_ESCAPE,
    KEY_SEARCH,
    KEY_SELECT_ALL,
  } from '$lib/keyboard'

  let { view }: { view: 'library' | 'trash' } = $props()

  // The route's result set, kept by `browseSession` rather than owned here
  // (`browse-feedback` design D1): a route remount — leaving for the trash,
  // the import screen or settings and coming back — must not lose it. The two
  // routes still never share one; only the lifetime moved out of this
  // component, to the app's whole run. Read once on purpose: `view` is fixed
  // for this screen's life, exactly as `SearchResults.view` says of itself.
  // svelte-ignore state_referenced_locally
  const results = browseSession.resultsFor(view)

  // The `/` shortcut's way to expand a collapsed sidebar before it focuses the
  // tag query (`sidebar-layout` design D4). Read once, during initialisation,
  // because `useSidebar` reads Svelte's component context and cannot be called
  // later from inside `screenKeys` — the provider is `+layout.svelte`'s, an
  // ancestor of this screen.
  const sidebarState = useSidebar()

  /**
   * The focus, the anchor and the selection, which are one state machine
   * (`selection-and-bulk` design D1). Its id resolver is the same request the
   * grid pages, at the range the selection covers (its design D3): row *n* of
   * the selection and row *n* of the grid have to be the same image. Its match
   * resolver is the same request over `limit`/`offset` of zero — `matchingIds`
   * ignores both (`browse-fixes` design D1) — so a write's prune reads the
   * same search the grid is showing, never a stale one.
   */
  const currentView = () => ({ sort: results.sort, group: results.group, view: results.view })
  const selection = new Selection(
    (offset, limit) =>
      searchIds(buildSearchRequest(results.inputs, currentView(), offset, limit)),
    (ids) => matchingIds(buildSearchRequest(results.inputs, currentView(), 0, 0), ids),
  )

  // The query lives here, not in the search bar: the sidebar, the rating pills
  // and the inspector rewrite it too (design D14), and the field has to show
  // what ran. Seeded from `results.inputs` rather than empty (`browse-feedback`
  // design D1): on a route remount that is the query the screen left with.
  let tagQuery = $state(results.inputs.tagQuery)
  let text = $state(results.inputs.text)
  const inputs = $derived<SearchInputs>({ tagQuery, text })
  // The 250 ms typing pause for both fields, one timer (`browse-feedback`
  // design D3): the sidebar's tag field and the toolbar's free-text field sit
  // in two different regions of the frame now, so a pause owned by either one
  // would leave the other without it.
  const TYPING_PAUSE_MS = 250
  let searchTimer: ReturnType<typeof setTimeout> | undefined

  function searchNow() {
    clearTimeout(searchTimer)
    runSearch({ tagQuery, text })
  }

  function searchAfterPause() {
    clearTimeout(searchTimer)
    searchTimer = setTimeout(searchNow, TYPING_PAUSE_MS)
  }

  $effect(() => () => clearTimeout(searchTimer))

  // The grid follows the drag; only the release writes the setting (design D11),
  // so this is the live edge and `gridTileSize` is where it comes back from.
  let tile = $state(settings.current?.gridTileSize ?? GRID_TILE_DEFAULT)
  let lightboxIndex = $state(0)
  let lightboxOpen = $state(false)
  /** Written by the grid, read by the viewer's row step (design D9). */
  let columns = $state(1)
  let grid = $state<LibraryGrid | null>(null)
  let hovering = $state(false)
  let actionError = $state<string | null>(null)
  /** Kept until dismissed: the only place a missing file is named (design D11). */
  let exportReport = $state<ExportReport | null>(null)
  /** Kept until dismissed: the only place a file that would not go is named (D4). */
  let deleteReport = $state<DeleteReport | null>(null)
  /**
   * The write waiting on the one confirmation this screen owns; `null` while
   * none is asked. One state and one dialog for all three questions: a second
   * dialog would be a second place to keep the confirm handler and the
   * selection-survives-the-question rule in step.
   */
  let pendingWrite = $state<PendingWrite | null>(null)

  /**
   * Edit mode (`stamps` design D4): one boolean, toggled by the toolbar's
   * `Toggle` alone — no key, by the owner's call (D4): a key is for the most
   * frequent day-to-day action, and a mode where a click writes is not one.
   * `stampText` is the bar's field, cleared the moment
   * the mode is left below, since a one-off stamp is meant to be lost that
   * way (design D5 risk: "Save as stamp…" is the way to keep it).
   */
  let editMode = $state(false)
  let stampText = $state('')
  /**
   * A stamp click Rust refused (`stamps` design D4), shown in the bar rather
   * than in `actionError` above the grid — the bar is what the user is
   * looking at while clicking, and the banner sits above a grid the click
   * never scrolled away from. Keyed to the text that earned it, not cleared
   * by an effect: the refusal belongs to the field's text at the moment it
   * was refused, so the template below only shows it while `stampText` still
   * equals `stampError.text` — any edit to the field changes that comparison
   * and the message disappears on its own, with no separate clear to forget.
   */
  let stampError = $state<{ text: string, message: string } | null>(null)

  $effect(() => {
    if (!editMode) stampText = ''
  })

  /**
   * What a plain click on a thumbnail applies while edit mode is on: an id
   * and a name when the field's text matches a saved stamp (first by list
   * order), `undefined`/`null` for a one-off (owner's review, 2026-09-23 —
   * amending design D4's own `$state`). The field is the one signal now, so
   * this is a `$derived` of it rather than a second place to hold what is
   * active: blank or unparseable text is `null`, same as leaving the mode.
   */
  type ActiveStamp = { id?: string, name: string | null, text: string, edit: TagEditSpec }
  const activeStamp = $derived.by((): ActiveStamp | null => {
    const parsed = parseStamp(stampText)
    if ('error' in parsed) return null
    const saved = stamps.list.find((stamp) => stamp.text === stampText)
    return { id: saved?.id, name: saved?.name ?? null, text: stampText, edit: parsed.edit }
  })

  const libraryPath = $derived(library.status?.libraryPath ?? null)
  const focused = $derived(results.at(selection.focus) ?? null)
  /**
   * The grid's tag footer: always on in edit mode, otherwise the toolbar's
   * own toggle (`tile-tags-in-edit-mode` design D3). Same rule as
   * `clickZoomCeiling` below: a `$derived` of `settings.current`, not an
   * `$effect` on it, so this doesn't re-run on every unrelated settings write.
   */
  const showTags = $derived(editMode || (settings.current?.showTileTags ?? false))
  /**
   * The viewer's click zoom ceiling, as a multiple of the fit
   * (`click-zoom-ceiling` design D3): read live off the settings store, never
   * copied into local state, so a change on the settings screen takes effect
   * the next time the viewer opens with no restart. A `$derived`, not an
   * `$effect` on `settings.current` — a field read off a store object
   * reassigned wholesale re-runs on every unrelated settings write.
   */
  const clickZoomCeiling = $derived(
    (settings.current?.clickZoomCeilingPercent ?? CLICK_ZOOM_CEILING_DEFAULT) / 100,
  )
  /**
   * The dialog's words. They outlive the pending write by the dialog's closing
   * animation: nulling them with it blanks the title mid-fade, so the last
   * question stays until the next one replaces it.
   */
  let lastPrompt: ConfirmPrompt | null = null
  const prompt = $derived.by(() => {
    if (pendingWrite) lastPrompt = confirmPrompt(pendingWrite, trash.count)
    return lastPrompt
  })

  function runSearch(next: SearchInputs) {
    tagQuery = next.tagQuery
    text = next.text
    // A new list: the old index names a different image, or none at all — and a
    // count over a result nobody is looking at any more is worse than none
    // (design D4).
    selection.reset()
    lightboxOpen = false
    void results.run(next)
  }

  /**
   * The refreshes nobody on this screen asked for. A capture lands at row 0 and
   * an import moves rows too, so a `kind: 'range'` selection that outlived one
   * of them would name images the user never picked (design D2). Pinning it by
   * id first is what an action does anyway, and it leaves the store in id mode
   * (design D4): never re-run the search under a live range.
   */
  async function refreshResults() {
    // `tag-vocabulary` design D5: a capture or an import can land through a
    // rule that creates an artist, and neither path writes through
    // `results.saveTags`, so this refresh is the only hook that sees it.
    void vocabulary.refresh()
    await selection.ids()
    await results.refresh()
  }

  /**
   * The one way the viewer opens: the grid's `Enter` and its second click, and
   * the inspector's selection strip (`selection-and-bulk` design D7, amended).
   * A second viewer for the strip would be a second thing to close.
   */
  function openViewer(index: number) {
    lightboxIndex = index
    lightboxOpen = true
  }

  /**
   * Every tag, rating, sidebar and account click rewrites the query and runs
   * it, keeping `id` — the image the click's panel describes — current in the
   * new result (`inspector-polish` design D2): the screen asks
   * `search_position` for the row `id` now occupies, in parallel with the
   * search itself, then moves the grid's focus there and, with the viewer
   * open, its index. With no id to keep — nothing is focused — this is a
   * plain reset, exactly like a typed search (design Non-Goals).
   */
  async function searchKeeping(next: string, id: string | undefined) {
    if (id === undefined) {
      runSearch({ tagQuery: next, text })
      return
    }

    const nextInputs: SearchInputs = { tagQuery: next, text }
    const view = { sort: results.sort, group: results.group, view: results.view }
    // Everything a typed search does, done now rather than a round trip later:
    // the field has to show the query that is running, and the old indices name
    // other images the moment `run` clears the rows. Only where the focus lands
    // has to wait for an answer.
    tagQuery = next
    selection.reset()
    const running = results.run(nextInputs)
    // Read after `run` has started — `#start` bumps it before its first await —
    // and never `results.inputs`: that field comes back out of `$state` as a
    // *proxy* of what was assigned, so it never compares equal to the object
    // this call passed in and the guard below would fire on every click.
    const generation = results.generation
    const [, row] = await Promise.all([
      running,
      // A position that cannot be answered is not a row: nobody awaits this
      // call, and an escaping rejection would leave the viewer open on an index
      // that now names another image (design D2's rejected alternative).
      searchPosition(buildSearchRequest(nextInputs, view, 0), id).catch((error) => {
        actionError = errorText(error)
        return null
      }),
    ])

    // A second click, a typed search or a refresh started a newer run while
    // this one waited on the position: the row it found is a row of a result
    // nobody is looking at (design D2 risk: the race with a fast second click).
    if (results.generation !== generation) return

    if (row === null) {
      lightboxOpen = false
      return
    }
    grid?.focusCard(row)
    if (lightboxOpen) {
      lightboxIndex = row
      // The run loaded page 0 and this row is usually not on it — that is why
      // the user is filtering. The viewer reads `results.at(index)` and would
      // sit on "Loading…" otherwise; its own `move` ensures the same range for
      // the same reason.
      results.ensureRange(row, row + 1)
    }
  }

  /** Slot Grid · tile: the menu rates its own image, not the inspector's. */
  async function rate(image: ImageRecord, rating: Rating | null) {
    try {
      await results.saveRating(image.id, rating)
    } catch (error) {
      actionError = errorText(error)
    }
  }

  /**
   * What every write that re-reads the search changes on this screen
   * (`browse-fixes` design D1): the rows, and the selection along with them —
   * an id the re-read search no longer matches leaves it, so the count, the
   * strip and the next bulk action never describe an image nobody can see.
   * One path for every such write (bulk tags, bulk rating, a collection
   * change, trash and restore) is what keeps the prune from being forgotten
   * by the next one; `keepMatching` asks the search itself rather than
   * trusting a list of ids the caller happened to write, which is the only
   * way to know which of a bulk tag edit's ids left the result.
   *
   * A live range has to be resolved to ids before the write moves the rows
   * out from under it, and every writer that takes ids does so itself; the
   * resolve here is the backstop for a caller that writes without them (the
   * sidebar's collection change), because pruning a range against rows that
   * now name different images would drop or keep the wrong ones.
   *
   * The refresh replaces every row, so the focused card — and the element the
   * focus was on — leaves the DOM and the focus falls to `<body>`, where the
   * next `Delete` reaches nothing. The refocus runs unconditionally, not only
   * when the old focus has run past the end: it is what lets `Delete` be
   * pressed twice in a row, since the first press would otherwise leave the
   * focus on `<body>` for the second one to reach nothing from.
   */
  async function afterWrite() {
    // `tag-vocabulary` design D5: no ordering dependency on the selection
    // prune below, so it runs alongside rather than gating it — a bulk tag
    // edit, a rule run and every other writer that ends up here may have
    // created or emptied a categorised tag.
    void vocabulary.refresh()
    await selection.ids()
    await results.refresh()
    await selection.keepMatching()
    if (selection.focus >= 0 && results.total > 0) {
      grid?.focusCard(Math.min(selection.focus, results.total - 1))
    }
  }

  /** Design D7: the whole trash, whatever the current search narrowed it to. */
  const askEmptyTrash = () => (pendingWrite = { kind: 'empty' })

  const actions: TrashActions = {
    // Design D12, amended: one image is still the unconfirmed keystroke the
    // decision argues for; two or more ask, whichever control asked for them —
    // the keys, the toolbar and the menus all arrive here. The selection is
    // left alone until the answer comes back, so dismissing the dialog leaves
    // the user exactly what they had picked.
    trash: (ids) => {
      if (needsConfirmation(ids.length)) pendingWrite = { kind: 'trash', ids }
      else void write(() => trashImages(ids))
    },
    restore: (ids) => void write(() => restoreImages(ids)),
    deleteForever: (ids) => (pendingWrite = { kind: 'delete', ids }),
  }

  /**
   * Slot Toolbar · rate (`bulk-confirm` design D2): the same "one or many"
   * rule as trash, over a write with no restore. One image writes at once;
   * two or more ask first, naming the rating (or the clear) and the count.
   * Named apart from the tile menu's own `rate` above — that one rates the
   * image the menu is on, this one a selection of any size.
   */
  function rateSelection(ids: string[], rating: Rating | null) {
    if (needsConfirmation(ids.length)) pendingWrite = { kind: 'rate', ids, rating }
    else void writeRating(ids, rating)
  }

  async function writeRating(ids: string[], rating: Rating | null) {
    try {
      await bulkSetRating(ids, rating)
      await afterWrite()
    } catch (error) {
      actionError = errorText(error)
    }
  }

  /**
   * The pinned chip's write over a selection (`tag-vocabulary` design D8):
   * the same "one or many" rule as {@link rateSelection}. Called through the
   * inspector's `onedit` prop, whose signature stays `(ids, add, remove)`
   * (`stamps` design D5): the chip only ever fills one of the two, so it has
   * nothing to gain from building a `TagEditSpec` itself, and this is the one
   * place that does — the bulk dialog builds its own, for the same reason it
   * always has (design D8). `label` is the tag itself: `confirmPrompt` never
   * reads it for a single-tag edit, which this always is.
   */
  function editSelectionTags(ids: string[], add: string[], remove: string[]): void {
    const spec: TagEditSpec = { add, remove, addCollections: [], removeCollections: [] }
    const pending: Extract<PendingWrite, { kind: 'edit' }> = {
      kind: 'edit',
      ids,
      spec,
      label: add[0] ?? remove[0] ?? '',
    }
    if (needsConfirmation(ids.length)) pendingWrite = pending
    else void writeEdit(pending)
  }

  /**
   * The confirmed half of every `edit` write, whatever built the spec — the
   * bulk dialog, the pinned chip, or `applyStampToSelection` below.
   * `afterWrite`, not `results.replaceMany`: a spec can move an image out of
   * the current search (a tag removed, a collection left), and only a re-run
   * prunes the selection to what still matches — the same reason every other
   * selection-wide writer below ends there.
   */
  async function writeEdit(pending: Extract<PendingWrite, { kind: 'edit' }>) {
    try {
      await applyEdit(pending.ids, pending.spec)
      await afterWrite()
    } catch (error) {
      actionError = errorText(error)
    }
  }

  /**
   * Slot StampBar · Apply to N selected (design D5): the same confirm/write
   * split as {@link editSelectionTags}, over the active stamp's whole spec
   * instead of one tag — `label` is the stamp's name, or its text while it is
   * still a one-off with none. A live range has to become ids before the
   * pending write can name them, same as every other selection-wide writer.
   */
  async function applyStampToSelection(): Promise<void> {
    if (!activeStamp) return
    const stamp = activeStamp
    const ids = await selection.ids()
    const pending: Extract<PendingWrite, { kind: 'edit' }> = {
      kind: 'edit',
      ids,
      spec: stamp.edit,
      label: stamp.name ?? stamp.text,
    }
    if (needsConfirmation(ids.length)) pendingWrite = pending
    else void writeEdit(pending)
  }

  /**
   * Slot Grid · onstamp (design D4): a plain click in edit mode applies the
   * active stamp to just the image clicked. `applyEdit`'s own returned rows
   * go straight to `results.replaceMany`, not `afterWrite` — a click has to
   * show its result at once (spec `stamps`, "Enter and stamp"), and a whole
   * search re-run is `applyStampToSelection`'s job, not a single tile's. The
   * clicked card becomes current, as a focusing click would, but the
   * selection is left exactly as it stood (spec `selection`, "A plain click
   * in edit mode"). `vocabulary.refresh()` only when the spec could have
   * created a tag — a stamp that only removes, rates or moves collections
   * cannot.
   */
  async function onStamp(index: number, id: string): Promise<void> {
    const stamp = activeStamp
    if (!stamp) return
    try {
      const records = await applyEdit([id], stamp.edit)
      results.replaceMany(records)
      selection.focusAt(index)
      if (stamp.edit.add.length > 0) void vocabulary.refresh()
      stampError = null
    } catch (error) {
      stampError = { text: stampText, message: errorText(error) }
    }
  }

  /** The confirmed half of every question the dialog asks. */
  function commit(pending: PendingWrite) {
    if (pending.kind === 'trash') void write(() => trashImages(pending.ids))
    else if (pending.kind === 'rate') void writeRating(pending.ids, pending.rating)
    else if (pending.kind === 'edit') void writeEdit(pending)
    else void destroy(pending)
  }

  /**
   * The trash and restore commands, which also move the library's and the
   * trash's own totals (design D13) — a bulk tag or rating edit moves
   * neither, which is why only this and `destroy` below (trash, restore,
   * delete-forever, empty trash) also refresh them.
   */
  async function write(run: () => Promise<void>) {
    try {
      // A range is rows of the current result, and the write is about to move
      // them; resolving it now is what lets the rest of it survive the write.
      await selection.ids()
      await run()
      await Promise.all([library.refresh(), trash.refresh()])
      await afterWrite()
    } catch (error) {
      actionError = errorText(error)
    }
  }

  /**
   * The confirmed half of both irreversible actions (design D7): emptying the
   * trash is `delete_forever` over everything in it, so it answers with the
   * same report and is followed by the same refresh.
   */
  async function destroy(pending: { kind: 'delete', ids: string[] } | { kind: 'empty' }) {
    // The selection, not the pending ids: a live range must become ids before
    // the delete moves the rows it was counted over (`afterWrite`'s note).
    await selection.ids()
    try {
      const report = pending.kind === 'empty'
        ? await emptyTrash()
        : await deleteForever(pending.ids)
      // A report with nothing left behind says only what the trash already
      // shows; the card exists for the paths, so it appears only when there are
      // any (design D4).
      if (report.filesLeft.length > 0) deleteReport = report
      await Promise.all([library.refresh(), trash.refresh()])
      await afterWrite()
    } catch (error) {
      actionError = errorText(error)
    }
  }

  // The configured boorus, for this screen's posted marks and the inspector's
  // upload action. Read here rather than in either of them: the tiles and the
  // panel have to name a site the same way, and the store answers once per
  // library (`booru-upload` design D13).
  $effect(() => {
    void booruSites.load(libraryPath)
  })

  // Spec `library-switching`: everything reading the library follows a switch,
  // and the switch is started from the sidebar footer, which this screen cannot
  // hear about any other way than by the path it holds changing.
  let shownPath = untrack(() => libraryPath)
  $effect(() => {
    const path = libraryPath
    if (path === shownPath) return
    shownPath = path
    selection.reset()
    lightboxOpen = false
    // The reports describe runs into the library that was open, not this one.
    imports.dismissAll()
    deleteReport = null
    // A question about the library that was open has no answer in this one.
    pendingWrite = null
    void results.run(inputs)
  })

  // A capture from the browser extension lands in the library with nothing on
  // this screen having asked for it. Without this the grid keeps the result set
  // it last searched, and the image only appears once something else re-runs the
  // search — leaving and coming back to the route, for instance.
  $effect(() => {
    const subscription = onCaptureStored(() => void refreshResults())
    subscription.catch((cause) => (actionError = errorText(cause)))
    return () => {
      void subscription.then((unlisten) => unlisten()).catch(() => {})
    }
  })

  // The runs outlive this route (design D9), so what they change is told to
  // whoever is on screen rather than to whoever started them.
  $effect(() => imports.onfinished(() => void refreshResults()))

  // Design D8: the subscription is the route's, not the Import menu's. A menu is
  // unmounted while its dropdown is closed, which is nearly always — registered
  // in there, drop-to-import would silently stop working.
  //
  // Not on /trash: an import lands in the library, which that view cannot show,
  // so a drop there would offer to do something and then appear to do nothing.
  // The overlay below follows from this — nothing else sets `hovering`.
  $effect(() => {
    if (view !== 'library') return
    const subscription = onFileDrop({
      onhover: () => (hovering = true),
      onleave: () => (hovering = false),
      ondrop: (paths) => {
        hovering = false
        imports.enqueue(paths)
      },
    })
    subscription.catch((cause) => (actionError = errorText(cause)))
    return () => {
      void subscription.then((unlisten) => unlisten()).catch(() => {})
    }
  })

  /**
   * The bindings that belong to this screen rather than to one region of it
   * (`app-shell` design D14, `selection-and-bulk` D5 amended). They live here
   * and not in the layout because each is about what this screen holds — the
   * toolbar's search field, the current result, and now edit mode
   * (`stamps` design D4); bound in the layout they would fire on /settings,
   * where none of the three exists.
   *
   * Select-all was the grid's until the owner found it only worked once the
   * grid had been clicked into — with the focus anywhere else the webview's own
   * select-all ran and highlighted the page. It is one binding, not two: the
   * grid no longer reads the key at all.
   */
  function screenKeys(event: KeyboardEvent) {
    // A dialog is what the user is looking at, and the viewer shows one image
    // and neither reads nor writes the selection (design Non-Goals).
    if (lightboxOpen || isTypingTarget(event) || isInDialog(event)) return

    if (event.key === KEY_SEARCH) {
      event.preventDefault()
      // Collapsed to the icon rail the field is `display: none`
      // (`group-data-[collapsible=icon]:hidden`), so the sidebar has to expand
      // first; `tick` waits for that state change to reach the DOM before the
      // focus call can land on a field that is still hidden (design D4). The
      // sidebar's tag field is an `Input` again (`browse-feedback` design D3);
      // the `instanceof` check still accepts a textarea too, since `TagInput`
      // renders one there for the inspector's own multiline editor.
      sidebarState.setOpen(true)
      void tick().then(() => {
        const field = document.getElementById('tag-query')
        if (field instanceof HTMLInputElement || field instanceof HTMLTextAreaElement) {
          field.focus()
        }
      })
      return
    }

    if ((event.metaKey || event.ctrlKey) && !event.altKey
      && event.key.toLowerCase() === KEY_SELECT_ALL) {
      // Suppresses the webview's own select-all, which is what the owner was
      // getting instead: text fields and dialogs are excluded above, so nothing
      // the user could have meant by it is lost.
      event.preventDefault()
      selection.selectAll(results.total)
      return
    }

    // Beside select-all for the same reason: a selection made with the focus
    // outside the grid needs a way out that does not go through the grid.
    // Never the focus: there would be no way back to it but the pointer.
    if (event.key === KEY_ESCAPE && selection.count > 0) {
      event.preventDefault()
      selection.clear()
    }
  }

  /**
   * A click anywhere that leaves no control focused hands the keyboard back
   * to the grid's current card (`browse-fixes` design D4): the panel's plain
   * text, an address, empty space in the sidebar or the toolbar band all take
   * the DOM focus off the card without meaning to, and every one of them is a
   * click somewhere under this window — a window listener is what reaches
   * the toolbar and the sidebar, which render into the frame's own regions
   * and never into this screen's own DOM. Bound on the window beside
   * `screenKeys`, so it runs after every target's own click handler and after
   * `mousedown` has already moved the focus (`ImageCard`'s note on
   * `onpointerdown`). A click inside the viewer's modal dialog leaves the
   * dialog itself the active element, which `isOrphanedFocus` does not count,
   * so nothing fires there.
   */
  function onScreenClick(event: MouseEvent) {
    // Not from inside a dialog: the viewer's native one keeps itself active,
    // but the bulk-tag, confirm, name and upload dialogs are plain `role`d
    // divs, and a click on their text can leave `<body>` active — refocusing
    // a card behind them would hand Delete and Space to a grid the user
    // cannot see, racing the dialog's own focus trap.
    if (isInDialog(event)) return
    if (selection.focus >= 0 && isOrphanedFocus(document.activeElement)) grid?.refocus()
  }

  // The route's controls in the frame's top bar and its sidebar region, for as
  // long as this route is mounted (see `frame.svelte.ts`). The frame's sidebar
  // slot is absent rather than empty on every other screen, which is what
  // unsetting it here means.
  $effect(() => {
    frame.toolbar = toolbar
    frame.sidebar = sidebar
    return () => {
      frame.toolbar = null
      frame.sidebar = null
    }
  })

  void results.run(results.inputs)
</script>

<svelte:window onkeydown={screenKeys} onclick={onScreenClick} />

{#snippet toolbar()}
  <!--
    Design D2: the free-text field sits at the left, beside the sidebar
    toggle `TopBar` renders before this snippet. There is no form and no
    Search button around it, as there never was one here (the query runs
    after a typing pause and on Enter): `TagInput` in the sidebar submits
    itself the same way, and a bare `Input` needs its own Enter handling
    since it has no such rule of its own.
  -->
  <Input
    id="text-query"
    class="h-7 w-56 shrink text-xs"
    aria-label="Page title or URL"
    bind:value={text}
    oninput={searchAfterPause}
    onkeydown={(event) => {
      blurOnEscape(event)
      if (event.key === KEY_ENTER) searchNow()
    }}
    placeholder="Title or URL"
    autocomplete="off"
    autocorrect="off"
    spellcheck={false}
  />

  <div class="flex-1"></div>

  <!--
    Design D4: before `SelectionToolbar`, not inside it — the toggle has to
    stay reachable while a selection stands, so the mode can be left without
    clearing the selection first. One render site covers both toolbars, since
    nothing here conditions it on `selection.count`.
  -->
  <Toggle
    size="sm"
    variant="outline"
    bind:pressed={editMode}
    aria-label="Edit mode"
    title="Edit mode: click an image to apply the active stamp"
  >
    <StampIcon />
  </Toggle>

  {#if selection.count > 0}
    <SelectionToolbar
      {selection}
      {results}
      {actions}
      rate={rateSelection}
      onerror={(message) => (actionError = message)}
      onexported={(report) => (exportReport = report)}
      onapplied={afterWrite}
    />
  {/if}

  <div class="flex-1"></div>

  <!--
    Design D2: this group — the screen's own action, the tile size and the
    inspector toggle — holds the right edge on every platform, selection or
    not (spec `app-frame`, "the controls at both edges are where they were
    before the selection"): the two spacers above collapse to one gap when
    there is nothing to put in the middle, and expand to make room for the
    selection toolbar without moving anything at either edge. The screen's
    own action still yields to a selection (`selection-and-bulk` design D6):
    the group at this edge only shortens, it does not move.
  -->
  {#if selection.count > 0}
    <!-- The selection toolbar above is this row's action while it lasts. -->
  {:else if view === 'trash' && trash.count > 0}
    <!--
      `trash` design D13: importing into the trash is meaningless, so on that
      screen this row offers the one action it actually has. An empty trash
      offers nothing here rather than a button that would do nothing (D7).
    -->
    <Button size="sm" variant="outline" onclick={askEmptyTrash}>Empty trash…</Button>
  {:else if view === 'library'}
    <ImportMenu />
  {/if}

  <Slider
    type="single"
    class="w-24 shrink-0"
    aria-label="Thumbnail size"
    min={GRID_TILE_MIN}
    max={GRID_TILE_MAX}
    step={10}
    bind:value={tile}
    onValueCommit={(size) => {
      settings.setGridTileSize(size).catch((error) => (actionError = errorText(error)))
    }}
  />

  <!--
    Disabled rather than hidden while edit mode is on, so the setting stays
    readable even though the mode forces it (`tile-tags-in-edit-mode` design
    D3); the variant follows the state as the inspector toggle's does below.
    The title sits on a wrapper: a disabled control gets no pointer events in
    either webview, so a title on the button itself never shows in the one
    state whose wording matters.
  -->
  <span
    class="inline-flex"
    title={editMode ? 'Tags under thumbnails (always on in edit mode)' : 'Tags under thumbnails'}
  >
    <Button
      size="icon-sm"
      variant={showTags ? 'secondary' : 'ghost'}
      aria-label="Show tags under thumbnails"
      aria-pressed={showTags}
      disabled={editMode}
      onclick={() => {
        settings.setShowTileTags(!showTags).catch((error) => (actionError = errorText(error)))
      }}
    >
      <TagsIcon />
    </Button>
  </span>

  <!-- The variant, not just `aria-pressed`: the state has to be visible. -->
  <Button
    size="icon-sm"
    variant={browseSession.inspectorOpen ? 'secondary' : 'ghost'}
    aria-label="Show or hide the inspector"
    aria-pressed={browseSession.inspectorOpen}
    onclick={() => (browseSession.inspectorOpen = !browseSession.inspectorOpen)}
  >
    <PanelRightIcon />
  </Button>
{/snippet}

{#snippet sidebar()}
  <!--
    Slot Sidebar · sidebar (`sidebar-layout` design D1, D5; order per
    `browse-feedback` design D4): Search, Rating, Tags, Collections, Order and
    grouping, top to bottom. The panels stay mounted while a search runs:
    unmounting them flickers the whole region on every click. Blank counts
    (`null`) are still honest (design D8) — the last query's numbers never
    show, only their own headings do until the new counts arrive.
  -->
  <SearchBar bind:tagQuery oninput={searchAfterPause} onsubmit={searchNow} />

  <RatingPills
    counts={results.counts?.ratings ?? null}
    {tagQuery}
    onquery={(next) => void searchKeeping(next, focused?.id)}
  />
  <TagSidebar
    tags={results.counts?.tags ?? null}
    {tagQuery}
    onquery={(next) => void searchKeeping(next, focused?.id)}
  />
  <CollectionsSection
    counts={results.counts?.collections ?? null}
    {tagQuery}
    onquery={(next) => void searchKeeping(next, focused?.id)}
    onchanged={() => void afterWrite()}
  />

  <ViewControls
    view={results.view}
    sort={results.sort}
    group={results.group}
    onsort={(sort) => {
      selection.reset()
      void results.setSort(sort)
    }}
    ongroup={(group) => {
      selection.reset()
      void results.setGroup(group)
    }}
  />
{/snippet}

{#each imports.reports as report (report)}
  <ImportReportCard {report} ondismiss={() => imports.dismiss(report)} />
{/each}

{#if exportReport}
  <ExportReportCard report={exportReport} ondismiss={() => (exportReport = null)} />
{/if}

{#if deleteReport}
  <DeleteReportCard report={deleteReport} ondismiss={() => (deleteReport = null)} />
{/if}

{#if actionError || imports.error}
  <p class="border-b border-border px-4 py-2 text-xs text-destructive">
    {actionError ?? imports.error}
  </p>
{/if}

{#if editMode}
  <StampBar
    bind:text={stampText}
    selectionCount={selection.count}
    onapplyselection={() => void applyStampToSelection()}
    error={stampError?.text === stampText ? stampError.message : null}
  />
{/if}

<div class="flex min-h-0 flex-1">
  <div class="flex min-w-0 flex-1 flex-col">
    <!-- Above everything the results area can be: the grid, the empty library
         and the search that found nothing (spec `pending-work`). -->
    <PendingBand {tile} />

    <div class="min-h-0 flex-1">
      {#if results.error}
        <div class="flex h-full flex-col items-center justify-center gap-2 p-8 text-center">
          <p class="text-sm font-medium">The search could not be run.</p>
          <p class="text-sm text-muted-foreground">{results.error}</p>
        </div>
      {:else if results.total === 0}
        <!--
          `total` holds its last answer while a search runs, so the grid is only
          replaced when there is genuinely nothing to show. Swapping it out on
          every refresh would remount it and throw away the scroll position.
        -->
        {#if results.loading}
          <p class="p-8 text-sm text-muted-foreground">Searching…</p>
        {:else}
          <EmptyState inputs={results.inputs} {view} />
        {/if}
      {:else}
        <LibraryGrid
          bind:this={grid}
          {results}
          {tile}
          {selection}
          {actions}
          bind:columns
          onactivate={openViewer}
          onrate={rate}
          ontoggleinspector={() => (browseSession.inspectorOpen = !browseSession.inspectorOpen)}
          onerror={(message) => (actionError = message)}
          onstamp={editMode && activeStamp ? (index, id) => void onStamp(index, id) : undefined}
          stampLabel={editMode ? activeStamp?.text : undefined}
          {showTags}
        />
      {/if}
    </div>
  </div>

  <!--
    Slot Grid · rail (`browse-feedback` design D9, `account-rail-counts` D2):
    every account of the view with what it would give, only while grouped by
    account. Between the grid and the inspector, and the grid gives up the
    width — the inspector's own stays as it is.
  -->
  {#if results.group === 'x-account'}
    <aside class="w-48 shrink-0 overflow-y-auto border-s border-border">
      <AccountRail
        accounts={results.counts?.accounts ?? null}
        {tagQuery}
        onquery={(next) => void searchKeeping(next, focused?.id)}
      />
    </aside>
  {/if}

  {#if browseSession.inspectorOpen}
    <aside class="w-80 shrink-0 border-s border-border">
      <Inspector
        image={focused}
        {results}
        {selection}
        {actions}
        {tagQuery}
        onquery={searchKeeping}
        onrelease={() => {
          // `tag-vocabulary` design D5: a tag save already refreshes the
          // vocabulary from inside `results.saveTags` itself, so this hook —
          // which also fires on a rating, a facts edit and a collection
          // change — does not need its own call; those writes never touch
          // the vocabulary, and asking again on every one of them cost an
          // IPC round trip nothing needed.
          grid?.refocus()
        }}
        onactivate={openViewer}
        onedit={editSelectionTags}
      />
    </aside>
  {/if}
</div>

{#if lightboxOpen}
  <Lightbox
    {results}
    {libraryPath}
    {columns}
    {actions}
    {clickZoomCeiling}
    {tagQuery}
    onquery={searchKeeping}
    bind:mode={browseSession.lightboxMode}
    bind:index={lightboxIndex}
    onmove={(index) => grid?.scrollIntoView(index)}
    onclose={() => {
      lightboxOpen = false
      // The native dialog has just put the focus back on the card it opened
      // from; the viewer may have moved on since, and the grid follows it.
      grid?.focusCard(lightboxIndex)
    }}
  />
{/if}

<!--
  The one dialog for all three questions (design D7, D12 amended). It stays
  mounted so it can animate closed; the words go with the pending write.
-->
<ConfirmDialog
  title={prompt?.title ?? ''}
  description={prompt?.description ?? ''}
  confirmLabel={prompt?.confirmLabel ?? ''}
  destructive={prompt?.destructive ?? true}
  open={pendingWrite !== null}
  onclose={() => (pendingWrite = null)}
  onconfirm={() => {
    const pending = pendingWrite
    pendingWrite = null
    if (pending) commit(pending)
  }}
/>

{#if hovering}
  <div
    class="
      pointer-events-none fixed inset-0 z-40 flex items-center justify-center bg-background/80
    "
  >
    <p class="rounded-xl border border-dashed border-border px-6 py-4 text-sm">
      Drop images or folders to import them
    </p>
  </div>
{/if}
