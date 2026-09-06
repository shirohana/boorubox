// @vitest-environment jsdom

import type { ImportProgress } from '@boorubox/shared'
import { emit } from '@tauri-apps/api/event'
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks'
import { afterEach, expect, it, vi } from 'vitest'
import { IMPORT_PROGRESS_EVENT, onImportProgress } from './events'

afterEach(() => {
  clearMocks()
})

const progress: ImportProgress = { done: 2, total: 5, imported: 1, skipped: 1, failed: 0 }

it('hands the subscriber the progress payload, unwrapped', async () => {
  mockIPC(() => {}, { shouldMockEvents: true })
  const handler = vi.fn()

  await onImportProgress(handler)
  await emit(IMPORT_PROGRESS_EVENT, progress)

  expect(handler).toHaveBeenCalledWith(progress)
})

// The mock IPC's unlisten leaves the event name registered and only drops the
// callback, so it logs "Couldn't find callback id" on the next emit. The
// handler still must not fire; that is what this asserts.
it('stops delivering once unlistened', async () => {
  mockIPC(() => {}, { shouldMockEvents: true })
  const handler = vi.fn()

  const unlisten = await onImportProgress(handler)
  await unlisten()
  await emit(IMPORT_PROGRESS_EVENT, progress)

  expect(handler).not.toHaveBeenCalled()
})
