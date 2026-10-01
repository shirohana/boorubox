// @vitest-environment jsdom

import type { Snippet } from 'svelte'
import { flushSync, mount, unmount } from 'svelte'
import { expect, it, vi } from 'vitest'
import FilterRow from './FilterRow.svelte'

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

it('draws the pinned dot before the count, and none for an unpinned row', () => {
  const mountRow = (pinned: boolean) => {
    const target = document.createElement('ul')
    document.body.appendChild(target)
    const instance = mount(FilterRow, {
      target,
      props: {
        name: 'solo',
        count: 3,
        mark: 'none',
        pinned,
        oninclude: vi.fn(),
        onexclude: vi.fn(),
        ontoggle: vi.fn(),
        menu: noMenu,
      },
    })
    flushSync()
    return { target, instance }
  }

  const pinned = mountRow(true)
  const dot = pinned.target.querySelector('span[aria-hidden="true"].rounded-full')
  const count = [...pinned.target.querySelectorAll('span')].find((span) => span.textContent === '3')
  expect(dot).not.toBeNull()
  expect(dot!.compareDocumentPosition(count!) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy()
  unmount(pinned.instance)

  const unpinned = mountRow(false)
  expect(unpinned.target.querySelector('.rounded-full')).toBeNull()
  unmount(unpinned.instance)
})
