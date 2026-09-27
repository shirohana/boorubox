// Which images the next action applies to (design D1–D4). It sits beside the
// command wrappers because a selection larger than the loaded pages can only
// name its images by asking Rust for them, and this is the one place that ask
// is made.
//
// The store owns the grid's focus as well as the selected set (design D1):
// every gesture is a function of both — a shift-arrow moves the focus AND
// rewrites the range from the anchor — and split across two owners each of
// those rules would have to be written twice.
//
// The focus, the anchor and a range are row indices, which are right between
// re-reads and wrong across one: a capture or a write moves rows under them.
// `pin` and `repin` carry them across a re-read of the same search by the
// images their rows held (`selection-by-id` design D1).

import { SvelteSet } from 'svelte/reactivity'

/**
 * The selection is only ever one of these (design D2). A set of ids is what a
 * click builds, because the tile it hit is drawn and its id is in hand; a range
 * is what "from here to there" means over a result the app has not loaded, so
 * it is indices into the current result order — re-pinned across a re-read of
 * the same search, to ids from the loaded rows or shifted past a capture's row
 * (`selection-by-id` design D2, D4), because the indices alone would name
 * whatever moved into those rows.
 */
export type SelectionState
  = | { kind: 'ids', ids: SvelteSet<string> }
  /** Half-open, over the current result order. */
    | { kind: 'range', start: number, end: number }

/** What a click was held down with, as the store reads it. */
export interface ClickModifiers {
  /** The platform's multi-select modifier: toggle this one image. */
  multi?: boolean
  /** Shift: take everything between the anchor and this card. */
  range?: boolean
  /**
   * The tile menu opening on an unselected tile (`collections` design D8):
   * current-ness only, never a stamp. `click` below reads only `multi` and
   * `range`, so a menu-open click still falls through to a plain focus here —
   * a stamp-mode router is what has to check this field before routing.
   */
  menu?: boolean
}

/**
 * Turns a row range of the current result into ids — `search_ids` in the app,
 * a stub in the tests. Injected rather than imported so the store never has to
 * know how the page builds a `SearchRequest`.
 */
export type IdResolver = (offset: number, limit: number) => Promise<string[]>

/**
 * Which of a candidate set of ids the current search still matches —
 * `matchingIds` in the app, a stub in the tests. Injected beside `IdResolver`
 * (`browse-fixes` design D1): after a write re-reads the search, this is how
 * the selection asks which of its own ids the result still shows, without
 * pulling the whole result down to intersect against it in the webview.
 */
export type MatchResolver = (ids: string[]) => Promise<string[]>

/** What a store needs of the loaded rows — `SearchResults` fits as is. */
export interface LoadedRows {
  total: number
  at: (index: number) => { id: string } | undefined
}

/**
 * The row `id` holds in the current request, or `null` when the request does
 * not match it — `searchPosition` in the app, a stub in the tests.
 */
export type PositionResolver = (id: string) => Promise<number | null>

/** An index and the image its row held when it was pinned, if that row was loaded. */
export interface Pinned { index: number, id: string | undefined }

/** Everything `repin` needs to put the store back on its images after a re-read. */
export interface SelectionPin { focus: Pinned, anchor: Pinned, state: SelectionState }

/**
 * An old row's index once rows were inserted at `positions` (new rows, sorted
 * ascending): each insertion at or before it pushes it down one. A capture
 * only inserts, so every old row keeps its order relative to the others
 * (`selection-by-id` design D4).
 */
export function shiftPast(index: number, positions: number[]): number {
  let shifted = index
  for (const position of positions) {
    if (position <= shifted) shifted++
  }
  return shifted
}

/** The row holding `id` among the loaded rows, or `-1` when none does. */
export function rowOf(rows: LoadedRows, id: string): number {
  for (let index = 0; index < rows.total; index++) {
    if (rows.at(index)?.id === id) return index
  }
  return -1
}

