// @vitest-environment jsdom

import { expect, it } from 'vitest'
import { isTypingTarget } from './keyboard'

/** The guard reads `event.target`, so every case has to dispatch a real event. */
function keydownOn(element: Element): KeyboardEvent {
  const event = new KeyboardEvent('keydown', { key: 'i', bubbles: true })
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
