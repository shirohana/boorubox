// @vitest-environment jsdom

import { describe, expect, it } from 'vitest'
import { isOrphanedFocus } from './focus-handback'

describe('isOrphanedFocus', () => {
  it('is orphaned for null', () => {
    expect(isOrphanedFocus(null)).toBe(true)
  })

  it('is orphaned for document.body', () => {
    expect(isOrphanedFocus(document.body)).toBe(true)
  })

  it('is orphaned for an element inside `within`', () => {
    const menu = document.createElement('div')
    const item = document.createElement('button')
    menu.append(item)
    document.body.append(menu)

    expect(isOrphanedFocus(item, menu)).toBe(true)
  })

  it('is not orphaned for a button elsewhere', () => {
    const menu = document.createElement('div')
    const button = document.createElement('button')
    document.body.append(menu, button)

    expect(isOrphanedFocus(button, menu)).toBe(false)
  })
})
