import { expect, test } from 'vitest'

test('background entry is importable', async () => {
  expect(await import('./index.js')).toBeDefined()
})
