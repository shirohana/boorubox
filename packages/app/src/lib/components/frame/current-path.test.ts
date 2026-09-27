import { expect, it } from 'vitest'
import { isCurrentPath } from './current-path'

it('is current on an exact match', () => {
  expect(isCurrentPath('/settings', '/settings')).toBe(true)
})

it('is current on a page under it', () => {
  expect(isCurrentPath('/settings/artists', '/settings')).toBe(true)
})

it('is not current on a sibling sharing the prefix', () => {
  expect(isCurrentPath('/settingsx', '/settings')).toBe(false)
})

it('does not let / claim another screen', () => {
  expect(isCurrentPath('/settings', '/')).toBe(false)
})

it('is current for / on /', () => {
  expect(isCurrentPath('/', '/')).toBe(true)
})
