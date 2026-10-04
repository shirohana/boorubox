// @vitest-environment jsdom

import type { TagEntry, Vocabulary } from '@boorubox/shared'
import { vocabulary } from '$lib/api'
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks'
import { createRawSnippet, flushSync, mount, tick, unmount } from 'svelte'
import { afterEach, expect, it, vi } from 'vitest'
import PinnedTagsPanel from './PinnedTagsPanel.svelte'

const tag = (name: string, pinnedGroup: number): TagEntry => ({
  name,
  category: 'general',
  pinnedGroup,
  note: null,
})

let calls: { cmd: string, payload: unknown }[] = []

function seed() {
  vocabulary.entries = [tag('cat', 1), tag('dog', 1), tag('fox', 2)]
  vocabulary.groups = [
    { name: 'Animals', collapsed: false },
    { name: '', collapsed: false },
    { name: 'Spare', collapsed: false },
  ]
}

function setup(
  refuse: Record<string, string> = {},
  answer?: (cmd: string, payload: unknown) => Vocabulary | undefined,
  props: { startEditing?: boolean, sticky?: boolean } = { startEditing: true },
) {
  seed()
  calls = []
  mockIPC((cmd, payload) => {
    if (cmd === 'tag_suggestions') return []
    calls.push({ cmd, payload })
    if (refuse[cmd]) throw new Error(refuse[cmd])
    const custom = answer?.(cmd, payload)
    if (custom) return custom
    const groups = cmd === 'create_pinned_group'
      ? [...vocabulary.groups, { name: (payload as { name: string }).name, collapsed: false }]
      : vocabulary.groups
    return { tags: vocabulary.entries, groups } satisfies Vocabulary
  })
  const instance = mount(PinnedTagsPanel, { target: document.body, props })
  flushSync()
  return instance
}

const originalElementFromPoint = document.elementFromPoint
const originalScrollIntoView = Element.prototype.scrollIntoView

afterEach(() => {
  document.elementFromPoint = originalElementFromPoint
  Element.prototype.scrollIntoView = originalScrollIntoView
  clearMocks()
  vocabulary.entries = []
  vocabulary.groups = []
  vocabulary.error = null
  document.body.innerHTML = ''
})

const settle = async () => {
  for (let i = 0; i < 4; i++) await tick()
  await new Promise((resolve) => setTimeout(resolve, 0))
}

const button = (text: string) =>
  [...document.body.querySelectorAll('button')].find((b) => b.textContent?.trim().startsWith(text))

function tick_(name: string) {
  document.body.querySelector<HTMLElement>(`[aria-label="Select ${name}"]`)!.click()
  flushSync()
}

function openMenuOf(label: string) {
  document.body
    .querySelector(`section[aria-label="Group ${label}"] [data-group-header]`)!
    .dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true }))
}

it('shows the move footer only while a tag is checked', () => {
  const instance = setup()
  expect(button('Move')).toBeUndefined()

  tick_('cat')
  tick_('dog')

  expect(button('Move 2 selected to')).toBeDefined()
  unmount(instance)
})

/** Opens the "Move n selected to…" Select; jsdom has no pointer capture, which bits-ui releases on pointerdown. */
async function openMoveSelect() {
  Element.prototype.hasPointerCapture ??= () => false
  button('Move')!.dispatchEvent(
    new PointerEvent('pointerdown', { bubbles: true, cancelable: true, button: 0, pointerType: 'mouse' }),
  )
  await settle()
}

const options = () => [...document.body.querySelectorAll<HTMLElement>('[role="option"]')]

async function choose(option: HTMLElement) {
  option.dispatchEvent(new PointerEvent('pointermove', { bubbles: true, pointerType: 'mouse' }))
  option.dispatchEvent(
    new PointerEvent('pointerup', { bubbles: true, cancelable: true, button: 0, pointerType: 'mouse' }),
  )
  await settle()
}

const optionText = () => options().map((item) => item.textContent?.trim())

