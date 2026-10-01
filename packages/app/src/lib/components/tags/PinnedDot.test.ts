// @vitest-environment jsdom

import { flushSync, mount, unmount } from 'svelte'
import { expect, it } from 'vitest'
import PinnedDot from './PinnedDot.svelte'

function setup(pinned: boolean) {
  const target = document.createElement('div')
  document.body.appendChild(target)
  const instance = mount(PinnedDot, { target, props: { pinned } })
  flushSync()
  return { target, instance }
}

it('renders nothing for a tag that is not pinned', () => {
  const { target, instance } = setup(false)

  expect(target.children.length).toBe(0)

  unmount(instance)
})

it('renders the disc and the hidden text for a pinned tag', () => {
  const { target, instance } = setup(true)

  const disc = target.querySelector('span[aria-hidden="true"]')
  expect(disc?.classList.contains('rounded-full')).toBe(true)
  expect(target.querySelector('.sr-only')?.textContent).toBe('pinned')

  unmount(instance)
})
