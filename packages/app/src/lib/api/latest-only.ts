// A ticket, as a function (`artist-workflow` design D5): the inspector's
// Artist row and the artist dialog's apply-preview both race a fetch against
// whichever image or edit the user has since moved on to, and both need the
// same answer to "is this still the one that matters" — one function rather
// than a ticket counter copied into each caller.

/** What [`latestOnly`]'s returned function resolves a promise to. */
export type LatestOnly<T> = { current: true, value: T } | { current: false }

/**
 * Each call made through the function this returns races against every later
 * call made through the same `latestOnly()`: only the call still current when
 * its promise settles reaches the caller as `{ current: true, value }` — an
 * earlier call that resolves after a later one started answers
 * `{ current: false }` instead, exactly as if it had never resolved. A
 * rejection is swallowed into `{ current: false }` the same way when the call
 * is stale, and rethrown when it is still the latest — a real failure must
 * still surface, just not from a call nobody is waiting on any more.
 */
export function latestOnly<T>(): (promise: Promise<T>) => Promise<LatestOnly<T>> {
  let ticket = 0
  return async (promise: Promise<T>): Promise<LatestOnly<T>> => {
    const mine = ++ticket
    try {
      const value = await promise
      return mine === ticket ? { current: true, value } : { current: false }
    } catch (error) {
      if (mine === ticket) throw error
      return { current: false }
    }
  }
}
