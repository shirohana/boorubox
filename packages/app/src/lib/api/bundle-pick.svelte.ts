// The bundle files picked but not yet confirmed (`import-confirm` design D4):
// a singleton beside `Imports`, not `/import/+page.svelte` state, so a plan
// that took seconds to read off a network drive survives leaving the route
// for settings and coming back, and so the confirm step's real behaviour —
// picking plans and does not enqueue, confirming enqueues exactly what was
// planned, discarding enqueues nothing — is provable in vitest rather than
// only by hand.

import type { BundlePlan } from '@boorubox/shared'
import { bundlePlan } from './commands'
import { errorText } from './errors'
import { imports } from './imports.svelte'

export class BundlePick {
  /** The files last picked, read back into {@link plan}. Empty before a pick. */
  files = $state<string[]>([])
  /** What `bundle_plan` answered for {@link files}; `null` until it has. */
  plan = $state.raw<BundlePlan | null>(null)
  /** Set while a fresh pick's plan is in flight. */
  planning = $state(false)
  error = $state<string | null>(null)

  /**
   * Plans what `picker` returns. A picker the user cancelled answers with no
   * files, which is not an error and plans nothing — the same rule
   * `Imports#pick` uses for the run pickers.
   */
  async pick(picker: () => Promise<string[]>): Promise<void> {
    let picked: string[]
    try {
      picked = await picker()
    } catch (cause) {
      this.error = errorText(cause)
      return
    }
    if (picked.length === 0) return
    this.files = picked
    this.plan = null
    this.error = null
    this.planning = true
    try {
      this.plan = await bundlePlan(picked)
    } catch (cause) {
      this.error = errorText(cause)
    } finally {
      this.planning = false
    }
  }

  /**
   * Starts the run over exactly the parts planned, in the order planned
   * (spec `legacy-bundle-import`, "Confirming the pick"), then clears the
   * pick. A no-op with nothing planned.
   */
  confirm(): void {
    if (!this.plan) return
    imports.enqueueBundle(this.plan.parts.map((part) => part.path))
    this.#clear()
  }

  /** Clears the pick without enqueueing anything. */
  discard(): void {
    this.#clear()
  }

  #clear(): void {
    this.files = []
    this.plan = null
    this.planning = false
    this.error = null
  }
}

export const bundlePick = new BundlePick()
