// @vitest-environment jsdom

import type { Snippet } from 'svelte'
import { flushSync, mount, unmount } from 'svelte'
import { expect, it, vi } from 'vitest'
import FilterRow from './FilterRow.svelte'
import Harness from './TooltipHarness.svelte'

// No menu content is exercised by these tests, which never open the row's
// context menu — a snippet that renders nothing stands in for it.
const noMenu = (() => {}) as unknown as Snippet

function setup(lookup?: { label: string, open: () => void }) {
  const target = document.createElement('ul')
  document.body.appendChild(target)
  const oninclude = vi.fn()
  const onexclude = vi.fn()
  const ontoggle = vi.fn()
  const instance = mount(FilterRow, {
    target,
    props: {
      name: 'solo',
      count: 3,
      mark: 'none',
      lookup,
      oninclude,
      onexclude,
      ontoggle,
      menu: noMenu,
    },
  })
  flushSync()
  return { target, instance, oninclude, onexclude, ontoggle }
}

it('draws no `?` without a lookup, only the include and exclude buttons before the name', () => {
  const { target, instance } = setup()

  const buttons = target.querySelectorAll('button')
  expect(buttons.length).toBe(3)
  expect([...buttons].some((button) => button.textContent?.trim() === '?')).toBe(false)
  expect(buttons[0].getAttribute('aria-label')).toBe('Include solo')
  expect(buttons[1].getAttribute('aria-label')).toBe('Exclude solo')

  unmount(instance)
})

it('draws the `?` first when a lookup is passed, and it alone opens the look-up', () => {
  const open = vi.fn()
  const { target, instance, oninclude, onexclude, ontoggle } = setup({
    label: 'Open Danbooru wiki',
    open,
  })

  const buttons = target.querySelectorAll('button')
  expect(buttons.length).toBe(4)
  const lookupButton = buttons[0]
  expect(lookupButton.textContent?.trim()).toBe('?')
  expect(lookupButton.getAttribute('title')).toBe('Open Danbooru wiki')
  expect(lookupButton.getAttribute('aria-label')).toBe('Open Danbooru wiki: solo')

  lookupButton.click()

  expect(open).toHaveBeenCalledOnce()
  expect(oninclude).not.toHaveBeenCalled()
  expect(onexclude).not.toHaveBeenCalled()
  expect(ontoggle).not.toHaveBeenCalled()

  unmount(instance)
})

it('draws the name, the pinned dot, the note glyph, then the count, and no dot when unpinned', () => {
  const mountRow = (pinnedLabel: string | null) => {
    const target = document.createElement('ul')
    document.body.appendChild(target)
    const instance = mount(Harness, {
      target,
      props: {
        component: FilterRow,
        props: {
          name: 'solo',
          count: 3,
          mark: 'none',
          note: 'a note',
          pinnedLabel,
          oninclude: vi.fn(),
          onexclude: vi.fn(),
          ontoggle: vi.fn(),
          menu: noMenu,
        },
      },
    })
    flushSync()
    return { target, instance }
  }
  const before = (a: Element, b: Element) => !!(a.compareDocumentPosition(b) & Node.DOCUMENT_POSITION_FOLLOWING)

  const pinned = mountRow('Scene')
  const name = [...pinned.target.querySelectorAll('button')].find((button) => button.querySelector('span.truncate')?.textContent === 'solo')
  const dot = pinned.target.querySelector('span[aria-hidden="true"].rounded-full')
  const glyph = pinned.target.querySelector('svg[aria-label="Tag note"]')
  const count = [...pinned.target.querySelectorAll('span')].find((span) => span.textContent === '3')
  expect(dot).not.toBeNull()
  expect(glyph).not.toBeNull()
  expect(before(name!, dot!)).toBe(true)
  // Inside the name button, right after its text: outside it the stretched button pushes the dot to the row's end.
  expect(name!.contains(dot!)).toBe(true)
  expect(before(dot!, glyph!)).toBe(true)
  expect(before(glyph!, count!)).toBe(true)
  unmount(pinned.instance)

  const unpinned = mountRow(null)
  expect(unpinned.target.querySelector('.rounded-full')).toBeNull()
  unmount(unpinned.instance)
})