it('moves the checked tags to the chosen group with one placeMany call, then clears the checks', async () => {
  const instance = setup()
  tick_('cat')
  tick_('dog')

  await openMoveSelect()
  expect(optionText()).toEqual(['#2', 'Spare', 'New group…'])
  await choose(options().find((item) => item.textContent?.includes('#2'))!)

  expect(calls).toEqual([
    { cmd: 'move_pinned_tags', payload: { names: ['cat', 'dog'], target: { group: 2 } } },
  ])
  expect(button('Move')).toBeUndefined()
  expect(document.body.querySelector('[aria-label="Select cat"]')!.getAttribute('aria-checked')).toBe('false')
  unmount(instance)
})

it('creates the named group, then moves the checked tags into the new last group', async () => {
  const instance = setup()
  tick_('cat')

  await openMoveSelect()
  await choose(options().find((item) => item.textContent?.includes('New group'))!)
  const field = document.body.querySelector<HTMLInputElement>('input[aria-label="New group name"]')!
  field.value = 'Fresh'
  field.dispatchEvent(new Event('input', { bubbles: true }))
  flushSync()
  document.body.querySelector<HTMLFormElement>('form')!.requestSubmit()
  await settle()

  expect(calls).toEqual([
    { cmd: 'create_pinned_group', payload: { name: 'Fresh' } },
    { cmd: 'move_pinned_tags', payload: { names: ['cat'], target: { group: 4 } } },
  ])
  expect(document.body.querySelector('form')).toBeNull()
  unmount(instance)
})

it('shows the refusal create_pinned_group answers for a blank new-group name', async () => {
  const instance = setup({ create_pinned_group: 'a group needs a name' })

  button('New group')!.click()
  flushSync()
  document.body.querySelector<HTMLFormElement>('form')!.requestSubmit()
  await settle()

  expect(calls).toEqual([{ cmd: 'create_pinned_group', payload: { name: '' } }])
  expect(document.body.textContent).toContain('a group needs a name')
  unmount(instance)
})

it('offers Delete group on an empty group only', async () => {
  const instance = setup()

  openMenuOf('Animals')
  await settle()
  expect(document.body.textContent).not.toContain('Delete group')
  document.body.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }))
  await settle()

  openMenuOf('Spare')
  await settle()
  expect(document.body.textContent).toContain('Delete group')
  unmount(instance)
})

const byLabel = (label: string) => document.body.querySelector<HTMLButtonElement>(`[aria-label="${label}"]`)!

it('moves a group by its header buttons, disabled at the ends', async () => {
  const instance = setup()

  expect(byLabel('Move Animals up').disabled).toBe(true)
  expect(byLabel('Move Spare down').disabled).toBe(true)
  expect(byLabel('Move Animals down').disabled).toBe(false)
  byLabel('Move Animals down').click()
  await settle()

  expect(calls).toEqual([{ cmd: 'move_pinned_group', payload: { from: 1, to: 2 } }])
  unmount(instance)
})

it('does not tick a tag on a right-click of its name', async () => {
  const instance = setup()
  const name = [...document.body.querySelectorAll('button')].find((b) => b.textContent?.trim() === 'cat')!

  name.dispatchEvent(Object.assign(new Event('pointerdown', { bubbles: true }), { button: 2 }))
  name.dispatchEvent(new MouseEvent('mousedown', { bubbles: true, button: 2 }))
  name.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true, button: 2 }))
  name.dispatchEvent(new MouseEvent('click', { bubbles: true, button: 2 }))
  await settle()

  expect(document.body.textContent).toContain('Unpin')
  expect(byLabel('Select cat').getAttribute('aria-checked')).toBe('false')

  name.click()
  flushSync()
  expect(byLabel('Select cat').getAttribute('aria-checked')).toBe('true')
  unmount(instance)
})

it('unpins a tag from the button at the end of its row', async () => {
  const instance = setup()

  byLabel('Unpin cat').click()
  await settle()

  expect(calls).toEqual([{ cmd: 'set_tag_pinned_group', payload: { name: 'cat', target: 'unpin' } }])
  unmount(instance)
})

