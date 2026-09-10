// @vitest-environment jsdom

import { expect, it } from 'vitest'
import { isInDialog, isTrashKey, isTypingTarget, KEY_BACKSPACE, KEY_DELETE } from './keyboard'

/** The guard reads `event.target`, so every case has to dispatch a real event. */
function keydownOn(element: Element, key = 'i'): KeyboardEvent {
  const event = new KeyboardEvent('keydown', { key, bubbles: true })
  element.dispatchEvent(event)
  return event
}

function mounted<T extends Element>(element: T): T {
  document.body.append(element)
  return element
}

it('guards a text input', () => {
  expect(isTypingTarget(keydownOn(mounted(document.createElement('input'))))).toBe(true)
})

it('guards a textarea', () => {
  expect(isTypingTarget(keydownOn(mounted(document.createElement('textarea'))))).toBe(true)
})

it('guards a select, where a letter jumps between its options', () => {
  expect(isTypingTarget(keydownOn(mounted(document.createElement('select'))))).toBe(true)
})

it('guards a contenteditable element and anything inside it', () => {
  const editable = mounted(document.createElement('div'))
  editable.setAttribute('contenteditable', 'true')
  const inner = document.createElement('span')
  editable.append(inner)

  expect(isTypingTarget(keydownOn(editable))).toBe(true)
  expect(isTypingTarget(keydownOn(inner))).toBe(true)
})

it('does not guard a contenteditable="false" island inside an editable region', () => {
  const editable = mounted(document.createElement('div'))
  editable.setAttribute('contenteditable', 'true')
  const island = document.createElement('span')
  island.setAttribute('contenteditable', 'false')
  editable.append(island)

  expect(isTypingTarget(keydownOn(island))).toBe(false)
})

it('does not guard a plain div', () => {
  expect(isTypingTarget(keydownOn(mounted(document.createElement('div'))))).toBe(false)
})

it('does not guard an event with no element target', () => {
  expect(isTypingTarget(new KeyboardEvent('keydown', { key: 'i' }))).toBe(false)
})

it('reads both delete keys in the library view', () => {
  const grid = mounted(document.createElement('div'))

  expect(isTrashKey(keydownOn(grid, KEY_DELETE), 'library')).toBe(true)
  expect(isTrashKey(keydownOn(grid, KEY_BACKSPACE), 'library')).toBe(true)
})

it('ignores both delete keys in the trash view', () => {
  const grid = mounted(document.createElement('div'))

  expect(isTrashKey(keydownOn(grid, KEY_DELETE), 'trash')).toBe(false)
  expect(isTrashKey(keydownOn(grid, KEY_BACKSPACE), 'trash')).toBe(false)
})

it('ignores both delete keys while the typing guard is true', () => {
  const field = mounted(document.createElement('input'))

  expect(isTrashKey(keydownOn(field, KEY_DELETE), 'library')).toBe(false)
  expect(isTrashKey(keydownOn(field, KEY_BACKSPACE), 'library')).toBe(false)
})

it('reads a press inside the viewer, which is a native dialog', () => {
  const dialog = mounted(document.createElement('dialog'))
  const inner = dialog.appendChild(document.createElement('div'))

  expect(isInDialog(keydownOn(inner))).toBe(true)
})

it('reads a press inside a shadcn dialog, which is a div with the role', () => {
  const dialog = mounted(document.createElement('div'))
  dialog.setAttribute('role', 'dialog')
  const inner = dialog.appendChild(document.createElement('button'))

  expect(isInDialog(keydownOn(inner))).toBe(true)
})

it('is not in a dialog on the screen behind one', () => {
  expect(isInDialog(keydownOn(mounted(document.createElement('div'))))).toBe(false)
})

it('is not any other key', () => {
  expect(isTrashKey(keydownOn(mounted(document.createElement('div'))), 'library')).toBe(false)
})
