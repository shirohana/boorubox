import { expect, it } from 'vitest'
import { latestOnly } from './latest-only'

function deferred<T>() {
  let resolve!: (value: T) => void
  let reject!: (reason?: unknown) => void
  const promise = new Promise<T>((res, rej) => {
    resolve = res
    reject = rej
  })
  return { promise, resolve, reject }
}

it('a later call resolving first wins; the earlier answer comes back stale', async () => {
  const call = latestOnly<string>()
  const first = deferred<string>()
  const second = deferred<string>()

  const firstAnswer = call(first.promise)
  const secondAnswer = call(second.promise)

  second.resolve('second')
  await expect(secondAnswer).resolves.toEqual({ current: true, value: 'second' })

  first.resolve('first')
  await expect(firstAnswer).resolves.toEqual({ current: false })
})

it('a stale rejection is also answered as not current', async () => {
  const call = latestOnly<string>()
  const first = deferred<string>()
  const second = deferred<string>()

  const firstAnswer = call(first.promise)
  const secondAnswer = call(second.promise)

  second.resolve('second')
  await secondAnswer

  first.reject(new Error('too late'))
  await expect(firstAnswer).resolves.toEqual({ current: false })
})

it('a rejection of the still-current call rejects', async () => {
  const call = latestOnly<string>()
  const only = deferred<string>()

  const answer = call(only.promise)
  only.reject(new Error('boom'))

  await expect(answer).rejects.toThrow('boom')
})
