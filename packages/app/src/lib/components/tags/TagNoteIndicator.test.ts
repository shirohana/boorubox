// @vitest-environment jsdom

import { flushSync, mount, unmount } from 'svelte'
import { expect, it } from 'vitest'
import Harness from './TagNoteIndicator.test-harness.svelte'

function setup(note: string | null) {
  const target = document.createElement('div')
  document.body.appendChild(target)
  const instance = mount(Harness, { target, props: { note } })
  flushSync()
  return { target, instance }
}

it('renders no element for a tag with no note', () => {
  const { target, instance } = setup(null)

  expect(target.querySelector('svg')).toBeNull()
  expect(target.querySelector('span')).toBeNull()

  unmount(instance)
})

it('renders the glyph for a tag with a note', () => {
  const { target, instance } = setup('whole background only')

  const glyph = target.querySelector('svg[aria-label="Tag note"]')
  expect(glyph).not.toBeNull()

  unmount(instance)
})

it('keeps the trigger out of the tab order', () => {
  const { target, instance } = setup('whole background only')

  const trigger = target.querySelector('span')
  expect(trigger?.getAttribute('tabindex')).toBe('-1')

  unmount(instance)
})
