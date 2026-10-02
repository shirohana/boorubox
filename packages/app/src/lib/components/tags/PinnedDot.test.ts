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

it('renders the disc named "Pinned in" its group, with no absolutely placed text', () => {
  const { target, instance } = setup('Scene')

  const disc = target.querySelector('span[role="img"]')
  expect(disc?.classList.contains('rounded-full')).toBe(true)
  expect(disc?.getAttribute('aria-label')).toBe('Pinned in Scene')
  expect(target.querySelector('.sr-only')).toBeNull()

  unmount(instance)
})
