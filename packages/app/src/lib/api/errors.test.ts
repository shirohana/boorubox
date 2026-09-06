import { expect, it } from 'vitest'
import { errorText } from './errors'

it('passes through the string a command rejects with', () => {
  expect(errorText('no library is open')).toBe('no library is open')
})

it('reads the message off an Error', () => {
  expect(errorText(new Error('boom'))).toBe('boom')
})

it('stringifies anything else', () => {
  expect(errorText({ code: 1 })).toBe('[object Object]')
})
