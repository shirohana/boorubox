// @vitest-environment jsdom

import type { TagEntry, Vocabulary } from '@boorubox/shared'
import { vocabulary } from '$lib/api'
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks'
import { flushSync, mount, tick, unmount } from 'svelte'
import { afterEach, expect, it } from 'vitest'
import PinnedGroupsDialog from './PinnedGroupsDialog.svelte'

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

function setup(refuse: Record<string, string> = {}) {
  seed()
  calls = []
  mockIPC((cmd, payload) => {
    calls.push({ cmd, payload })
    if (refuse[cmd]) throw new Error(refuse[cmd])
    const groups = cmd === 'create_pinned_group'
      ? [...vocabulary.groups, { name: (payload as { name: string }).name, collapsed: false }]
      : vocabulary.groups
    return { tags: vocabulary.entries, groups } satisfies Vocabulary
  })
  const instance = mount(PinnedGroupsDialog, {
    target: document.body,
    props: { open: true, onclose: () => {} },
  })
  flushSync()
  return instance
}

afterEach(() => {
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
    .querySelector(`section[aria-label="Group ${label}"]`)!
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
  expect(document.body.textContent).toContain('Move up')
  expect(document.body.textContent).not.toContain('Delete group')
  document.body.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }))
  await settle()

  openMenuOf('Spare')
  await settle()
  expect(document.body.textContent).toContain('Delete group')
  unmount(instance)
})
