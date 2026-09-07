import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { DEFAULT_PORT } from '@boorubox/shared'

import { installFakeChrome, type FakeChrome } from './test-support/chrome.js'
import {
  capturesEndpoint,
  getPort,
  pendingEndpoint,
  PORT_KEY,
  setPort,
  statusEndpoint,
} from './settings.js'

let chrome: FakeChrome

beforeEach(() => {
  chrome = installFakeChrome()
})

afterEach(() => {
  vi.unstubAllGlobals()
})

it('falls back to the port the app defaults to when nothing is stored', async () => {
  expect(await getPort()).toBe(DEFAULT_PORT)
})

it('reads a stored override', async () => {
  await setPort(51234)

  expect(await getPort()).toBe(51234)
  expect(chrome.storage.local.data.get(PORT_KEY)).toBe(51234)
})

it('refuses a value the app could never be listening on', async () => {
  await expect(setPort(0)).rejects.toThrow('not a port')
  await expect(setPort(70000)).rejects.toThrow('not a port')
  await expect(setPort(8080.5)).rejects.toThrow('not a port')
  expect(chrome.storage.local.set).not.toHaveBeenCalled()
})

it('ignores a stored value that is out of range rather than using it', async () => {
  chrome.storage.local.data.set(PORT_KEY, 99999)

  expect(await getPort()).toBe(DEFAULT_PORT)
})

it('addresses the app on loopback only', () => {
  expect(capturesEndpoint(47201)).toBe('http://127.0.0.1:47201/captures')
  expect(statusEndpoint(51234)).toBe('http://127.0.0.1:51234/status')
})

it('announces under the captures path and withdraws under the id', () => {
  expect(pendingEndpoint(47201)).toBe('http://127.0.0.1:47201/captures/pending')
  expect(pendingEndpoint(47201, 'id-1')).toBe('http://127.0.0.1:47201/captures/pending/id-1')
})
