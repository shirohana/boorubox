// @vitest-environment jsdom

import type { ImageRecord } from '@boorubox/shared'
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks'
import { flushSync, mount, unmount } from 'svelte'
import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { img } from '$lib/domain/image-fixture'
import ImageCard from './ImageCard.svelte'

beforeEach(() => {
  mockIPC(() => null)
})

afterEach(() => {
  clearMocks()
  document.body.innerHTML = ''
})

function tile(image: ImageRecord) {
  const target = document.createElement('div')
  document.body.appendChild(target)
  const instance = mount(ImageCard, {
    target,
    props: {
      image,
      focused: false,
      selected: false,
      onfocus: () => {},
      onselect: () => {},
      ontoggle: () => {},
      onrate: () => {},
      view: 'library' as const,
      actions: { trash: vi.fn(), restore: vi.fn(), deleteForever: vi.fn() },
      selection: { ids: [], all: false } as never,
      onwritten: () => {},
      onnewcollection: () => {},
      onerror: () => {},
    },
  })
  flushSync()
  return { target, instance }
}

it('shows the duration on a video tile', () => {
  const { target, instance } = tile(img({ mime: 'video/mp4', ext: 'mp4', durationMs: 8033 }))
  expect(target.textContent).toContain('0:08')
  unmount(instance)
})

it('shows no duration on an image tile', () => {
  const { target, instance } = tile(img())
  expect(target.textContent).not.toMatch(/\d:\d\d/)
  unmount(instance)
})
