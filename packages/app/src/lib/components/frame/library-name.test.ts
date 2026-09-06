import { expect, it } from 'vitest'
import { libraryName } from './library-name'

it('is the basename of the folder', () => {
  expect(libraryName('/Users/me/Pictures/booru')).toBe('booru')
  expect(libraryName('C:\\Users\\me\\booru')).toBe('booru')
})

it('ignores a trailing separator', () => {
  expect(libraryName('/Users/me/booru/')).toBe('booru')
})

it('has nothing to show without a library', () => {
  expect(libraryName(null)).toBe('')
})
