// @vitest-environment jsdom

import type { AppSettings } from '@boorubox/shared'
import { GRID_TILE_DEFAULT, GRID_TILE_MAX } from '@boorubox/shared'
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks'
import { afterEach, expect, it, vi } from 'vitest'
import { settings } from './settings.svelte'

afterEach(() => {
  clearMocks()
})

const stored: AppSettings = {
  theme: 'system',
  gridTileSize: GRID_TILE_DEFAULT,
  notesCollapsed: false,
  openLastOnLaunch: true,
}

it('asks Rust once and keeps the answer', async () => {
  const calls = vi.fn()
  mockIPC((cmd) => {
    calls(cmd)
    return stored
  })

  await settings.load()
  await settings.load()

  expect(calls).toHaveBeenCalledTimes(1)
  expect(settings.current?.theme).toBe('system')
})

it('keeps what the write answered, not what it was asked for', async () => {
  mockIPC(
    () =>
      ({
        theme: 'dark',
        gridTileSize: GRID_TILE_MAX,
        notesCollapsed: true,
        openLastOnLaunch: true,
      }) satisfies AppSettings,
  )

  await settings.setTheme('dark')
  expect(settings.current?.theme).toBe('dark')

  // Rust clamps, so a size out of range comes back as the bound it was clamped
  // to; taking the argument instead would show a value the file does not hold.
  await settings.setGridTileSize(GRID_TILE_MAX + 100)
  expect(settings.current?.gridTileSize).toBe(GRID_TILE_MAX)

  await settings.setNotesCollapsed(true)
  expect(settings.current?.notesCollapsed).toBe(true)

  await settings.setOpenLastOnLaunch(false)
  expect(settings.current?.openLastOnLaunch).toBe(true)
})
