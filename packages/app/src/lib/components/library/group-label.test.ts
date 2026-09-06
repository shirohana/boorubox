import { describe, expect, it } from 'vitest'
import { groupLabel } from './group-label'

describe('groupLabel', () => {
  it('shows an account key as a handle', () => {
    expect(groupLabel('x-account', 'someone')).toBe('@someone')
  })

  it('shows a duplicates key as its dimensions and size', () => {
    expect(groupLabel('duplicates', '1920x1080-2400000')).toBe('1920 × 1080 · 2.4 MB')
  })

  it('shows a key it cannot read as it came', () => {
    expect(groupLabel('duplicates', 'not-a-key')).toBe('not-a-key')
    expect(groupLabel('none', 'whatever')).toBe('whatever')
  })
})
