// @vitest-environment jsdom

import type {
  CaptureMeta,
  CaptureWithdrawn,
  ExportProgress,
  ImageRecord,
  ImportProgress,
} from '@boorubox/shared'
import { emit } from '@tauri-apps/api/event'
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks'
import { afterEach, expect, it, vi } from 'vitest'
import {
  CAPTURE_PENDING_EVENT,
  CAPTURE_STORED_EVENT,
  CAPTURE_WITHDRAWN_EVENT,
  EXPORT_PROGRESS_EVENT,
  IMPORT_PROGRESS_EVENT,
  LIBRARY_OPENED_EVENT,
  onCapturePending,
  onCaptureStored,
  onCaptureWithdrawn,
  onExportProgress,
  onImportProgress,
  onLibraryOpened,
  onRebuildProgress,
  onRulesProgress,
  onSidecarsProgress,
  REBUILD_PROGRESS_EVENT,
  RULES_PROGRESS_EVENT,
  SIDECARS_PROGRESS_EVENT,
} from './events'

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

it('hands the subscriber the capture that was stored', async () => {
  mockIPC(() => {}, { shouldMockEvents: true })
  const handler = vi.fn()
  const image = { id: 'cap-1', pageTitle: 'a page' } as ImageRecord

  await onCaptureStored(handler)
  await emit(CAPTURE_STORED_EVENT, image)

  expect(handler).toHaveBeenCalledWith(image)
})

it('hands the subscriber the capture that was announced', async () => {
  mockIPC(() => {}, { shouldMockEvents: true })
  const handler = vi.fn()
  const meta: CaptureMeta = {
    id: 'cap-1',
    imageUrl: 'https://pbs.example/media/1.jpg',
    pageUrl: 'https://x.com/an/status/1',
    pageTitle: 'a post',
    capturedAt: 1_700_000_000_000,
  }

  await onCapturePending(handler)
  await emit(CAPTURE_PENDING_EVENT, meta)

  expect(handler).toHaveBeenCalledWith(meta)
})

it('hands the subscriber the export progress payload, unwrapped', async () => {
  mockIPC(() => {}, { shouldMockEvents: true })
  const handler = vi.fn()
  const progress: ExportProgress = { done: 2, total: 5 }

  await onExportProgress(handler)
  await emit(EXPORT_PROGRESS_EVENT, progress)

  expect(handler).toHaveBeenCalledWith(progress)
})

it('hands the subscriber the rules-run progress payload, unwrapped', async () => {
  mockIPC(() => {}, { shouldMockEvents: true })
  const handler = vi.fn()
  const progress: ExportProgress = { done: 12, total: 400 }

  await onRulesProgress(handler)
  await emit(RULES_PROGRESS_EVENT, progress)

  expect(handler).toHaveBeenCalledWith(progress)
})

it('hands the subscriber the rebuild progress payload, unwrapped', async () => {
  mockIPC(() => {}, { shouldMockEvents: true })
  const handler = vi.fn()
  const progress: ExportProgress = { done: 100, total: 25_000 }

  await onRebuildProgress(handler)
  await emit(REBUILD_PROGRESS_EVENT, progress)

  expect(handler).toHaveBeenCalledWith(progress)
})

it('hands the subscriber the sidecar backfill progress payload, unwrapped', async () => {
  mockIPC(() => {}, { shouldMockEvents: true })
  const handler = vi.fn()
  const progress: ExportProgress = { done: 12, total: 400 }

  await onSidecarsProgress(handler)
  await emit(SIDECARS_PROGRESS_EVENT, progress)

  expect(handler).toHaveBeenCalledWith(progress)
})

it('hands the subscriber the launch-time open settling, with no payload', async () => {
  mockIPC(() => {}, { shouldMockEvents: true })
  const handler = vi.fn()

  await onLibraryOpened(handler)
  await emit(LIBRARY_OPENED_EVENT)

  expect(handler).toHaveBeenCalledTimes(1)
  expect(handler).toHaveBeenCalledWith()
})

it('hands the subscriber the announcement that was withdrawn, with its reason', async () => {
  mockIPC(() => {}, { shouldMockEvents: true })
  const handler = vi.fn()
  const withdrawn: CaptureWithdrawn = { id: 'cap-1', reason: 'the image could not be decoded' }

  await onCaptureWithdrawn(handler)
  await emit(CAPTURE_WITHDRAWN_EVENT, withdrawn)

  expect(handler).toHaveBeenCalledWith(withdrawn)
})
