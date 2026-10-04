// @vitest-environment jsdom

import { vocabulary } from '$lib/api'
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks'
import { flushSync, mount, tick, unmount } from 'svelte'
import { afterEach, expect, it } from 'vitest'
import PinnedTagsDialog from './PinnedTagsDialog.svelte'

afterEach(() => {
  clearMocks()
  vocabulary.entries = []
  vocabulary.groups = []
  document.body.innerHTML = ''
})

it('opens on the groups region, not on a field', async () => {
  vocabulary.entries = [{ name: 'cat', category: 'general', pinnedGroup: 1, note: null }]
  vocabulary.groups = [{ name: 'Animals', collapsed: false }]
  mockIPC((cmd) => (cmd === 'tag_suggestions' ? [] : { tags: vocabulary.entries, groups: vocabulary.groups }))
  const instance = mount(PinnedTagsDialog, { target: document.body, props: { open: true, onclose: () => {} } })
  flushSync()
  for (let i = 0; i < 4; i++) await tick()
  await new Promise((resolve) => setTimeout(resolve, 0))

  expect(document.activeElement).toBe(document.body.querySelector('[data-groups]'))
  unmount(instance)
})

it('mounts the panel editing, because the dialog is opened to manage', async () => {
  vocabulary.entries = [{ name: 'cat', category: 'general', pinnedGroup: 1, note: null }]
  vocabulary.groups = [{ name: 'Animals', collapsed: false }]
  mockIPC((cmd) => (cmd === 'tag_suggestions' ? [] : { tags: vocabulary.entries, groups: vocabulary.groups }))
  const instance = mount(PinnedTagsDialog, { target: document.body, props: { open: true, onclose: () => {} } })
  flushSync()
  for (let i = 0; i < 4; i++) await tick()

  expect(document.body.querySelector('[aria-label="Select cat"]')).not.toBeNull()
  expect(document.body.querySelector('[aria-pressed="true"]')?.textContent).toContain('Edit')
  unmount(instance)
})
