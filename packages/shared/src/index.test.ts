import { expect, test } from 'vitest'

test('shared package is importable', async () => {
  expect(await import('./index.js')).toBeDefined()
})
