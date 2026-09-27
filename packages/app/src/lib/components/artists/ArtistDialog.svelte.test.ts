// @vitest-environment jsdom

import type { ArtistDialogRequest } from './artist-dialog'
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks'
import { flushSync, mount, tick, unmount } from 'svelte'
import { afterEach, expect, it, vi } from 'vitest'
import ArtistDialog from './ArtistDialog.svelte'

afterEach(() => {
  clearMocks()
  document.body.innerHTML = ''
})

/**
 * Every command the dialog reaches, answered from `refusals`: a command named
 * there rejects with its text once per queued entry, then answers normally.
 * `calls` keeps the save commands in order, payload included.
 */
function ipc(refusals: Record<string, string[]>) {
  const calls: { cmd: string, payload: unknown }[] = []
  mockIPC((cmd, payload) => {
    if (cmd === 'artist_preview') return { carriers: 3, urls: ['https://x.com/alice'], candidate: null }
    if (cmd === 'artists_apply_preview') return { images: 2, untagged: 1 }
    calls.push({ cmd, payload })
    const refusal = refusals[cmd]?.shift()
    if (refusal !== undefined) throw refusal
    if (cmd === 'rename_artist') return { retagged: 3, merged: false }
    if (cmd === 'artists_apply') return { tagged: 2, skipped: 0 }
    if (cmd === 'artists_upsert') return []
    return null
  })
  return calls
}

async function settle() {
  for (let i = 0; i < 5; i++) await tick()
  flushSync()
}

function open(request: ArtistDialogRequest) {
  const onsaved = vi.fn()
  const onclose = vi.fn()
  const target = document.createElement('div')
  document.body.appendChild(target)
  const instance = mount(ArtistDialog, { target, props: { open: true, request, onsaved, onclose } })
  return { instance, onsaved, onclose }
}

function nameInput() {
  return document.querySelector<HTMLInputElement>('#artist-dialog-name')!
}

function type(field: HTMLInputElement, text: string) {
  field.value = text
  field.dispatchEvent(new Event('input', { bubbles: true }))
  flushSync()
}

function confirmButton() {
  return document.querySelector<HTMLButtonElement>('form button[type="submit"]')!
}

function refusalLine() {
  return document.querySelector('form p.text-destructive')?.textContent?.trim() ?? null
}

async function submit() {
  document.querySelector('form')!.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }))
  await settle()
}

const edit: ArtistDialogRequest = { mode: 'edit', tag: 'alice', adapter: null }

it('after a landed rename, a refused apply keeps the dialog open and the retry runs only the apply', async () => {
  const calls = ipc({ artists_apply: ['disk full'] })
  const { instance, onsaved, onclose } = open(edit)
  await settle()

  type(nameInput(), 'Alice Art')
  document.querySelector<HTMLButtonElement>('#artist-dialog-apply')!.click()
  flushSync()
  await submit()

  expect(calls.map((call) => call.cmd)).toEqual(['rename_artist', 'artists_apply'])
  expect(refusalLine()).toBe('Renamed; applying failed: disk full')
  expect(confirmButton().textContent?.trim()).toBe('Retry apply')
  expect(document.body.textContent).not.toContain('will be retagged')
  expect(nameInput().readOnly).toBe(true)
  expect(onsaved).toHaveBeenCalledTimes(1)
  expect(onclose).not.toHaveBeenCalled()

  await submit()

  expect(calls.map((call) => call.cmd)).toEqual(['rename_artist', 'artists_apply', 'artists_apply'])
  expect(calls[2].payload).toEqual({ tag: 'alice_art' })
  expect(onsaved).toHaveBeenCalledTimes(2)
  expect(onclose).toHaveBeenCalledTimes(1)

  unmount(instance)
})

it('a refused first step keeps nothing, so the retry is planned from the corrected fields', async () => {
  const calls = ipc({ rename_artist: ['bob is a general tag'] })
  const { instance, onsaved, onclose } = open(edit)
  await settle()

  type(nameInput(), 'bob')
  await submit()

  expect(refusalLine()).toBe('bob is a general tag')
  expect(onsaved).not.toHaveBeenCalled()
  expect(onclose).not.toHaveBeenCalled()

  type(nameInput(), 'bobby')
  expect(confirmButton().textContent?.trim()).toBe('Rename 3 images')
  await submit()

  expect(calls.map((call) => call.cmd)).toEqual(['rename_artist', 'rename_artist'])
  expect(calls[1].payload).toEqual({ input: { from: 'alice', to: 'bobby', urls: ['https://x.com/alice'] } })
  expect(onsaved).toHaveBeenCalledTimes(1)
  expect(onclose).toHaveBeenCalledTimes(1)

  unmount(instance)
})