function typeInto(field: HTMLInputElement, text: string) {
  field.value = text
  field.dispatchEvent(new Event('input', { bubbles: true }))
  flushSync()
}

function pressEnter(field: HTMLInputElement) {
  field.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true, cancelable: true }))
}

it('pins the typed tag into the group on Enter and keeps the field for the next', async () => {
  const instance = setup()

  byLabel('Pin a tag into Animals').click()
  await settle()
  const field = byLabel('Tag to pin') as unknown as HTMLInputElement
  typeInto(field, 'owl')
  pressEnter(field)
  await settle()

  expect(calls).toEqual([{ cmd: 'set_tag_pinned_group', payload: { name: 'owl', target: { group: 1 } } }])
  expect(byLabel('Tag to pin')).not.toBeNull()
  expect((byLabel('Tag to pin') as unknown as HTMLInputElement).value).toBe('')
  unmount(instance)
})

it('shows the refusal under the pin field and keeps the text', async () => {
  const instance = setup({ set_tag_pinned_group: 'owl is not a tag' })

  byLabel('Pin a tag into Animals').click()
  await settle()
  const field = byLabel('Tag to pin') as unknown as HTMLInputElement
  typeInto(field, 'owl')
  pressEnter(field)
  await settle()

  expect(document.body.textContent).toContain('owl is not a tag')
  expect((byLabel('Tag to pin') as unknown as HTMLInputElement).value).toBe('owl ')
  unmount(instance)
})

function pointer(type: string, x = 0, y = 0) {
  const event = new Event(type, { bubbles: true, cancelable: true })
  Object.assign(event, { clientX: x, clientY: y, pointerId: 1, button: 0 })
  return event
}

/** Drags the handle of `from`'s row onto `under`; jsdom has no layout, so the test says what is under the pointer. */
function dragTagOnto(from: string, under: Element) {
  const handle = document.body
    .querySelector(`[data-tag-row="${from}"] [data-tag-handle]`)!
    .firstElementChild as HTMLElement
  handle.setPointerCapture = () => {}
  handle.releasePointerCapture = () => {}
  document.elementFromPoint = () => under
  handle.dispatchEvent(pointer('pointerdown'))
  handle.dispatchEvent(pointer('pointermove', 20, 20))
  flushSync()
  const ringed = under.hasAttribute('data-drop-over')
  handle.dispatchEvent(pointer('pointerup', 20, 20))
  return ringed
}

const dragTagTo = (from: string, label: string) =>
  dragTagOnto(from, document.body.querySelector(`section[aria-label="Group ${label}"]`)!)

it('drags a ticked tag onto another group and moves every ticked tag', async () => {
  const instance = setup()
  tick_('cat')
  tick_('dog')

  expect(dragTagTo('cat', 'Spare')).toBe(true)
  await settle()

  expect(calls).toEqual([
    { cmd: 'move_pinned_tags', payload: { names: ['cat', 'dog'], target: { group: 3 } } },
  ])
  unmount(instance)
})

it('drags an unticked tag alone, and a drop on its own group writes nothing', async () => {
  const instance = setup()
  tick_('dog')

  dragTagTo('cat', 'Animals')
  await settle()
  expect(calls).toEqual([])

  dragTagTo('cat', 'Spare')
  await settle()
  expect(calls).toEqual([
    { cmd: 'move_pinned_tags', payload: { names: ['cat'], target: { group: 3 } } },
  ])
  unmount(instance)
})

it('a drop on New group… opens the naming prompt for the dragged tags and writes nothing yet', async () => {
  const instance = setup()
  tick_('cat')
  tick_('dog')

  expect(dragTagOnto('cat', document.body.querySelector('[data-new-group]')!)).toBe(true)
  await settle()

  expect(calls).toEqual([])
  expect(document.body.querySelector('input[aria-label="New group name"]')).not.toBeNull()
  expect(button('Create and move 2')).toBeDefined()
  unmount(instance)
})

