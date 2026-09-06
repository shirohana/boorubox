// @vitest-environment jsdom

import type { Theme } from '@boorubox/shared'
import { expect, it } from 'vitest'
import { resolveTheme } from './theme.svelte'

const cases: [Theme, boolean, 'light' | 'dark'][] = [
  ['system', true, 'dark'],
  ['system', false, 'light'],
  ['light', true, 'light'],
  ['light', false, 'light'],
  ['dark', true, 'dark'],
  ['dark', false, 'dark'],
]

it.each(cases)('%s with system dark=%s paints %s', (setting, systemDark, painted) => {
  expect(resolveTheme(setting, systemDark)).toBe(painted)
})
