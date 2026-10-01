// @vitest-environment jsdom

import type { TagEntry, Vocabulary } from '@boorubox/shared'
import { vocabulary } from '$lib/api'
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks'
import { flushSync, mount, tick, unmount } from 'svelte'
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
  const instance = mount(PinnedTagsPanel, { target: document.body })
  flushSync()
  return instance
}

const originalElementFromPoint = document.elementFromPoint

afterEach(() => {
  document.elementFromPoint = originalElementFromPoint
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
