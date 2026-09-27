// @vitest-environment jsdom

import { clearMocks, mockIPC } from '@tauri-apps/api/mocks'
import { afterEach, expect, it } from 'vitest'
import { openDanbooruLookup } from './danbooru-open'

const lookup = { label: 'Open Danbooru wiki', url: 'https://danbooru.donmai.us/wiki_pages/solo' }

afterEach(() => {
  clearMocks()
})

it('sends the lookup\'s URL to the opener plugin', async () => {
  let opened: string | undefined
  mockIPC((cmd, args) => {
    if (cmd === 'plugin:opener|open_url') {
      opened = (args as { url: string }).url
      return null
    }
    throw new Error(`unexpected command ${cmd}`)
  })

  openDanbooruLookup(lookup)
  await new Promise((resolve) => setTimeout(resolve, 0))

  expect(opened).toBe(lookup.url)
})

it('drops a refused open rather than throwing or leaving a rejected promise', async () => {
  let reached = false
  mockIPC((cmd) => {
    if (cmd === 'plugin:opener|open_url') {
      reached = true
      throw new Error('refused')
    }
    throw new Error(`unexpected command ${cmd}`)
  })

  expect(() => openDanbooruLookup(lookup)).not.toThrow()

  // Lets the caught rejection settle inside `openExternal` before the test ends,
  // so a regression that stops swallowing it surfaces as an unhandled rejection here.
  await new Promise((resolve) => setTimeout(resolve, 0))

  expect(reached).toBe(true)
})
