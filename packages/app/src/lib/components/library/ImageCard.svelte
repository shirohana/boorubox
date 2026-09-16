<script lang="ts">
  import type { ImageRecord, Rating } from '@boorubox/shared'
  import ArchiveRestoreIcon from '@lucide/svelte/icons/archive-restore'
  import BookmarkIcon from '@lucide/svelte/icons/bookmark'
  import CloudUploadIcon from '@lucide/svelte/icons/cloud-upload'
  import Trash2Icon from '@lucide/svelte/icons/trash-2'
  import type { ClickModifiers, Selection } from '$lib/api'
  import { booruSites, collections } from '$lib/api'
  import { postedNames } from '$lib/components/booru/posted'
  import { RATING_COLOUR, RATINGS } from '$lib/components/tags/ratings'
  import { Button } from '$lib/components/ui/button'
  import { Checkbox } from '$lib/components/ui/checkbox'
  import * as ContextMenu from '$lib/components/ui/context-menu'
  import { ratingLabel } from '$lib/domain/format'
  import type { CollectionTarget } from './collection-actions'
  import CollectionMenuItems from './CollectionMenuItems.svelte'
  import { cachedThumbnail, thumbnail } from './thumbnail-cache'
  import type { TilePress } from './tile-click'
  import { shouldActivate, travelled } from './tile-click'
  import type { TrashActions } from './trash-actions'

  interface Props {
    /** `undefined` while the page holding this row is still loading. */
    image: ImageRecord | undefined
    /** The grid's current card: it holds the tab stop and shows its overlay. */
    focused: boolean
    /** In the selection the next bulk action reads — true for an unloaded row too. */
    selected: boolean
    /** The card took the focus by pointer or by Tab; the keys move it elsewhere. */
    onfocus: () => void
    /** A click on the tile, with what it was held down with (spec `selection`). */
    onselect: (modifiers: ClickModifiers) => void
    onactivate: () => void
    /** Slot Grid · tile: rates THIS image, not the inspector's (design D17). */
    onrate: (rating: Rating | null) => void
    /** Which pair of trash actions this tile offers (`trash` design D13). */
    view: 'library' | 'trash'
    /** They act on THIS image, like the rating entries above them. */
    actions: TrashActions
    /**
     * The menu's collection submenu acts on the whole selection when this
     * tile is part of it, and on this image alone otherwise (design D8) — so
     * it needs the store itself, not just the boolean `selected` already
     * carries.
     */
    selection: Selection
    /** A collection write from this menu, for the caller's own `replaceMany` (design D8/D10). */
    onwritten: (records: ImageRecord[]) => void
    /**
     * "New collection…" was chosen on this tile: the grid raises the one
     * dialog it owns for the whole grid, with what this tile's menu acts on.
     * Not a dialog of this tile's own — the grid mounts and destroys tiles as
     * it scrolls, and one `Dialog` per tile is machinery on that path.
     */
    onnewcollection: (target: CollectionTarget) => void
    /** Reported like every other action failure on this screen. */
    onerror: (message: string) => void
  }

  let {
    image,
    focused,
    selected,
    onfocus,
    onselect,
    onactivate,
    onrate,
    view,
    actions,
    selection,
    onwritten,
    onerror,
    onnewcollection,
  }: Props = $props()

  let src = $state<string | null>(null)
  let previewFailed = $state(false)

  const title = $derived(image?.pageTitle || image?.imageUrl || image?.id || '')
  const capturedOn = $derived(image ? new Date(image.capturedAt).toLocaleDateString() : '')

  /**
   * `posted-label`: the mark says the image is posted at a glance and names the
   * site on hover, like the rating badge beside it. The names come from the
   * store rather than a prop — the list belongs to the library, not to the row
   * the grid handed this tile.
   */
  const posted = $derived(image ? postedNames(image.posts, booruSites.sites) : [])
  const postedTitle = $derived(posted.join(', '))

  /**
   * Selected outranks focused, and a card that is both keeps the focus visible
   * through the offset — a placeholder inside a range carries the same mark, so
   * a selection larger than what has loaded is drawn honestly.
   */
  const ring = $derived.by(() => {
    if (!selected) return focused ? 'ring-3 ring-ring ring-offset-2 ring-offset-background' : ''
    return focused
      ? 'ring-3 ring-primary ring-offset-2 ring-offset-background'
      : 'ring-3 ring-primary'
  })

  /**
   * The press this tile is in, or `null` between presses. It carries what the
   * click cannot read for itself: the tile's current-ness before the press
   * moved it, and where the pointer started (design D7).
   */
  let press: TilePress | null = null

  /**
   * Derived, not read inside the effect: `image` is a fresh record on every
   * result refresh, and an effect keyed on it would reset the preview each time.
   */
  const previewId = $derived(image && !image.missing ? image.id : null)

  /** This tile's own collections, for the submenu's checkmark (design D8). */
  const collectionMemberships = $derived(image ? new Set(image.collections) : null)

  /**
   * Names for the tile's mark (`browse-feedback` design D6): a membership
   * whose collection is gone — a placeholder in a rebuilt library — shows as
   * its bare id rather than dropping out of the title silently.
   */
  const collectionNames = $derived(
    image ? image.collections.map((id) => collections.byId(id)?.name ?? id) : [],
  )

  /**
   * Ids: the selection's when the tile is in it, else this image's alone
   * (design D8) — resolved at write time so a live range is never read from a
   * stale array kept from before the menu opened.
   */
  async function resolveCollectionIds(): Promise<string[]> {
    if (!image) return []
    return selected ? selection.ids() : [image.id]
  }

  /** The trigger, for the card's tab stop the closed menu hands the focus to. */
  let tile = $state<HTMLDivElement | null>(null)
  /** The menu's own element while it is open, to tell its focus from another control's. */
  let menu = $state<HTMLElement | null>(null)

  /**
   * Where the focus goes when the menu closes. bits-ui returns it to the
   * trigger only when the trigger is tabbable, and this one is a
   * `tabindex="-1"` div; failing that, to whatever was focused before the menu
   * opened, which a right-click never set. Either way the focus landed
   * outside the grid, where none of its keys are bound, and the arrows, Space
   * and `i` were dead after every menu action. The card's own tab stop takes
   * it instead; `focusin` then makes this tile current, as a click would.
   *
   * Only when the focus is orphaned — on `<body>`, or still on an item of the
   * menu that is going away. A menu closed by a click elsewhere is closed on
   * the pointer's release, after the mousedown has already focused what was
   * clicked: another tile, the search field. That focus is the user's and
   * stays; pulling it back here made a right-click on one tile and a click
   * on the next land on the first again between the two.
   */
  function onmenuclose(event: Event) {
    const active = document.activeElement
    const orphaned = !active || active === document.body || menu?.contains(active)
    if (!orphaned) return
    event.preventDefault()
    tile?.querySelector<HTMLElement>('[data-card-focus]')?.focus()
  }

  const collectionTarget: CollectionTarget = {
    resolveIds: resolveCollectionIds,
    memberships: () => collectionMemberships,
    onwritten: (records) => onwritten(records),
    onerror: (message) => onerror(message),
  }

  $effect(() => {
    const id = previewId
    previewFailed = false
    const cached = id ? cachedThumbnail(id) : null
    src = cached
    if (!id || cached) return

    let current = true
    void thumbnail(id)
      .then((url) => {
        if (current) src = url
      })
      .catch(() => {
        if (current) previewFailed = true
      })
    return () => {
      current = false
    }
  })
