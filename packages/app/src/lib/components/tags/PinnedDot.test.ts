// @vitest-environment jsdom

import { flushSync, mount, unmount } from 'svelte'
import { expect, it } from 'vitest'
import PinnedDot from './PinnedDot.svelte'
import Harness from './TooltipHarness.svelte'

function setup(group: string | null) {
  const target = document.createElement('div')
  document.body.appendChild(target)
  const instance = mount(Harness, { target, props: { component: PinnedDot, props: { group } } })
  flushSync()
  return { target, instance }
}

it('renders nothing for a tag that is not pinned', () => {
  const { target, instance } = setup(null)

  expect(target.children.length).toBe(0)

  unmount(instance)
})

it('renders the disc and the hidden "Pinned in" text for a group label', () => {
  const { target, instance } = setup('Scene')

  const disc = target.querySelector('span[aria-hidden="true"]')
  expect(disc?.classList.contains('rounded-full')).toBe(true)
  expect(target.querySelector('.sr-only')?.textContent).toBe('Pinned in Scene')

  unmount(instance)
})
