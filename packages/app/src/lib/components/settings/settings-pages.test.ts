import { afterEach, expect, it, vi } from 'vitest'
import { lastSettingsPage, rememberSettingsPage, SETTINGS_PAGES, settingsPageFor } from './settings-pages'

afterEach(() => {
  vi.unstubAllGlobals()
})

function mapStorage(): Storage {
  const map = new Map<string, string>()
  return {
    getItem: (key) => map.get(key) ?? null,
    setItem: (key, value) => void map.set(key, value),
  } as Storage
}

function throwingStorage(): Storage {
  return {
    getItem: () => {
      throw new Error('disabled')
    },
    setItem: () => {
      throw new Error('disabled')
    },
  } as unknown as Storage
}

it('lists the pages in the owner\'s order', () => {
  expect(SETTINGS_PAGES.map((page) => page.label)).toEqual([
    'General',
    'Library',
    'Artists',
    'Rules',
    'Stamps',
    'Booru',
    'Keyboard',
    'About',
  ])
})

it('names the page a stored slug points at', () => {
  expect(settingsPageFor('artists').slug).toBe('artists')
})

it('answers General for null, empty and unknown slugs', () => {
  expect(settingsPageFor(null).slug).toBe('general')
  expect(settingsPageFor('').slug).toBe('general')
  expect(settingsPageFor('nonexistent').slug).toBe('general')
})

it('round-trips a remembered page', () => {
  vi.stubGlobal('localStorage', mapStorage())
  rememberSettingsPage('stamps')
  expect(lastSettingsPage().slug).toBe('stamps')
})

it('answers General and does not throw when storage is unavailable', () => {
  vi.stubGlobal('localStorage', throwingStorage())
  expect(lastSettingsPage().slug).toBe('general')
  expect(() => rememberSettingsPage('stamps')).not.toThrow()
})
