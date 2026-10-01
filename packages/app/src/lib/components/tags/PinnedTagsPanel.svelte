<script lang="ts">
  // The pinned tags, managed in one place (`pinned-tags-panel` design D2): name
  // a group, move groups by their header buttons, pin a tag into a group, unpin
  // one, drag tags between groups, move several ticked tags at once, start a
  // new group, delete an empty one. One panel behind two doors, the dialog and
  // the settings page; it never knows which one it is in. Reads and writes the
  // vocabulary store, whose every answer replaces the groups and their tags
  // together, so the sections below cannot show a name on the wrong row after a
  // compaction.
  //
  // The host bounds the panel's height: the groups scroll inside their own region
  // and the bar (the move control, "New group…", the host's `barEnd`) sits outside
  // it. The bar is sticky with the page background so it also holds when the host
  // is a scrolling page, where the groups region has no bound of its own.
  //
  // A tag is dragged by its handle with `pointerDrag`, never HTML5 drag (see
  // `api/drag-drop.ts`). No `<label>` wraps a row's checkbox: a label toggles
  // on the press that opens a context menu, and a right-click must never tick.
  import ArrowDownIcon from '@lucide/svelte/icons/arrow-down'
  import ArrowUpIcon from '@lucide/svelte/icons/arrow-up'
  import PinOffIcon from '@lucide/svelte/icons/pin-off'
  import PlusIcon from '@lucide/svelte/icons/plus'
  import Trash2Icon from '@lucide/svelte/icons/trash-2'
  import { tick, type Snippet } from 'svelte'
  import { vocabulary } from '$lib/api'
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

  interface Props {
    /**
     * Pin the bar to the bottom of a scrolling host (the Settings page). Off in the
     * dialog: there the host bounds the panel and the bar is already outside the
     * scroll region, and WebKit misplaces a sticky box inside the dialog's
     * centring transform.
     */
    stickyBar?: boolean
    /** Where menus and the move list portal: the viewer's `<dialog>` when opened from inside it. */
    portalTo?: Element
    /** Rendered at the bar's end after "New group…", e.g. the dialog's Done. */
    barEnd?: Snippet
  }

  let { portalTo, barEnd, stickyBar = false }: Props = $props()

  const NEW_GROUP = 'new'

  // `drafts`, `renameErrors` and `adding` are keyed by position. Every write goes
  // through `write`, which clears them when the group count changed (a pin, an
  // unpin or a move that empties an unnamed group renumbers the rest); a group
  // move renumbers without changing the count and clears them itself.
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

  let root = $state<HTMLElement | null>(null)
  let groupsRegion = $state<HTMLElement | null>(null)

  const groups = $derived(vocabulary.pinnedGroups)
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
   * Runs a vocabulary write, then drops the position-keyed state if the groups were renumbered.
   * Focus that was inside the panel stays on its element, or rests on the groups region when
   * the write removed it: left alone, the dialog's focus scope would send it to the first
   * tabbable, a group's name field.
   */
  async function write<T>(change: () => Promise<T>): Promise<T> {
    const before = vocabulary.groups.length
    const focused = root?.contains(document.activeElement) ? document.activeElement : null
    const result = await change()
    if (vocabulary.groups.length !== before) clearPositionKeyed()
    if (focused) {
      await tick()
      const kept = focused.isConnected && !(focused as HTMLButtonElement).disabled
      ;(kept ? (focused as HTMLElement) : groupsRegion)?.focus()
    }
    return result
  }

  const deleteGroup = (position: number) => write(() => vocabulary.deleteGroup(position))

  function moveGroup(from: number, to: number) {
    if (moveItem(groups, from, to) === groups) return
    clearPositionKeyed()
    void write(() => vocabulary.moveGroup(from + 1, to + 1))
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

<div bind:this={root} class="flex min-h-0 flex-1 flex-col gap-3">
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
          </div>
        {/snippet}
        {#if group.tags.length === 0}
          <ContextMenu.Root>
            <ContextMenu.Trigger>
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
        {:else}
          {@render header({})}
        {/if}
        {#if renameErrors[position]}
          <p class="text-xs text-destructive">{renameErrors[position]}</p>
        {/if}
        {#if group.tags.length === 0}
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
                        <span data-tag-handle class="inline-flex"><ReorderHandle /></span>
                        <Checkbox
                          checked={selected.includes(tag)}
                          onCheckedChange={(on) => toggle(tag, on)}
                          aria-label="Select {tag}"
                        />
                        <button
                          type="button"
                          class="
                            min-w-0 flex-1 cursor-pointer truncate text-left
                            {CATEGORY_TEXT_CLASS[vocabulary.categoryOf(tag)]}
                          "
                          onclick={(event) => toggleByClick(event, tag)}
                        >
                          {tag}
                        </button>
                        <Button
                          type="button"
                          variant="ghost"
                          size="icon-xs"
                          aria-label="Unpin {tag}"
                          onclick={() => void unpin(tag)}
                        >
                          <PinOffIcon />
                        </Button>
                      </div>
                    {/snippet}
                  </ContextMenu.Trigger>
                  <ContextMenu.Content portalProps={{ to: portalTo }}>
                    <ContextMenu.Item onSelect={() => void unpin(tag)}>
                      <PinOffIcon />
                      Unpin
                    </ContextMenu.Item>
                  </ContextMenu.Content>
                </ContextMenu.Root>
              </li>
            {/each}
          </ul>
        {/if}
        {#if adding === position}
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

  {#if naming}
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
      class="flex flex-wrap items-center justify-end gap-2 pt-2 {stickyBar
        ? `sticky bottom-0 bg-background`
        : ''}"
    >
      {#if selected.length > 0}
        <Select.Root
          type="single"
          bind:value={moveChoice}
          onValueChange={(value) => {
            moveChoice = ''
            void moveTo(selected, value)
          }}
        >
          <Select.Trigger aria-label="Move selected tags to" class="mr-auto">
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
      {@render barEnd?.()}
    </div>
  {/if}
</div>
