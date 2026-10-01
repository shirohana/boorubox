// @vitest-environment jsdom

import { flushSync, mount, unmount } from 'svelte'
import { afterEach, expect, it, vi } from 'vitest'
import { stamps } from '$lib/api'
import StampBar from './StampBar.svelte'

vi.mock('$lib/api/commands', () => ({ stampsList: vi.fn(async () => []) }))

const mounted: Array<{ target: HTMLElement, instance: object }> = []

afterEach(() => {
  for (const { target, instance } of mounted.splice(0)) {
    unmount(instance)
    target.remove()
  }
  stamps.list = []
})

function setup(initial = '') {
  stamps.list = [{ id: 'cat', name: 'Cat', text: 'cat animal', createdAt: 0, updatedAt: 0 }]
  const target = document.createElement('div')
  document.body.appendChild(target)
  const props = $state({ text: initial })
  const instance = mount(StampBar, {
    target,
    props: {
      get text() { return props.text },
      set text(next: string) { props.text = next },
      selectionCount: 0,
      onapplyselection: () => {},
      error: null,
    },
  })
  mounted.push({ target, instance })
  flushSync()
  const field = () => target.querySelector('input') as HTMLInputElement
  const chip = () => target.querySelector('[aria-pressed]') as HTMLButtonElement
  const clear = () => target.querySelector('[aria-label="Clear stamp"]') as HTMLButtonElement | null
  return { props, field, chip, clear }
}

it('fills the field from a chip and empties it on the second click', () => {
  const { field, chip } = setup()

  chip().click()
  flushSync()
  expect(field().value).toBe('cat animal')
  expect(chip().getAttribute('aria-pressed')).toBe('true')

  chip().click()
  flushSync()
  expect(field().value).toBe('')
  expect(chip().getAttribute('aria-pressed')).toBe('false')
})

it('offers the clear control only while the field holds text, and it empties the field', () => {
  const { props, field, clear } = setup()
  expect(clear()).toBeNull()

  props.text = 'cat -dog'
  flushSync()
  expect(clear()).not.toBeNull()

  clear()!.click()
  flushSync()
  expect(props.text).toBe('')
  expect(field().value).toBe('')
  expect(clear()).toBeNull()
})
