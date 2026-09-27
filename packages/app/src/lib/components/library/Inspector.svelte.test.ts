// @vitest-environment jsdom

import type { ImageRecord } from '@boorubox/shared'
import type { SearchResults } from '$lib/api'
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks'
import { flushSync, mount, tick, unmount } from 'svelte'
import { afterEach, expect, it, vi } from 'vitest'
import { img } from '$lib/domain/image-fixture'
import Inspector from './Inspector.svelte'
import type { TrashActions } from './trash-actions'

afterEach(() => {
  clearMocks()
})

function stubResults(): SearchResults {
  return {
    view: 'library',
    generation: 0,
    saveTags: vi.fn(async (id: string, tags: string[]) => img({ id, tags })),
    saveFacts: vi.fn(async (id: string) => img({ id })),
    saveRating: vi.fn(async (id: string) => img({ id })),
    replace: vi.fn(),
    replaceMany: vi.fn(),
  } as unknown as SearchResults
}

function stubActions(): TrashActions {
  return { trash: vi.fn(), restore: vi.fn(), deleteForever: vi.fn() }
}

/**
 * `mockIPC` answers every store read the panel makes at mount with an empty
 * value (`tag-row-and-inspector-fixes` design D7): with no tags and no
 * collections on the fixture image, the only command a test can reach is
 * `tag_suggestions`, from typing into the tag editor.
 */
function setup(image: ImageRecord) {
  mockIPC((cmd) => (cmd === 'tag_suggestions' ? [] : null))
  const target = document.createElement('div')
  document.body.appendChild(target)
  const props = $state({
    image: image as ImageRecord | null,
    results: stubResults(),
    actions: stubActions(),
    tagQuery: '',
    onquery: () => {},
    onartistrenamed: () => {},
  })
  const instance = mount(Inspector, { target, props })
  return { target, props, instance }
}

function tagTextarea(target: HTMLElement) {
  return target.querySelector<HTMLTextAreaElement>('textarea[aria-label="Tags of this image"]')
}

function type(field: HTMLTextAreaElement | HTMLInputElement, text: string) {
  field.value = text
  field.dispatchEvent(new Event('input', { bubbles: true }))
}

it('closes the tag editor and writes nothing when the image changes', () => {
  const imageA = img({ id: 'a' })
  const imageB = img({ id: 'b' })
  const { target, props, instance } = setup(imageA)

  instance.startEditTags()
  flushSync()
  const textarea = tagTextarea(target)
  expect(textarea).not.toBeNull()
  type(textarea!, 'dog')
  flushSync()

  props.image = imageB
  flushSync()

  expect(tagTextarea(target)).toBeNull()
  expect([...target.querySelectorAll('button')].some((button) => button.textContent?.trim() === 'Save')).toBe(false)

  unmount(instance)
})

it('keeps the tag editor open when the same image gets a new record', () => {
  const imageA = img({ id: 'a', updatedAt: 0 })
  const { target, props, instance } = setup(imageA)

  instance.startEditTags()
  flushSync()
  const textarea = tagTextarea(target)!
  type(textarea, 'dog')
  flushSync()

  props.image = { ...imageA, updatedAt: 1 }
  flushSync()

  const stillOpen = tagTextarea(target)
  expect(stillOpen).not.toBeNull()
  expect(stillOpen!.value).toBe('dog')
  expect(props.results.saveTags).not.toHaveBeenCalled()

  unmount(instance)
})

it('keeps an editor opened right after mount', () => {
  const { target, instance } = setup(img({ id: 'a' }))

  // `mount` runs no effects on its own: the reset effect's first run (which
  // only records the mounted image's id, per D6) has not fired yet when this
  // calls `startEditTags()` synchronously. `flushSync()` then runs both the
  // reset effect's first pass and the editor's own updates in one go; without
  // the `sawFirstImageId` guard the reset effect would see `imageId` change
  // from `undefined` and close the editor it just opened.
  instance.startEditTags()
  flushSync()

  expect(tagTextarea(target)).not.toBeNull()

  unmount(instance)
})

it('closes the facts form when the image changes', () => {
  const imageA = img({ id: 'a' })
  const imageB = img({ id: 'b' })
  const { target, props, instance } = setup(imageA)

  target.querySelector<HTMLButtonElement>('button[aria-label="Edit title and addresses"]')!.click()
  flushSync()
  expect(target.querySelector('input[aria-label="Title"]')).not.toBeNull()

  props.image = imageB
  flushSync()

  expect(target.querySelector('input[aria-label="Title"]')).toBeNull()

  unmount(instance)
})

it('the facts heading\'s pencil opens the form', async () => {
  const { target, instance } = setup(img({ id: 'a' }))

  const pencils = target.querySelectorAll<HTMLButtonElement>(
    'button[aria-label="Edit title and addresses"]',
  )
  expect(pencils.length).toBe(2)

  pencils[1].click()
  flushSync()
  await tick()

  expect(
    target.querySelectorAll('button[aria-label="Edit title and addresses"]').length,
  ).toBe(0)
  const titleInput = target.querySelector<HTMLInputElement>('input[aria-label="Title"]')
  expect(titleInput).not.toBeNull()
  expect(document.activeElement).toBe(titleInput)

  unmount(instance)
})