/**
 * Where a pinned image sits after a re-read (`selection-by-id` design D3):
 * among the loaded rows first, then by asking the search; a pin with no id
 * moves by the insertions `shiftBy` names. An image that left the result, and
 * every case with nothing to go by, keeps its index.
 */
export async function relocate(
  pinned: Pinned,
  rows: LoadedRows,
  position: PositionResolver,
  shiftBy: number[] = [],
): Promise<number> {
  if (pinned.index < 0) return pinned.index
  if (pinned.id === undefined) return shiftPast(pinned.index, shiftBy)

  const loaded = rowOf(rows, pinned.id)
  if (loaded >= 0) return loaded
  return (await position(pinned.id)) ?? pinned.index
}

function noSelection(): SelectionState {
  return { kind: 'ids', ids: new SvelteSet<string>() }
}

/** The one rule a multi-click and the checkbox share: in or out, nothing else. */
function toggleId(ids: SvelteSet<string>, id: string): void {
  if (ids.has(id)) ids.delete(id)
  else ids.add(id)
}

export class Selection {
  /** The card the arrows move and the inspector reads when nothing is selected. */
  focus = $state(-1)
  /** The card a range extends from. */
  anchor = $state(-1)

  /**
   * `SvelteSet` rather than a plain one: every gesture here replaces the whole
   * state, so the assignment would be enough today — but a `Set` inside `$state`
   * is not proxied, so the first in-place `add` anyone writes would update
   * nothing and show nothing.
   */
  #state = $state<SelectionState>(noSelection())
  /** The id state `ids()` last moved a range into: an edit tells that write from a gesture's. */
  #promoted: SelectionState | null = null
  /** The range `#shiftRange` last moved a range to: an edit re-resolves it, never yields. */
  #shifted: SelectionState | null = null
  readonly #resolve: IdResolver
  readonly #matches: MatchResolver
  readonly #position: PositionResolver

  /**
   * Bumped by every `#state` reassignment (`#setState` below) — a cheap,
   * synchronous stand-in for "which selection is this", for a reader that
   * must not call `ids()` or `peekIds()` on every re-render just to learn
   * whether the selection actually changed (the inspector's pinned-chip
   * effect, `tag-vocabulary` design D8).
   */
  generation = $state(0)

  constructor(resolve: IdResolver, matches: MatchResolver, position: PositionResolver) {
    this.#resolve = resolve
    this.#matches = matches
    this.#position = position
  }

