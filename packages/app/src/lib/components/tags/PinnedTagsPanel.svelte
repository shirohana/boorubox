<script lang="ts">
  // The pinned tags, managed in one place (`pinned-tags-panel` design D2). One
  // panel behind two doors, the dialog and the Settings page; it never knows which
  // one it is in. It has two modes (`pinned-tags-manage` D1): reading, where a
  // group is its fold toggle and label and its tags are plain coloured names, and
  // editing. The host picks the mode it opens in (the page reads, the dialog opened
  // by "Manage pinned tags…" edits) and the Edit button in the bar toggles it.
  // Editing names groups and moves them by their header buttons, pins a tag into a
  // group, unpins one, drags tags between groups, moves several ticked tags at
  // once, starts a new group and deletes an empty one. Reads and writes the
  // vocabulary store, whose every answer replaces the groups and their tags
  // together, so the sections below cannot show a name on the wrong row after a
  // compaction.
  //
  // The host bounds the panel's height: the groups scroll inside their own region
  // and the bar (the Edit toggle, the move control, "New group…", the host's
  // `barEnd`) sits outside it. With `sticky` the group index and the bar are sticky
  // with the page background so they also hold when the host is a scrolling page,
  // where the groups region has no bound of its own. The index (one button per
  // group, shown from two groups on) scrolls its group into view.
  //
  // A tag is dragged by its handle with `pointerDrag`, never HTML5 drag (see
  // `api/drag-drop.ts`). No `<label>` wraps a row's checkbox: a label toggles
  // on the press that opens a context menu, and a right-click must never tick.
  import ArrowDownIcon from '@lucide/svelte/icons/arrow-down'
  import ArrowUpIcon from '@lucide/svelte/icons/arrow-up'
  import ChevronDownIcon from '@lucide/svelte/icons/chevron-down'
  import ChevronRightIcon from '@lucide/svelte/icons/chevron-right'
  import PencilIcon from '@lucide/svelte/icons/pencil'
  import PinOffIcon from '@lucide/svelte/icons/pin-off'
  import PlusIcon from '@lucide/svelte/icons/plus'
  import Trash2Icon from '@lucide/svelte/icons/trash-2'
  import { tick, type Snippet } from 'svelte'
  import { vocabulary } from '$lib/api'
  import type { ArtistDialogRequest } from '$lib/components/artists/artist-dialog'
  import ArtistDialog from '$lib/components/artists/ArtistDialog.svelte'
  import { moveItem } from '$lib/components/common/reorder'
  import { edgeScroller, pointerDrag } from '$lib/components/common/pointer-drag'
  import ReorderHandle from '$lib/components/common/ReorderHandle.svelte'
  import { Button } from '$lib/components/ui/button'
  import { Checkbox } from '$lib/components/ui/checkbox'
  import * as ContextMenu from '$lib/components/ui/context-menu'
  import { Input } from '$lib/components/ui/input'
  import * as Select from '$lib/components/ui/select'
  import { KEY_ENTER, KEY_ESCAPE } from '$lib/keyboard'
  import { CATEGORY_TEXT_CLASS } from './categories'
  import TagInput from './TagInput.svelte'
  import TagNoteDialog from './TagNoteDialog.svelte'
  import TagVocabularyMenuItems from './TagVocabularyMenuItems.svelte'

  interface Props {
    /**
     * The host scrolls as a whole (the Settings page): pin the index to its top and the
     * bar to its bottom. Off in the dialog: there the host bounds the panel and both are
     * already outside the scroll region, and WebKit misplaces a sticky box inside the
     * dialog's centring transform (`pinned-tags-manage` D8).
     */
    sticky?: boolean
    /** Where menus and the move list portal: the viewer's `<dialog>` when opened from inside it. */
    portalTo?: Element
    /** Open in edit mode; the toggle in the bar still flips it. */
    startEditing?: boolean
    /** Rendered at the bar's end after "New group…", e.g. the dialog's Done. */
    barEnd?: Snippet
  }

  let { portalTo, barEnd, sticky = false, startEditing = false }: Props = $props()

  // The prop seeds the mode once; the host does not drive it afterwards.
  // svelte-ignore state_referenced_locally
  let editing = $state(startEditing)

  const NEW_GROUP = 'new'

  // `drafts`, `renameErrors` and `adding` are keyed by position. They are cleared
  // whenever the group count changes (a pin, an unpin or a move that empties an
  // unnamed group renumbers the rest), whoever wrote: the row menu's moves go to
  // the store, not through `write`. A group move renumbers without changing the
  // count and clears them itself.
  /** What was typed into a group's name field and not yet written, by position. */
  let drafts = $state<Record<number, string>>({})
  /** A refusal of a group's rename, by position. */
  let renameErrors = $state<Record<number, string>>({})
  /** Tags ticked for a move; a tag that stops being pinned drops out of {@link selected}. */
  let ticked = $state<string[]>([])
  /** The new-group prompt: `names` are the tags it moves into the group it creates. */
  let naming = $state<{ names: string[] } | null>(null)
  let newName = $state('')
  let newError = $state<string | null>(null)
  /** The move Select's value, reset after each choice so choosing the same group again fires. */
  let moveChoice = $state('')
  /** The group whose pin field is open, one at a time; `null` is none. */
  let adding = $state<number | null>(null)
  let addText = $state('')
  let addError = $state<string | null>(null)
  /** A pin is in flight; a second one waits for the first's answer. */
  let pinning = false
  let addField = $state<ReturnType<typeof TagInput> | null>(null)
  /** A tag drag in flight: the row held and every tag that goes with it. */
  let dragging = $state<{ tag: string, names: string[] } | null>(null)
  /** What the pointer is over while dragging: a group's position, or the new-group button. */
  let over = $state<number | typeof NEW_GROUP | null>(null)

  /** The tag `TagNoteDialog` is open for, snapshotted when "Edit note…" is chosen. */
  let editingNote = $state<string | null>(null)
  let editingArtist = $state<ArtistDialogRequest | null>(null)

  let root = $state<HTMLElement | null>(null)
  let groupsRegion = $state<HTMLElement | null>(null)

  const groups = $derived(vocabulary.pinnedGroups)
  const groupCount = $derived(vocabulary.groups.length)
  // Pre: the stale positions must be gone before the rows render under the new numbering.
  $effect.pre(() => {
    void groupCount
    clearPositionKeyed()
  })
  const selected = $derived(ticked.filter((tag) => vocabulary.isPinned(tag)))
  /** The one group every selected tag already sits in, which a move into would do nothing. */
  const sharedGroup = $derived.by(() => {
    const positions = new Set(selected.map((tag) => vocabulary.groupOf(tag)))
    return positions.size === 1 ? [...positions][0] : null
  })
  const targets = $derived(
    groups.map((_, index) => index + 1).filter((position) => position !== sharedGroup),
  )

  function nameOf(position: number): string {
    return drafts[position] ?? vocabulary.groups[position - 1]?.name ?? ''
  }

  async function commitName(position: number) {
    const draft = drafts[position]
    if (draft === undefined) return
    if (draft === vocabulary.groups[position - 1]?.name) {
      delete drafts[position]
      return
    }
    const landed = await write(() => vocabulary.renameGroup(position, draft))
    if (landed) {
      delete drafts[position]
      delete renameErrors[position]
    } else {
      renameErrors[position] = vocabulary.error ?? 'The group could not be renamed'
    }
  }

  function toggleEditing() {
    editing = !editing
    if (editing) return
    ticked = []
    naming = null
    clearPositionKeyed()
  }

  function closeAdd() {
    adding = null
    addError = null
  }

  function clearPositionKeyed() {
    drafts = {}
    renameErrors = {}
    closeAdd()
  }

  /**
   * Runs a vocabulary write. Focus that was inside the panel stays on its element, or rests on
   * the groups region when the write removed it: left alone, the dialog's focus scope would
   * send it to the first tabbable, a group's fold button.
   */
  async function write<T>(change: () => Promise<T>): Promise<T> {
    const focused = root?.contains(document.activeElement) ? document.activeElement : null
    const result = await change()
    if (focused) {
      await tick()
      const kept = focused.isConnected && !(focused as HTMLButtonElement).disabled
      // A restore says where the next key goes, never what is in view: the reader's scroll
      // position is where they pressed, and a focus() that scrolls would throw it away.
      ;(kept ? (focused as HTMLElement) : groupsRegion)?.focus({ preventScroll: true })
    }
    return result
  }

  /** `scrollIntoView` scrolls every scrolling ancestor, so one call serves the dialog's region and the page's scroller. */
  function showGroup(position: number) {
    groupsRegion
      ?.querySelector(`[data-group-position="${position}"]`)
      ?.scrollIntoView({ block: 'start' })
  }

  const deleteGroup = (position: number) => write(() => vocabulary.deleteGroup(position))

  function moveGroup(from: number, to: number) {
    if (moveItem(groups, from, to) === groups) return
    clearPositionKeyed()
    void write(() => vocabulary.moveGroup(from + 1, to + 1))
  }

  const toggleFold = (position: number, collapsed: boolean) => {
    if (!collapsed && adding === position) closeAdd()
    return write(() => vocabulary.setGroupCollapsed(position, !collapsed))
  }

  const unpin = (tag: string) => write(() => vocabulary.place(tag, 'unpin'))

  function toggle(tag: string, on: boolean) {
    ticked = on ? [...ticked, tag] : ticked.filter((other) => other !== tag)
  }

  /** A primary click only: the press that opens the row's menu is not a toggle. */
  function toggleByClick(event: MouseEvent, tag: string) {
    if (event.button !== 0 || event.ctrlKey) return
    toggle(tag, !selected.includes(tag))
  }

  async function openAdd(position: number) {
    adding = position
    addText = ''
    addError = null
    await tick()
    addField?.focusEnd()
  }

  /**
   * Pins the typed tag into the open group and keeps the field for the next.
   * Runs after `TagInput`'s own Enter handling (which may have completed the
   * word or accepted the highlighted suggestion), so it pins what the field
   * holds. Tab and a clicked suggestion only fill the field. One pin at a time;
   * the field is emptied only if nothing was typed while the pin was in flight.
   */
  async function pinTyped() {
    const typed = addText
    const name = typed.trim()
    if (adding === null || name === '' || pinning) return
    pinning = true
    try {
      const landed = await write(() => vocabulary.place(name, { group: adding! }))
      if (!landed) {
        addError = vocabulary.error
        return
      }
      addError = null
      if (addText === typed) addText = ''
      const group = vocabulary.groupOf(name)
      if (group !== null && group !== adding) {
        adding = group
        await tick()
        addField?.focusEnd()
      }
    } finally {
      pinning = false
    }
  }

  /** Enter bubbles here after `TagInput` has handled it; Escape must not reach the dialog's own Escape listener. */
  function onAddKeydown(event: KeyboardEvent) {
    if (event.key === KEY_ENTER) void pinTyped()
    else if (event.key === KEY_ESCAPE) event.stopPropagation()
  }

  function closeAddIfEmpty() {
    if (addText.trim() === '') closeAdd()
  }

  async function placeAndUntick(names: string[], position: number) {
    await write(() => vocabulary.placeMany(names, { group: position }))
    if (!vocabulary.error) ticked = ticked.filter((tag) => !names.includes(tag))
  }

  function moveTo(names: string[], value: string) {
    if (value === NEW_GROUP) openNaming(names)
    else void placeAndUntick(names, Number(value))
  }

  function openNaming(names: string[]) {
    naming = { names }
    newName = ''
    newError = null
  }

  async function createNamed() {
    if (!naming) return
    const { names } = naming
    if (!(await vocabulary.createGroup(newName))) {
      newError = vocabulary.error
      return
    }
    if (names.length > 0) {
      await placeAndUntick(names, vocabulary.groups.length)
    }
    naming = null
  }

  function targetAt(x: number, y: number): typeof over {
    const hit = document.elementFromPoint(x, y)
    if (hit?.closest('[data-new-group]')) return NEW_GROUP
    const section = hit?.closest<HTMLElement>('[data-group-position]')
    return section ? Number(section.dataset.groupPosition) : null
  }

  /** The tags a drag of `tag` moves: the ticked ones when it is among them, else it alone. */
  const draggedNames = (tag: string) => (selected.includes(tag) ? selected : [tag])

  function dropDrag(x: number, y: number, dropped: boolean) {
    const held = dragging
    const target = targetAt(x, y)
    dragging = null
    over = null
    if (!held || !dropped || target === null) return
    if (target === NEW_GROUP) openNaming(held.names)
    else if (target !== vocabulary.groupOf(held.tag)) void placeAndUntick(held.names, target)
  }

  let pointer = { x: 0, y: 0 }

  /** The ring follows what scrolls under the held pointer. */
  const scroller = edgeScroller(() => {
    if (dragging) over = targetAt(pointer.x, pointer.y)
  })

  function tagDrag(node: HTMLElement) {
    const drag = pointerDrag(
      node,
      {
        onstart(_x, _y, handle) {
          const tag = handle.closest<HTMLElement>('[data-tag-row]')?.dataset.tagRow
          if (tag) dragging = { tag, names: draggedNames(tag) }
        },
        onmove: (x, y) => {
          if (!dragging) return
          pointer = { x, y }
          over = targetAt(x, y)
          scroller.at(x, y)
        },
        onend: (x, y, dropped) => {
          scroller.stop()
          dropDrag(x, y, dropped)
        },
      },
      { selector: '[data-tag-handle]' },
    )
    return {
      destroy() {
        scroller.stop()
        drag.destroy()
      },
    }
  }