it('a drop outside every section writes nothing', async () => {
  const instance = setup()

  expect(dragTagOnto('cat', document.body)).toBe(false)
  await settle()

  expect(calls).toEqual([])
  expect(document.body.querySelector('input[aria-label="New group name"]')).toBeNull()
  unmount(instance)
})

it('Escape in the pin field closes it and does not reach the document', async () => {
  const instance = setup()
  const reached = vi.fn()
  document.addEventListener('keydown', reached)

  byLabel('Pin a tag into Animals').click()
  await settle()
  byLabel('Tag to pin').dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true, cancelable: true }))
  await settle()

  expect(reached).not.toHaveBeenCalled()
  expect(document.body.querySelector('[aria-label="Tag to pin"]')).toBeNull()
  document.removeEventListener('keydown', reached)
  unmount(instance)
})

it('closing the pin field drops its refusal, so a later refused write shows at the bottom', async () => {
  const instance = setup({ set_tag_pinned_group: 'owl is not a tag', move_pinned_group: 'cannot move' })

  byLabel('Pin a tag into Animals').click()
  await settle()
  const field = byLabel('Tag to pin') as unknown as HTMLInputElement
  typeInto(field, 'owl')
  pressEnter(field)
  await settle()
  field.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true, cancelable: true }))
  await settle()

  byLabel('Move Animals down').click()
  await settle()

  expect(document.body.textContent).toContain('cannot move')
  unmount(instance)
})

it('keeps the pin field in the group the tag landed in when the pin empties an unnamed group', async () => {
  const compacted: Vocabulary = {
    tags: [tag('cat', 1), tag('dog', 1), tag('fox', 2)],
    groups: [
      { name: 'Animals', collapsed: false },
      { name: 'Spare', collapsed: false },
    ],
  }
  const instance = setup({}, (cmd) => (cmd === 'set_tag_pinned_group' ? compacted : undefined))

  byLabel('Pin a tag into Spare').click()
  await settle()
  const field = byLabel('Tag to pin') as unknown as HTMLInputElement
  typeInto(field, 'fox')
  pressEnter(field)
  await settle()

  const open = document.body.querySelectorAll('[aria-label="Tag to pin"]')
  expect(open).toHaveLength(1)
  expect(document.body.querySelector('section[aria-label="Group Spare"] [aria-label="Tag to pin"]')).not.toBeNull()
  unmount(instance)
})

it('pins once when Enter is pressed twice while the first pin is in flight', async () => {
  const instance = setup()

  byLabel('Pin a tag into Animals').click()
  await settle()
  const field = byLabel('Tag to pin') as unknown as HTMLInputElement
  typeInto(field, 'owl')
  pressEnter(field)
  pressEnter(field)
  await settle()

  expect(calls.filter((call) => call.cmd === 'set_tag_pinned_group')).toHaveLength(1)
  unmount(instance)
})

it('renders barEnd inside the bar, which is a sibling of the scroll region', () => {
  seed()
  mockIPC((cmd) => (cmd === 'tag_suggestions' ? [] : { tags: vocabulary.entries, groups: vocabulary.groups }))
  mount(PinnedTagsPanel, {
    target: document.body,
    props: {
      startEditing: true,
      barEnd: createRawSnippet(() => ({ render: () => '<button data-done>Done</button>' })),
    },
  })
  flushSync()
  const bar = document.body.querySelector('[data-bar]')!
  const groupsRegion = document.body.querySelector('[data-groups]')!
  expect(bar.querySelector('[data-done]')).not.toBeNull()
  expect(groupsRegion.contains(bar)).toBe(false)
  expect(bar.parentElement).toBe(groupsRegion.parentElement)
})