  /** The one place `#state` is written, so `generation` cannot drift from it. */
  #setState(next: SelectionState): void {
    this.#state = next
    this.generation++
  }

  /** Exact in both representations, whatever the app has loaded (design D2). */
  get count(): number {
    const state = this.#state
    return state.kind === 'ids' ? state.ids.size : state.end - state.start
  }

  /**
   * A tile has both to hand, and needs both: a row inside a range is selected
   * before its record — and so its id — has arrived.
   */
  has(index: number, id: string | undefined): boolean {
    const state = this.#state
    return state.kind === 'range'
      ? index >= state.start && index < state.end
      : id !== undefined && state.ids.has(id)
  }

  /**
   * A deliberate move to a card — a plain arrow, a plain or multi-select click,
   * the card the viewer closed on: the focus moves and takes the anchor with
   * it, so the next shift gesture measures from here.
   */
  focusAt(index: number): void {
    this.focus = index
    this.anchor = index
  }

  /**
   * The DOM focus landed on a card: a pointer press, `Tab`, or the grid
   * catching up with a key that already moved the focus here. It moves the
   * focus and never the anchor.
   *
   * Both callers need that. WebKit focuses the tile on `mousedown`
   * (`tile-click`'s note), so this runs *before* the click that says whether
   * shift was held — anchoring here made a shift-click a range of one. And the
   * grid focuses the card a shift-arrow moved to, so anchoring here dragged the
   * anchor along behind the focus and left the range a two-card window sliding
   * down the grid instead of growing from where the user started.
   */
  focusEntered(index: number): void {
    this.focus = index
  }

  /**
   * A pointer gesture on a tile. Async because a multi-select-click over a live
   * range has to resolve that range before it can toggle one id out of it
   * (design D2); the other two paths settle immediately.
   */
  async click(
    index: number,
    id: string | undefined,
    modifiers: ClickModifiers = {},
  ): Promise<void> {
    if (modifiers.range) {
      this.extendTo(index)
      return
    }

    if (modifiers.multi) {
      // A placeholder row has no id to toggle; the range gestures are how a
      // selection reaches rows that have not loaded.
      if (id === undefined) return
      await this.#editIds(async (ids) => {
        // Design D2: the card the user was standing on joins a first
        // multi-click, the way shift-click already includes both ends —
        // once per selection, since a later multi-click finds the set no
        // longer empty. The anchor, never the focus: `focusEntered` has
        // already moved `focus` to the clicked card by the time this runs
        // (see its own doc comment), so a rule on `focus` would select the
        // clicked card twice and pick up nothing.
        if (ids.size === 0 && this.anchor >= 0 && this.anchor !== index) {
          const [anchorId] = await this.#resolve(this.anchor, 1)
          if (anchorId !== undefined) ids.add(anchorId)
        }
        toggleId(ids, id)
      })
      this.focusAt(index)
      return
    }

    // Design D1: a plain click only focuses, so looking at an image never
    // starts a selection and never replaces the toolbar's action row.
    this.focusAt(index)
    this.#setState(noSelection())
  }

  /**
   * The per-tile checkbox (design D2): it names exactly the image it is drawn
   * on, so — unlike a modifier-click — it never picks up the card the user
   * was standing on.
   */
  async toggle(index: number, id: string): Promise<void> {
    await this.#editIds((ids) => toggleId(ids, id))
    this.focusAt(index)
  }

  /** Shift-arrow, shift-click, shift-`Home`/`End`: the range from the anchor to here. */
  extendTo(index: number): void {
    const anchor = this.anchor < 0 ? index : this.anchor
    this.anchor = anchor
    this.focus = index
    this.#setState({
      kind: 'range',
      start: Math.min(anchor, index),
      end: Math.max(anchor, index) + 1,
    })
  }

  /**
   * The whole current result, at the cost of two numbers: nothing is resolved
   * and nothing is fetched, so the count is right the instant it is asked for
   * over any library size (design D3).
   */
  selectAll(total: number): void {
    if (total <= 0) return
    this.anchor = this.focus < 0 ? 0 : this.focus
    this.#setState({ kind: 'range', start: 0, end: total })
  }

  /**
   * Takes one image out of the selection and leaves the rest, whatever
   * representation it was in: the inspector's strip drops a thumbnail this way,
   * where the tile that would have been ⌘-clicked may be on a page the grid has
   * scrolled past. A range resolves first, as every other action on ids does
   * (design D3).
   */
  async remove(id: string): Promise<void> {
    if (this.count === 0) return
    await this.#editIds((ids) => {
      ids.delete(id)
    })
  }

  /**
   * After a write that re-read the search (`browse-fixes` design D1): keep
   * only the ids the search still matches, so the count, the strip and the
   * next bulk action describe only images the result still shows. This asks
   * the search itself, which is the only thing that knows which of a bulk
   * edit's ids left the result. An empty selection makes no round trip.
   */
  async keepMatching(): Promise<void> {
    if (this.count === 0) return
    await this.#editIds(async (ids) => {
      /* eslint-disable-next-line svelte/prefer-svelte-reactivity --
         A local index for the membership test below, built and dropped inside
         this call: nothing reads it again. */
      const kept = new Set(await this.#matches([...ids]))
      for (const id of [...ids]) {
        if (!kept.has(id)) ids.delete(id)
      }
    })
  }

  /**
   * The one way a gesture changes which ids are selected: resolve first, edit
   * the resolved set, and leave the store in id mode (design D4). `edit` may
   * itself await — the anchor pickup and `keepMatching` both make a second
   * round trip inside it — so the before/after guard is checked only once
   * `edit` has settled, covering every await it made and not just the
   * resolve (`browse-fixes` design D1 risk: a gesture during the round trip
   * wins).
   */
  async #editIds(edit: (ids: SvelteSet<string>) => void | Promise<void>): Promise<void> {
    let before = this.#state
    let resolved = await this.ids()
    // A capture's re-read moved the range under the resolve: its old rows now
    // name other images, and the moved range is still the user's selection.
    while (this.#state !== before && this.#state === this.#shifted) {
      before = this.#state
      resolved = await this.ids()
    }
    const ids = new SvelteSet(resolved)
    await edit(ids)
    // An `Esc` or a new query during the resolve or the edit owns the
    // selection now, and writing this edit back would resurrect what the
    // user cleared. `ids()` itself moves a range into id mode on return,
    // which is the one other state a resolve may leave behind.
    const after = this.#state
    if (after !== before && after !== this.#promoted) return
    this.#setState({ kind: 'ids', ids })
  }

  /** Leaves the focus alone: `Esc` must not lose the card the arrows move. */
  clear(): void {
    this.#setState(noSelection())
  }

  /** A new query: the old indices name different images, and so does the focus. */
  reset(): void {
    this.focus = -1
    this.anchor = -1
    this.clear()
  }

  /**
   * Before a re-read of the same search: which images the focus, the anchor
   * and the selection are on, read off the rows loaded now — the one record
   * of the order the indices were taken in, since the database already
   * answers in the new one (`selection-by-id` design D3). No round trip: a
   * range whose rows are all loaded becomes their ids here, and one that
   * reaches past them stays a range for `repin` to move.
   */
  pin(rows: LoadedRows): SelectionPin {
    const pinned = (index: number): Pinned => ({
      index,
      id: index >= 0 ? rows.at(index)?.id : undefined,
    })
    this.#promoteLoaded(rows)
    // Read back after the promotion, never the literal (the proxy rule in
    // `ids()`): `repin` compares it against `#state`.
    return { focus: pinned(this.focus), anchor: pinned(this.anchor), state: this.#state }
  }

  /**
   * After the re-read: each pinned index goes to its image's new row, unless a
   * gesture made during the re-read already moved it. `inserted` names the
   * captures the re-read picked up; a range `pin` could not promote follows
   * their rows (`selection-by-id` design D4). Returns their positions in the
   * new result, sorted, so the caller can move its own pins by the same shift.
   */
  async repin(pin: SelectionPin, rows: LoadedRows, inserted?: string[]): Promise<number[]> {
    const positions = inserted ? await this.#positionsOf(inserted) : []
    const [focus, anchor] = await Promise.all([
      relocate(pin.focus, rows, this.#position, positions),
      relocate(pin.anchor, rows, this.#position, positions),
    ])
    if (this.focus === pin.focus.index) this.focus = focus
    if (this.anchor === pin.anchor.index) this.anchor = anchor
    if (inserted) await this.#shiftRange(pin.state, positions, inserted)
    return positions
  }

  /** A range whose every row is loaded, as ids — the same promotion `ids()` makes. */
  #promoteLoaded(rows: LoadedRows): void {
    const state = this.#state
    if (state.kind !== 'range') return

    const ids: string[] = []
    for (let index = state.start; index < state.end; index++) {
      const id = rows.at(index)?.id
      if (id === undefined) return
      ids.push(id)
    }
    this.#setState({ kind: 'ids', ids: new SvelteSet(ids) })
    this.#promoted = this.#state
  }

  /** The rows `ids` landed on in the current request, sorted; ids it does not match drop out. */
  async #positionsOf(ids: string[]): Promise<number[]> {
    const positions = await Promise.all(ids.map((id) => this.#position(id)))
    return positions
      .filter((position): position is number => position !== null)
      .sort((a, b) => a - b)
  }

  /**
   * A range over rows that were not loaded, across captures at `positions`:
   * shifted with no round trip when none landed inside it, so a select-all
   * stays two numbers; resolved minus the captures when one did.
   */
  async #shiftRange(state: SelectionState, positions: number[], inserted: string[]): Promise<void> {
    if (state.kind !== 'range' || this.#state !== state || positions.length === 0) return

    const start = shiftPast(state.start, positions)
    const last = shiftPast(state.end - 1, positions)
    if (!positions.some((position) => position >= start && position <= last)) {
      this.#setState({ kind: 'range', start, end: last + 1 })
      this.#shifted = this.#state
      return
    }

    const resolved = await this.#resolve(start, last - start + 1)
    // A gesture during the round trip owns the selection now, as in `ids()`.
    if (this.#state !== state) return
    /* eslint-disable-next-line svelte/prefer-svelte-reactivity --
       A local index for the filter below, dropped with this call. */
    const captures = new Set(inserted)
    this.#setState({
      kind: 'ids',
      ids: new SvelteSet(resolved.filter((id) => !captures.has(id))),
    })
    this.#promoted = this.#state
  }

  /**
   * Up to `limit` ids for the inspector's thumbnail strip, drawn from what the
   * grid already holds (design D7): `at` answers `undefined` for a row whose
   * page has not arrived, and those rows are simply not drawn. The count above
   * the strip is the number that has to be right, not the strip's length.
   */
  previewIds(limit: number, at: (index: number) => string | undefined): string[] {
    const state = this.#state
    if (state.kind === 'ids') return [...state.ids].slice(0, limit)

    const shown: string[] = []
    const end = Math.min(state.end, state.start + limit)
    for (let index = state.start; index < end; index++) {
      const id = at(index)
      if (id !== undefined) shown.push(id)
    }
    return shown
  }

  /**
   * The ids every command takes. Resolving a range is one call, and it leaves
   * the store in id mode: ids are what survives the refresh an action ends with
   * (design D4), where an index range would silently re-point at whatever moved
   * into those rows.
   *
   * This is the one call that writes state — a reader that must not (the
   * inspector's pinned-chip fill, `tag-vocabulary` design D8) uses `peekIds()`
   * below instead.
   */
  async ids(): Promise<string[]> {
    const state = this.#state
    if (state.kind === 'ids') return [...state.ids]

    const resolved = await this.#resolve(state.start, state.end - state.start)
    // A gesture during the round trip owns the selection now; the action that
    // asked still gets the ids it asked about.
    if (this.#state === state) {
      this.#setState({ kind: 'ids', ids: new SvelteSet(resolved) })
      // Read back, never the literal: the webview's `$state` hands out a proxy
      // of what was assigned, so only two reads of the field compare equal.
      // Vitest compiles runes for the server, where there is no proxy, which
      // is why a test cannot catch the literal being stored here.
      this.#promoted = this.#state
    }
    return resolved
  }

  /**
   * The ids, without moving the store into id mode: a read for display only
   * (the inspector's pinned-chip fill, `tag-vocabulary` design D8), never for
   * an action. `ids()` promotes a range to id mode as a side effect of
   * resolving it, which is right for an action but wrong for a read made
   * inside a tracking scope: the promotion reassigns `#state`, which
   * invalidates that same scope's own dependency on it and reruns it — and it
   * would resolve a plain select-all's ids the moment its chips draw, which
   * is exactly what `selection-and-bulk` D3 restricts to "only when an action
   * needs ids". `peekIds()` resolves the same range and stops there.
   */
  async peekIds(): Promise<string[]> {
    const state = this.#state
    if (state.kind === 'ids') return [...state.ids]

    return this.#resolve(state.start, state.end - state.start)
  }
}