</script>

{#snippet note(tag: string)}
  {@const text = vocabulary.noteOf(tag)}
  <!-- The whole note is in the title: the line truncates it, and the list has a tag per row. -->
  {#if text !== null}
    <span class="min-w-0 flex-1 truncate text-muted-foreground" title={text}>{text}</span>
  {/if}
{/snippet}

<div bind:this={root} class="flex min-h-0 flex-1 flex-col gap-3">
  <!-- One entry would be a control that does nothing. -->
  {#if groups.length > 1}
    <nav
      aria-label="Groups"
      data-group-index
      class="flex flex-wrap gap-1 {sticky ? 'sticky top-0 z-10 bg-background pb-2' : ''}"
    >
      {#each groups as _, index (index)}
        <Button
          type="button"
          variant="ghost"
          size="xs"
          onclick={() => showGroup(index + 1)}
        >
          {vocabulary.labelOf(index + 1)}
        </Button>
      {/each}
    </nav>
  {/if}
  <div
    bind:this={groupsRegion}
    tabindex="-1"
    class="flex min-h-0 flex-1 flex-col gap-3 overflow-y-auto pr-1 outline-none"
    data-groups
    use:tagDrag
  >
    {#each groups as group, index (index)}
      {@const position = index + 1}
      {@const label = vocabulary.labelOf(position)}
      <section
        aria-label="Group {label}"
        data-group-position={position}
        data-drop-over={over === position ? '' : undefined}
        class="
          flex flex-col gap-1.5 rounded-md border border-border p-2
          data-drop-over:ring-1 data-drop-over:ring-primary
        "
      >
        {#snippet header(props: Record<string, unknown>)}
          <div {...props} data-group-header class="flex items-center gap-1.5">
            <Button
              type="button"
              variant="ghost"
              size={editing ? 'icon-xs' : 'xs'}
              aria-expanded={!group.collapsed}
              aria-label="{group.collapsed ? 'Unfold' : 'Fold'} {label}"
              onclick={() => void toggleFold(position, group.collapsed)}
            >
              {#if group.collapsed}
                <ChevronRightIcon />
              {:else}
                <ChevronDownIcon />
              {/if}
              {#if !editing}{label}{/if}
            </Button>
            {#if editing}
              <Input
                value={nameOf(position)}
                autocomplete="off"
                placeholder="#{position}"
                aria-label="Name of group {position}"
                aria-invalid={renameErrors[position] ? true : undefined}
                oninput={(event) => (drafts[position] = event.currentTarget.value)}
                onblur={() => void commitName(position)}
                onkeydown={(event) => {
                  if (event.key !== KEY_ENTER) return
                  event.preventDefault()
                  void commitName(position)
                }}
              />
            {/if}
            {#if group.collapsed}
              <!-- Beside the name field the count has no room of its own and would wrap. -->
              <span class="
                shrink-0 text-[10px] whitespace-nowrap text-muted-foreground tabular-nums
              ">
                · {group.tags.length}
              </span>
            {/if}
            {#if editing}
              <Button
                type="button"
                variant="ghost"
                size="icon-xs"
                aria-label="Move {label} up"
                disabled={index === 0}
                onclick={() => moveGroup(index, index - 1)}
              >
                <ArrowUpIcon />
              </Button>
              <Button
                type="button"
                variant="ghost"
                size="icon-xs"
                aria-label="Move {label} down"
                disabled={index === groups.length - 1}
                onclick={() => moveGroup(index, index + 1)}
              >
                <ArrowDownIcon />
              </Button>
              <Button
                type="button"
                variant="ghost"
                size="icon-xs"
                aria-label="Pin a tag into {label}"
                onclick={() => void openAdd(position)}
              >
                <PlusIcon />
              </Button>
            {/if}
          </div>
        {/snippet}
        <!--
          One header element per position, whatever the group holds: the menu's trigger is
          disabled rather than the menu conditional, so a move that swaps an empty group with a
          full one at this index keeps the pressed button in the DOM and `write()` can restore
          focus to it.
        -->
        <ContextMenu.Root>
          <ContextMenu.Trigger disabled={!editing || group.tags.length > 0}>
            {#snippet child({ props })}
              {@render header(props)}
            {/snippet}
          </ContextMenu.Trigger>
          <ContextMenu.Content portalProps={{ to: portalTo }}>
            <ContextMenu.Item variant="destructive" onSelect={() => void deleteGroup(position)}>
              <Trash2Icon />
              Delete group
            </ContextMenu.Item>
          </ContextMenu.Content>
        </ContextMenu.Root>
        {#if renameErrors[position]}
          <p class="text-xs text-destructive">{renameErrors[position]}</p>
        {/if}
        {#if group.collapsed}
          <!-- Folded: the header's count stands for the tags. -->
        {:else if group.tags.length === 0}
          <p class="px-1 text-xs text-muted-foreground">No tags</p>
        {:else}
          <ul role="list" class="flex flex-col gap-1">
            {#each group.tags as tag (tag)}
              <li>
                <ContextMenu.Root>
                  <ContextMenu.Trigger>
                    {#snippet child({ props })}
                      <div
                        {...props}
                        data-tag-row={tag}
                        class="
                          flex items-center gap-1.5 px-1 text-xs
                          {dragging?.names.includes(tag) ? 'opacity-50' : ''}
                        "
                      >
                        {#if editing}
                          <span data-tag-handle class="inline-flex"><ReorderHandle /></span>
                          <Checkbox
                            checked={selected.includes(tag)}
                            onCheckedChange={(on) => toggle(tag, on)}
                            aria-label="Select {tag}"
                          />
                          <button
                            type="button"
                            class="
                              min-w-0 cursor-pointer truncate text-left
                              {CATEGORY_TEXT_CLASS[vocabulary.categoryOf(tag)]}
                            "
                            onclick={(event) => toggleByClick(event, tag)}
                          >
                            {tag}
                          </button>
                          {@render note(tag)}
                          <Button
                            type="button"
                            variant="ghost"
                            size="icon-xs"
                            class="ml-auto"
                            aria-label="Unpin {tag}"
                            onclick={() => void unpin(tag)}
                          >
                            <PinOffIcon />
                          </Button>
                        {:else}
                          <span
                            class="
                              min-w-0 truncate
                              {CATEGORY_TEXT_CLASS[vocabulary.categoryOf(tag)]}
                            "
                          >
                            {tag}
                          </span>
                          {@render note(tag)}
                        {/if}
                      </div>
                    {/snippet}
                  </ContextMenu.Trigger>
                  <ContextMenu.Content portalProps={{ to: portalTo }}>
                    <TagVocabularyMenuItems
                      name={tag}
                      oneditnote={(name) => (editingNote = name)}
                      oneditartist={(name) =>
                      (editingArtist = { mode: 'edit', tag: name, adapter: null, pageUrl: null })}
                      onmanagegroups={null}
                    />
                  </ContextMenu.Content>
                </ContextMenu.Root>
              </li>
            {/each}
          </ul>
        {/if}
        {#if editing && !group.collapsed && adding === position}
          <div
            role="presentation"
            class="flex flex-col gap-1"
            onfocusout={closeAddIfEmpty}
            onkeydown={onAddKeydown}
          >
            <TagInput
              bind:this={addField}
              bind:value={addText}
              label="Tag to pin"
              placeholder="Tag name"
              oninput={() => (addError = null)}
              onescape={closeAdd}
            />
            {#if addError}
              <p class="text-xs text-destructive">{addError}</p>
            {/if}
          </div>
        {/if}
      </section>
    {/each}
  </div>

  {#if vocabulary.error && !addError}
    <p class="text-xs text-destructive">{vocabulary.error}</p>
  {/if}

  {#if editing && naming}
    <form
      class="flex flex-col gap-2"
      onsubmit={(event) => {
        event.preventDefault()
        void createNamed()
      }}
    >
      <Input
        bind:value={newName}
        autocomplete="off"
        autofocus
        placeholder="Group name"
        aria-label="New group name"
      />
      {#if newError}
        <p class="text-xs text-destructive">{newError}</p>
      {/if}
      <div class="flex justify-end gap-2">
        <Button type="button" variant="ghost" onclick={() => (naming = null)}>Cancel</Button>
        <Button type="submit">
          {naming.names.length > 0 ? `Create and move ${naming.names.length}` : 'Create'}
        </Button>
      </div>
    </form>
  {:else}
    <div
      data-bar
      class="flex flex-wrap items-center justify-end gap-2 pt-2 {sticky
        ? `sticky bottom-0 bg-background`
        : ''}"
    >
      <Button
        type="button"
        variant={editing ? 'default' : 'outline'}
        aria-pressed={editing}
        onclick={toggleEditing}
      >
        <PencilIcon />
        Edit
      </Button>
      {#if editing && selected.length > 0}
        <Select.Root
          type="single"
          bind:value={moveChoice}
          onValueChange={(value) => {
            moveChoice = ''
            void moveTo(selected, value)
          }}
        >
          <Select.Trigger aria-label="Move selected tags to">
            Move {selected.length} selected to…
          </Select.Trigger>
          <Select.Content portalProps={{ to: portalTo }}>
            {#each targets as position (position)}
              <Select.Item value={String(position)} label={vocabulary.labelOf(position)} />
            {/each}
            <Select.Item value={NEW_GROUP} label="New group…" />
          </Select.Content>
        </Select.Root>
      {/if}
      <span class="mr-auto"></span>
      {#if editing}
        <Button
          type="button"
          variant="outline"
          data-new-group
          data-drop-over={over === NEW_GROUP ? '' : undefined}
          class="data-drop-over:ring-1 data-drop-over:ring-primary"
          onclick={() => openNaming([])}
        >
          New group…
        </Button>
      {/if}
      {@render barEnd?.()}
    </div>
  {/if}
</div>

<!--
  Beside the menus, never inside one: a menu's content unmounts on select and would
  take its dialog with it. Both portal to `portalTo` (or `body`), so from the dialog
  door they open over the pinned-tags dialog and Escape closes the top one first.
-->
<TagNoteDialog
  open={editingNote !== null}
  name={editingNote ?? ''}
  {portalTo}
  onclose={() => (editingNote = null)}
/>
<!--
  The dialog never re-reads the vocabulary, so `onsaved` does: a rename moves the tag's
  pin and note to the new name, and the rows read both. The browse search behind the
  dialog door is not re-run (this panel has no handle on it): its results show the old
  name until their next read.
-->
<ArtistDialog
  open={editingArtist !== null}
  request={editingArtist}
  {portalTo}
  onclose={() => (editingArtist = null)}
  onsaved={() => void vocabulary.refresh()}
/>