it('rests the focus on the groups when an unpin removes the pressed button, and keeps it on a button that stays', async () => {
  const instance = setup({}, (cmd) =>
    cmd === 'set_tag_pinned_group' ? { tags: [tag('dog', 1), tag('fox', 2)], groups: vocabulary.groups } : undefined)
  const groupsRegion = document.body.querySelector('[data-groups]')

  byLabel('Move Animals down').focus()
  byLabel('Move Animals down').click()
  await settle()
  expect(document.activeElement).toBe(byLabel('Move Animals down'))

  byLabel('Unpin cat').focus()
  byLabel('Unpin cat').click()
  await settle()
  expect(document.body.querySelector('[data-tag-row="cat"]')).toBeNull()
  expect(document.activeElement).toBe(groupsRegion)
  unmount(instance)
})

it('reports the pointer to the edge scroller on every move and stops it on release', async () => {
  const pointer_ = await import('$lib/components/common/pointer-drag')
  const at = vi.fn()
  const stop = vi.fn()
  vi.spyOn(pointer_, 'edgeScroller').mockReturnValue({ at, stop })
  const instance = setup()
  const handle = document.body
    .querySelector('[data-tag-row="cat"] [data-tag-handle]')!
    .firstElementChild as HTMLElement
  handle.setPointerCapture = () => {}
  handle.releasePointerCapture = () => {}
  document.elementFromPoint = () => document.body
  handle.dispatchEvent(pointer('pointerdown'))
  handle.dispatchEvent(pointer('pointermove', 20, 290))
  expect(at).toHaveBeenCalledWith(20, 290)
  handle.dispatchEvent(pointer('pointerup', 20, 290))
  expect(stop).toHaveBeenCalled()
  vi.restoreAllMocks()
  unmount(instance)
})

const reading = () => setup({}, undefined, {})

it('reads by default: no field, checkbox or Unpin, and an Edit button', () => {
  const instance = reading()

  expect(document.body.querySelector('input')).toBeNull()
  expect(document.body.querySelector('[role="checkbox"]')).toBeNull()
  expect(document.body.querySelector('[aria-label="Unpin cat"]')).toBeNull()
  expect(document.body.querySelector('[data-tag-handle]')).toBeNull()
  expect(document.body.querySelector('[aria-label="Move Animals down"]')).toBeNull()
  expect(document.body.querySelector('[aria-label="Pin a tag into Animals"]')).toBeNull()
  expect(document.body.textContent).toContain('cat')
  expect(button('Edit')!.getAttribute('aria-pressed')).toBe('false')
  expect(button('New group')).toBeUndefined()
  unmount(instance)
})

it('Edit shows the controls, a second press hides them and clears a tick', () => {
  const instance = reading()

  button('Edit')!.click()
  flushSync()
  expect(byLabel('Name of group 1')).not.toBeNull()
  expect(byLabel('Unpin cat')).not.toBeNull()
  tick_('cat')
  expect(button('Move 1 selected')).toBeDefined()

  button('Edit')!.click()
  flushSync()
  expect(document.body.querySelector('input')).toBeNull()
  expect(button('Move')).toBeUndefined()
  button('Edit')!.click()
  flushSync()
  expect(byLabel('Select cat').getAttribute('aria-checked')).toBe('false')
  unmount(instance)
})

it('startEditing mounts editing', () => {
  const instance = setup({}, undefined, { startEditing: true })

  expect(button('Edit')!.getAttribute('aria-pressed')).toBe('true')
  expect(byLabel('Select cat')).not.toBeNull()
  unmount(instance)
})

it('folds a group through set_pinned_group_collapsed and lists no tags while folded', async () => {
  const folded = (): Vocabulary => ({
    tags: vocabulary.entries,
    groups: [{ name: 'Animals', collapsed: true }, vocabulary.groups[1], vocabulary.groups[2]],
  })
  const instance = setup({}, (cmd) => (cmd === 'set_pinned_group_collapsed' ? folded() : undefined))

  expect(byLabel('Fold Animals').getAttribute('aria-expanded')).toBe('true')
  byLabel('Fold Animals').click()
  await settle()

  expect(calls).toEqual([{ cmd: 'set_pinned_group_collapsed', payload: { position: 1, collapsed: true } }])
  const section = document.body.querySelector('section[aria-label="Group Animals"]')!
  expect(section.querySelector('[data-tag-row]')).toBeNull()
  expect(section.textContent).toContain('· 2')
  expect(byLabel('Unfold Animals').getAttribute('aria-expanded')).toBe('false')
  expect(section.hasAttribute('data-group-position')).toBe(true)
  unmount(instance)
})