</script>

<!--
  Roving tabindex: exactly one card in the grid carries `tabindex="0"`, so Tab
  lands on the current card and the grid finds the element to focus by querying
  for it (`data-card-focus`). `onfocusin` rather than `onfocus` on the tile,
  because on a missing-file card the tab stop is its own button.
-->
<ContextMenu.Root
  onOpenChange={(open) => {
    // A right-click on a tile outside the selection makes it current, as a
    // left click would (design D8) — the menu's ids follow from `selected`
    // either way, but the tile the user opened the menu on is the one the
    // rest of the screen should agree is current.
    if (open && image && !selected) onselect({})
  }}
>
  <ContextMenu.Trigger class="group/tile relative aspect-square">
    {#snippet child({ props })}
      <!-- A class written here is replaced: {...props} carries the trigger's merged class. -->
      <div bind:this={tile} onfocusin={onfocus} {...props}>
        {#if !image}
          <div class="h-full rounded-lg border border-border bg-muted/40 {ring}">
            <span class="sr-only">Loading</span>
          </div>
        {:else if image.missing}
          <!--
      `trash` design D6: the file may come back — an unmounted volume, a sync
      client catching up, a rename undone — and the spec requires the image to
      render again when it does. So the action here moves the record to the
      trash, which is reversible and asks nothing, rather than destroying it.
      In the trash the record is already there, and the way back is the same
      Restore the tile menu offers.
    -->
          <div
            class="
              flex h-full flex-col justify-between gap-2 overflow-hidden rounded-lg border
              border-dashed border-destructive/40 bg-destructive/5 p-3
              {ring}
            "
          >
            <div class="min-h-0">
              <p class="text-xs font-medium text-destructive">File not found</p>
              <p class="mt-1 line-clamp-3 text-xs break-all text-muted-foreground">{title}</p>
            </div>

            <!--
              The card's tab stop: a missing tile has no image button, so this
              is the element the grid's roving focus lands on, in both views.
            -->
            <Button
              size="xs"
              variant="outline"
              data-card-focus
              tabindex={focused ? 0 : -1}
              onclick={() => (view === 'trash' ? actions.restore([image.id]) : actions.trash([image.id]))}
            >
              {view === 'trash' ? 'Restore' : 'Move to trash'}
            </Button>
          </div>
        {:else}
          <button
            type="button"
            data-card-focus
            tabindex={focused ? 0 : -1}
            onpointerdown={(event) => {
              // The press, not the click, is where the tile's current-ness can
              // still be read: the context-menu trigger around this button is
              // `tabindex="-1"` and WebKit focuses a tabbable element on
              // mousedown, so `focusin` has already made this tile current by
              // the time the click fires (design D7).
              press = { x: event.clientX, y: event.clientY, wasCurrent: focused }
            }}
            onclick={(event) => {
              // WebKit does not focus a button on click, and the grid's keys only
              // fire while the focus is inside it — so a clicked card has to take
              // the focus itself, or a click and then an arrow key does nothing.
              event.currentTarget.focus()
              const modifiers = { multi: event.metaKey || event.ctrlKey, range: event.shiftKey }
              onselect(modifiers)
              // Design D7: the second click on the tile the inspector is already
              // describing opens it — and only if the pointer stayed still, since
              // a press the user dragged out of is a gesture they abandoned.
              if (shouldActivate(press, { x: event.clientX, y: event.clientY, ...modifiers })) {
                onactivate()
              }
            }}
            ondblclick={(event) => {
              // A double click opens whatever the tile was, but a press that
              // travelled is still not a click (design D7).
              if (!press || !travelled(press, { x: event.clientX, y: event.clientY })) onactivate()
            }}
            class="
              group relative block size-full overflow-hidden rounded-lg border border-border
              bg-muted/40 outline-none
              {ring}
            "
          >
            {#if src}
              <!--
                Undraggable (design D1): an `<img>` the webview promises to the
                OS starts a drag the import dropzone then reads as an incoming
                file, raising the overlay over the user's own grid.
              -->
              <img
                {src}
                alt={title}
                draggable="false"
                loading="lazy"
                decoding="async"
                class="size-full object-contain"
                onerror={() => (previewFailed = true)}
              />
            {:else}
              <span
                class="
                  flex size-full items-center justify-center px-2 text-center text-xs
                  text-muted-foreground
                "
              >
                {previewFailed ? 'No preview' : ''}
              </span>
            {/if}

            <!--
        Rated tiles only (spec `rating`): a badge for the absence of a rating is
        noise on the majority of a fresh library. The colour is what is read at
        a glance; the letter is for anyone who cannot tell them apart.
      -->
            {#if image.rating || posted.length > 0 || collectionNames.length > 0}
              <span class="absolute top-1 left-1 flex gap-1">
                {#if image.rating}
                  <span
                    class="
                      flex size-5 items-center justify-center rounded-md text-xs font-semibold
                      uppercase shadow-sm
                      {RATING_COLOUR[image.rating]}
                    "
                    title={ratingLabel(image.rating)}
                  >
                    {image.rating}
                  </span>
                {/if}

                {#if posted.length > 0}
                  <span
                    class="
                      flex size-5 items-center justify-center rounded-md bg-background/80 shadow-sm
                    "
                    title={postedTitle}
                  >
                    <CloudUploadIcon class="size-3.5" />
                    <span class="sr-only">{postedTitle}</span>
                  </span>
                {/if}

                {#if collectionNames.length > 0}
                  <!--
                    The mark says the image is in at least one collection
                    (`browse-feedback` design D6); the count is the inspector's.
                  -->
                  <span
                    class="
                      flex size-5 items-center justify-center rounded-md bg-background/80 shadow-sm
                    "
                    title={collectionNames.join(', ')}
                  >
                    <BookmarkIcon class="size-3.5" />
                    <span class="sr-only">{collectionNames.join(', ')}</span>
                  </span>
                {/if}
              </span>
            {/if}

            <!--
        The tile is the image (`library-browse`): its facts only ever cover it
        while it is hovered or focused, never as a caption strip below it.
      -->
            <span
              class="
                absolute inset-x-0 bottom-0 block bg-linear-to-t from-black/80 to-transparent px-2
                pt-6 pb-1.5 text-left opacity-0 transition-opacity
                group-hover:opacity-100
                group-focus-visible:opacity-100
                {focused ? 'opacity-100' : ''}
              "
            >
              <span class="block truncate text-xs text-white">{title}</span>
              <span class="block text-[0.7rem] text-white/70">{capturedOn} · {image.source}</span>
            </span>
          </button>
        {/if}

        {#if image}
          {@const trashLabel = view === 'trash' ? 'Restore' : 'Move to trash'}
          <!--
            Slot Grid · tile. Outside the tile's button rather than inside it: a
            control nested in a button is not reachable on its own. Ticking the
            box selects the tile and makes it the current card, but never opens
            it: activation is the tile's own second click.

            The trash action sits beside it because the owner reached for the
            one on the missing-file card and found every other tile had only the
            context menu (design D13, amended). It is this image, so it is one
            image: no confirmation, and it leaves the selection alone —
            though it does make its tile the current card, so the write's
            refresh returns the focus to that row and not to wherever it was.
          -->
          <span
            class="
              absolute top-1 right-1 flex items-center gap-1 transition-opacity
              group-hover/tile:opacity-100
              focus-within:opacity-100
              {selected || focused ? 'opacity-100' : 'opacity-0'}
            "
          >
            {#if !image.missing}
              <!-- The missing-file card already *is* this button, full width. -->
              <Button
                size="icon-xs"
                variant="secondary"
                class="bg-background/80"
                aria-label={trashLabel}
                title={trashLabel}
                onclick={() => {
                  // Its tile becomes the current card first (a click does not
                  // focus the button in WebKit), so the row it empties is the
                  // row the focus and the scroll come back to after the write.
                  onfocus()
                  if (view === 'trash') actions.restore([image.id])
                  else actions.trash([image.id])
                }}
              >
                {#if view === 'trash'}
                  <ArchiveRestoreIcon />
                {:else}
                  <Trash2Icon />
                {/if}
              </Button>
            {/if}

            <Checkbox
              checked={selected}
              aria-label="Select this image"
              class="bg-background/80"
              onCheckedChange={() => onselect({ multi: true })}
            />
          </span>
        {/if}
      </div>
    {/snippet}
  </ContextMenu.Trigger>

  <!-- The menu acts on this tile's image, whatever the inspector is showing. -->
  <ContextMenu.Content bind:ref={menu} onCloseAutoFocus={onmenuclose}>
    <!--
      `ContextMenu.GroupHeading` reads a `Menu.Group` context and throws
      without one (bits-ui 2.19) — the group is what makes the heading, and so
      this whole menu, openable at all.
    -->
    <ContextMenu.Group>
      <ContextMenu.GroupHeading>Rating</ContextMenu.GroupHeading>
      {#each RATINGS as rating (rating)}
        <ContextMenu.Item disabled={!image} onSelect={() => onrate(rating)}>
          {rating} · {ratingLabel(rating)}
        </ContextMenu.Item>
      {/each}
      <ContextMenu.Item disabled={!image} onSelect={() => onrate(null)}>
        none · unrated
      </ContextMenu.Item>
    </ContextMenu.Group>

    <!-- The collection submenu (`collections` design D8). -->
    <ContextMenu.Separator />
    <ContextMenu.Sub>
      <ContextMenu.SubTrigger disabled={!image}>Collections</ContextMenu.SubTrigger>
      <ContextMenu.SubContent class="max-h-(--bits-floating-available-height) overflow-y-auto">
        <CollectionMenuItems
          target={collectionTarget}
          onnew={() => onnewcollection(collectionTarget)}
        >
          {#snippet checkboxItem({ label, checked, onSelect })}
            <ContextMenu.CheckboxItem {checked} onCheckedChange={onSelect}>
              {label}
            </ContextMenu.CheckboxItem>
          {/snippet}
          {#snippet item({ label, onSelect })}
            <ContextMenu.Item {onSelect}>{label}</ContextMenu.Item>
          {/snippet}
          {#snippet separator()}
            <ContextMenu.Separator />
          {/snippet}
        </CollectionMenuItems>
      </ContextMenu.SubContent>
    </ContextMenu.Sub>

    <!--
      Slot Grid · tile (`trash` design D13), added to the menu that is already
      here rather than as a second one. The reversible action is plain; the
      irreversible one is marked, named with an ellipsis and confirmed.
    -->
    <ContextMenu.Separator />
    {#if view === 'trash'}
      <ContextMenu.Item
        disabled={!image}
        onSelect={() => {
          if (image) actions.restore([image.id])
        }}
      >
        Restore
      </ContextMenu.Item>
      <ContextMenu.Item
        variant="destructive"
        disabled={!image}
        onSelect={() => {
          if (image) actions.deleteForever([image.id])
        }}
      >
        Delete forever…
      </ContextMenu.Item>
    {:else}
      <ContextMenu.Item
        disabled={!image}
        onSelect={() => {
          if (image) actions.trash([image.id])
        }}
      >
        Move to trash
      </ContextMenu.Item>
    {/if}
  </ContextMenu.Content>
</ContextMenu.Root>
