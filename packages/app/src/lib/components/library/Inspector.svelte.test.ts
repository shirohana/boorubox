// @vitest-environment jsdom

import type { ArtistMatch, ImageRecord } from '@boorubox/shared'
import type { SearchResults } from '$lib/api'
import { artistRevision } from '$lib/api'
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
function setup(
  image: ImageRecord,
  options: {
    ipc?: (cmd: string, payload: unknown) => unknown
    onrelease?: () => void
    onquery?: (next: string, id: string | undefined) => void
  } = {},
) {
  mockIPC(options.ipc ?? ((cmd) => (cmd === 'tag_suggestions' ? [] : null)))
  const target = document.createElement('div')
  document.body.appendChild(target)
  const props = $state({
    image: image as ImageRecord | null,
    results: stubResults(),
    actions: stubActions(),
    tagQuery: '',
    onquery: options.onquery ?? (() => {}),
    onrelease: options.onrelease,
    onartistsaved: () => {},
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

it('hands the focus back when an image change closes the editor that held it', () => {
  const onrelease = vi.fn()
  const { target, props, instance } = setup(img({ id: 'a' }), { onrelease })

  instance.startEditTags()
  flushSync()
  const cancel = [...target.querySelectorAll<HTMLButtonElement>('button')]
    .find((button) => button.textContent?.trim() === 'Cancel')!
  cancel.focus()
  expect(document.activeElement).toBe(cancel)

  props.image = img({ id: 'b' })
  flushSync()

  expect(tagTextarea(target)).toBeNull()
  expect(onrelease).toHaveBeenCalledTimes(1)

  unmount(instance)
})

it('does not hand the focus back when the panel loses its image', () => {
  const onrelease = vi.fn()
  const { target, props, instance } = setup(img({ id: 'a' }), { onrelease })

  instance.startEditTags()
  flushSync()
  const cancel = [...target.querySelectorAll<HTMLButtonElement>('button')]
    .find((button) => button.textContent?.trim() === 'Cancel')!
  cancel.focus()

  props.image = null
  flushSync()

  expect(onrelease).not.toHaveBeenCalled()

  unmount(instance)
})

it('does not hand the focus back on the first run, with an editor opened right after mount', () => {
  const onrelease = vi.fn()
  const { target, instance } = setup(img({ id: 'a' }), { onrelease })

  instance.startEditTags()
  flushSync()
  const cancel = [...target.querySelectorAll<HTMLButtonElement>('button')]
    .find((button) => button.textContent?.trim() === 'Cancel')!
  cancel.focus()
  flushSync()

  expect(tagTextarea(target)).not.toBeNull()
  expect(onrelease).not.toHaveBeenCalled()

  unmount(instance)
})

it('leaves the focus alone when the editor did not hold it', () => {
  const onrelease = vi.fn()
  const { props, instance } = setup(img({ id: 'a' }), { onrelease })
  const elsewhere = document.createElement('button')
  document.body.appendChild(elsewhere)

  instance.startEditTags()
  flushSync()
  elsewhere.focus()

  props.image = img({ id: 'b' })
  flushSync()

  expect(onrelease).not.toHaveBeenCalled()

  elsewhere.remove()
  unmount(instance)
})

function xImage(id: string, handle: string): ImageRecord {
  return img({ id, adapter: { site: 'x', fields: { handle } } })
}

function matchOf(handle: string, owner: string | null): ArtistMatch {
  return { url: `https://x.com/${handle}`, owner, derived: handle }
}

/** `artist_match` answers held open per handle, so a test decides the order they land in. */
function heldArtistMatches() {
  const held = new Map<string, (match: ArtistMatch | null) => void>()
  const calls: string[] = []
  const ipc = (cmd: string, payload: unknown) => {
    if (cmd !== 'artist_match') return null
    const handle = (payload as { adapter: { fields: { handle: string } } }).adapter.fields.handle
    calls.push(handle)
    return new Promise<ArtistMatch | null>((resolve) => held.set(handle, resolve))
  }
  return { held, calls, ipc }
}

async function settle() {
  for (let i = 0; i < 5; i++) await tick()
  flushSync()
}

function artistOwnerButton(target: HTMLElement, owner: string) {
  return [...target.querySelectorAll<HTMLButtonElement>('button')]
    .find((button) => button.textContent?.trim() === owner)
}

it('the Artist row drops an answer for an image it has moved past', async () => {
  const { held, ipc } = heldArtistMatches()
  const { target, props, instance } = setup(xImage('a', 'alice_x'), { ipc })
  flushSync()
  await settle()

  props.image = xImage('b', 'bob_x')
  flushSync()
  await settle()

  held.get('bob_x')!(matchOf('bob_x', 'bob'))
  await settle()
  held.get('alice_x')!(matchOf('alice_x', 'alice'))
  await settle()

  expect(artistOwnerButton(target, 'bob')).toBeDefined()
  expect(artistOwnerButton(target, 'alice')).toBeUndefined()

  unmount(instance)
})

it('the Artist row\'s owner toggles the owner\'s tag in the query', async () => {
  const { held, ipc } = heldArtistMatches()
  const onquery = vi.fn()
  const { target, instance } = setup(xImage('a', 'alice_x'), { ipc, onquery })
  flushSync()
  await settle()

  held.get('alice_x')!(matchOf('alice_x', 'alice'))
  await settle()

  artistOwnerButton(target, 'alice')!.click()
  expect(onquery).toHaveBeenCalledWith('alice', 'a')

  unmount(instance)
})

it('the Artist row asks by page URL for an image stored with no record, and offers Create artist…', async () => {
  const payloads: unknown[] = []
  const ipc = (cmd: string, payload: unknown) => {
    if (cmd !== 'artist_match') return null
    payloads.push(payload)
    return { url: 'https://x.com/alice_art', owner: null, derived: 'alice_art' } satisfies ArtistMatch
  }
  const pageUrl = 'https://x.com/Alice_Art/status/1/photo/1'
  const { target, instance } = setup(img({ id: 'legacy', adapter: undefined, pageUrl }), { ipc })
  flushSync()
  await settle()

  expect(payloads).toEqual([{ adapter: null, pageUrl }])
  expect(artistOwnerButton(target, 'Create artist…')).toBeDefined()

  unmount(instance)
})

it('the Artist row re-reads when an artist entry is written elsewhere', async () => {
  const { held, calls, ipc } = heldArtistMatches()
  const { target, instance } = setup(xImage('a', 'alice_x'), { ipc })
  flushSync()
  await settle()
  held.get('alice_x')!(matchOf('alice_x', null))
  await settle()
  expect(calls).toEqual(['alice_x'])

  artistRevision.bump()
  flushSync()
  await settle()
  held.get('alice_x')!(matchOf('alice_x', 'alice'))
  await settle()

  expect(calls).toEqual(['alice_x', 'alice_x'])
  expect(artistOwnerButton(target, 'alice')).toBeDefined()

  unmount(instance)
})