it('folding the group whose pin field is open closes the field', async () => {
  const instance = setup({}, (cmd) => (cmd === 'set_pinned_group_collapsed'
    ? { tags: vocabulary.entries, groups: [{ name: 'Animals', collapsed: true }, vocabulary.groups[1], vocabulary.groups[2]] }
    : undefined))

  byLabel('Pin a tag into Animals').click()
  await settle()
  expect(document.body.querySelector('[aria-label="Tag to pin"]')).not.toBeNull()
  byLabel('Fold Animals').click()
  await settle()

  expect(document.body.querySelector('[aria-label="Tag to pin"]')).toBeNull()
  unmount(instance)
})

/** Animals full, Birds empty, Spare holding fox; a group move answers the groups swapped. */
function setupBesideEmpty() {
  const animals = { name: 'Animals', collapsed: false }
  const birds = { name: 'Birds', collapsed: false }
  const spare = { name: 'Spare', collapsed: false }
  const instance = setup({}, (cmd, payload) => {
    if (cmd !== 'move_pinned_group') return undefined
    const { to } = payload as { from: number, to: number }
    return to === 2
      ? { tags: [tag('cat', 2), tag('dog', 2), tag('fox', 3)], groups: [birds, animals, spare] }
      : { tags: [tag('cat', 1), tag('dog', 1), tag('fox', 3)], groups: [animals, birds, spare] }
  })
  vocabulary.entries = [tag('cat', 1), tag('dog', 1), tag('fox', 3)]
  vocabulary.groups = [animals, birds, spare]
  flushSync()
  return instance
}

it('keeps the pressed arrow the same node when a move swaps a full group with an empty one', async () => {
  const instance = setupBesideEmpty()
  const down = byLabel('Move Animals down')
  expect(document.body.querySelector('section[aria-label="Group Birds"]')!.textContent).toContain('No tags')

  down.focus()
  down.click()
  await settle()

  expect(document.body.querySelector('section[aria-label="Group Birds"]')!.getAttribute('data-group-position')).toBe('1')
  expect(down.isConnected).toBe(true)
  expect(document.activeElement).toBe(down)

  const up = byLabel('Move Animals up')
  up.focus()
  up.click()
  await settle()

  expect(document.body.querySelector('section[aria-label="Group Animals"]')!.getAttribute('data-group-position')).toBe('1')
  expect(document.activeElement).toBe(up)
  unmount(instance)
})

it('restores focus without scrolling', async () => {
  const instance = setup()
  const focus = vi.spyOn(HTMLElement.prototype, 'focus')

  byLabel('Move Animals down').focus()
  focus.mockClear()
  byLabel('Move Animals down').click()
  await settle()

  expect(focus).toHaveBeenCalledWith({ preventScroll: true })
  focus.mockRestore()
  unmount(instance)
})

/** Right-clicks a row's name; bits-ui renders the menu in a portal under `body`. */
async function openRowMenu(name: string) {
  document.body
    .querySelector(`[data-tag-row="${name}"]`)!
    .dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true, button: 2 }))
  await settle()
}

const menuItems = () => [...document.body.querySelectorAll<HTMLElement>('[role="menuitem"]')]

it('offers the tag menu on a row, with one Unpin and no Manage pinned tags', async () => {
  for (const props of [{ startEditing: true }, {}]) {
    const instance = setup({}, undefined, props)
    await openRowMenu('cat')

    const items = menuItems().map((item) => item.textContent?.trim())
    expect(items).toContain('Open Danbooru wiki')
    expect(items).toContain('Edit note…')
    expect(items).toContain('New group above')
    expect(items).toContain('Move to Spare')
    expect(items.filter((item) => item === 'Unpin')).toHaveLength(1)
    expect(items).not.toContain('Manage pinned tags…')
    unmount(instance)
    document.body.innerHTML = ''
  }
})

