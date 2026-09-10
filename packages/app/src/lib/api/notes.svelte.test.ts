// @vitest-environment jsdom

import { clearMocks, mockIPC } from '@tauri-apps/api/mocks'
import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { NOTE_DEBOUNCE_MS, Notes } from './notes.svelte'

beforeEach(() => {
  vi.useFakeTimers()
})

afterEach(() => {
  vi.useRealTimers()
  clearMocks()
})

/** Records what reached the IPC boundary; `note_get` answers with `content`. */
function spyIPC(content = '') {
  const calls = vi.fn()
  mockIPC((cmd, args) => {
    calls(cmd, args)
    return { content: cmd === 'note_get' ? content : (args as { content: string }).content,
      updatedAt: 1 }
  })
  return calls
}

it('reads the open library’s note', async () => {
  spyIPC('what the library is for')

  const notes = new Notes()
  await notes.load('/libraries/one')

  expect(notes.content).toBe('what the library is for')
})

it('writes once for a burst of keystrokes, after the pause', async () => {
  const calls = spyIPC()
  const notes = new Notes()

  notes.edit('a')
  notes.edit('ab')
  notes.edit('abc')
  expect(calls).not.toHaveBeenCalled()

  await vi.advanceTimersByTimeAsync(NOTE_DEBOUNCE_MS)

  expect(calls).toHaveBeenCalledExactlyOnceWith('note_set', { content: 'abc' })
})

it('writes what is owed when the panel is left before the pause', async () => {
  const calls = spyIPC()
  const notes = new Notes()

  notes.edit('typed and gone')
  await notes.flush()

  expect(calls).toHaveBeenCalledExactlyOnceWith('note_set', { content: 'typed and gone' })

  // The timer must not fire a second write for text Rust already has.
  await vi.advanceTimersByTimeAsync(NOTE_DEBOUNCE_MS)
  expect(calls).toHaveBeenCalledTimes(1)
})

it('does not carry one library’s unwritten text into the next', async () => {
  const calls = spyIPC('the second library’s note')
  const notes = new Notes()
  await notes.load('/libraries/one')
  notes.edit('the first library’s note')

  await notes.load('/libraries/two')
  await notes.flush()

  expect(calls).not.toHaveBeenCalledWith('note_set', expect.anything())
  expect(notes.content).toBe('the second library’s note')
})

it('keeps what is being typed when the same library is loaded again', async () => {
  const calls = spyIPC('what is on disk')
  const notes = new Notes()
  await notes.load('/libraries/one')
  notes.edit('half a senten')

  // What the panel's effect does on every capture: the library has not changed,
  // so re-reading would overwrite the text mid-sentence and drop the write the
  // debounce still owes.
  await notes.load('/libraries/one')

  expect(notes.content).toBe('half a senten')
  await vi.advanceTimersByTimeAsync(NOTE_DEBOUNCE_MS)
  expect(calls).toHaveBeenLastCalledWith('note_set', { content: 'half a senten' })
})

it('keeps the text and reports the reason when a write fails', async () => {
  mockIPC(() => {
    throw 'no library is open'
  })
  const notes = new Notes()

  notes.edit('kept')
  await vi.advanceTimersByTimeAsync(NOTE_DEBOUNCE_MS)

  expect(notes.content).toBe('kept')
  expect(notes.error).toBe('no library is open')
})
