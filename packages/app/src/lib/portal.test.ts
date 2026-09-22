// @vitest-environment jsdom

import { describe, expect, it } from 'vitest'
import { portalTarget } from './portal'

describe('portalTarget', () => {
  it('answers the nearest dialog for an element inside one', () => {
    const dialog = document.createElement('dialog')
    const field = document.createElement('input')
    dialog.append(field)
    document.body.append(dialog)

    expect(portalTarget(field)).toBe(dialog)
  })

  it('answers undefined for an element outside a dialog', () => {
    const field = document.createElement('input')
    document.body.append(field)

    expect(portalTarget(field)).toBeUndefined()
  })

  it('answers undefined for null', () => {
    expect(portalTarget(null)).toBeUndefined()
  })
})