it('opens the note dialog from the row menu', async () => {
  const instance = setup()
  await openRowMenu('cat')
  menuItems().find((item) => item.textContent?.trim() === 'Edit note…')!.click()
  await settle()

  expect(document.body.querySelector('[role="dialog"]')).not.toBeNull()
  unmount(instance)
})

it('closes the pin field when a row menu move renumbers the groups', async () => {
  const split: Vocabulary = {
    tags: [tag('cat', 1), tag('dog', 2), tag('fox', 3)],
    groups: [
      { name: '', collapsed: false },
      { name: 'Animals', collapsed: false },
      { name: '', collapsed: false },
      { name: 'Spare', collapsed: false },
    ],
  }
  const instance = setup({}, (cmd) => (cmd === 'set_tag_pinned_group' ? split : undefined))
  byLabel('Pin a tag into Spare').click()
  await settle()
  typeInto(byLabel('Tag to pin') as unknown as HTMLInputElement, 'owl')

  await openRowMenu('cat')
  menuItems().find((item) => item.textContent?.trim() === 'New group above')!.click()
  await settle()

  expect(calls).toEqual([{ cmd: 'set_tag_pinned_group', payload: { name: 'cat', target: { newGroupAt: 1 } } }])
  expect(document.body.querySelector('[aria-label="Tag to pin"]')).toBeNull()
  unmount(instance)
})

it('shows a tag\'s note on its row with the whole note in title, in both modes', () => {
  for (const props of [{ startEditing: true }, {}]) {
    const instance = setup({}, undefined, props)
    vocabulary.entries = vocabulary.entries.map((entry) =>
      entry.name === 'cat' ? { ...entry, note: 'a long note about cats' } : entry)
    flushSync()

    const note = document.body.querySelector('[data-tag-row="cat"] [title]')!
    expect(note.textContent?.trim()).toBe('a long note about cats')
    expect(note.getAttribute('title')).toBe('a long note about cats')
    expect(document.body.querySelector('[data-tag-row="dog"] [title]')).toBeNull()
    unmount(instance)
  }
})

const groupIndex = () => document.body.querySelector<HTMLElement>('nav[aria-label="Groups"]')

it('shows no group index at one group', () => {
  const instance = setup()
  vocabulary.groups = [{ name: 'Animals', collapsed: false }]
  vocabulary.entries = [tag('cat', 1)]
  flushSync()

  expect(groupIndex()).toBeNull()
  unmount(instance)
})

it('lists every group by label in the index and scrolls the clicked one into view', () => {
  const instance = setup()
  const scrolled = vi.fn()
  Element.prototype.scrollIntoView = function (this: Element, options?: boolean | ScrollIntoViewOptions) {
    scrolled(this, options)
  }

  const entries = [...groupIndex()!.querySelectorAll('button')]
  expect(entries.map((entry) => entry.textContent?.trim())).toEqual(['Animals', '#2', 'Spare'])
  entries[2].click()

  expect(scrolled).toHaveBeenCalledTimes(1)
  expect(scrolled.mock.calls[0][0]).toBe(document.body.querySelector('section[aria-label="Group Spare"]'))
  expect(scrolled.mock.calls[0][1]).toEqual({ block: 'start' })
  unmount(instance)
})

it('pins the index and the bar to a scrolling host with sticky', () => {
  for (const sticky of [true, false]) {
    const instance = setup({}, undefined, { sticky })
    expect(groupIndex()!.classList.contains('sticky')).toBe(sticky)
    expect(document.body.querySelector('[data-bar]')!.classList.contains('sticky')).toBe(sticky)
    unmount(instance)
    document.body.innerHTML = ''
  }
})
