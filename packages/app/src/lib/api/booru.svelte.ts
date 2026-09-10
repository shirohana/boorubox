// The library's configured booru sites, read once for the two places that need
// them: the Settings section that edits them (design D13) and the Inspector's
// upload action, which has to know both which sites exist and which of them an
// image is already on (design D12). Two independent readers would each re-read
// on every mount and could disagree about the list a moment after a site was
// added.
//
// It also carries what `BooruSite` deliberately does not: what this session has
// learnt about each site's credential, from the two calls that read one — a
// refusal, or a connection test's answer. `booru_site_list` never touches the
// credential store — that is the whole point of design D7 — and there is no
// side-effect-free command that does. Both live here rather than in the
// Settings section because they are per-site session state, and the section is
// mounted and unmounted around a list that outlives it: state held there
// survives neither a return to the screen nor, worse, a library switch, which
// would show one library's outcome against another's site of the same slug.
//
// FIXME(booru-upload D7): the right shape is a Rust credential probe —
// `Credentials::get` with no network behind it — so the settings list can say
// "unusable" before the user tries anything. Until that command exists, a
// site's credential is unknown here until something is refused.

import type { BooruConnectionTest, BooruSite } from '@boorubox/shared'
import { booruSiteList } from './booru'
import { errorText } from './errors'

export class BooruSites {
  /** Ordered by name, as Rust answers. Empty until the first read lands. */
  sites = $state<BooruSite[]>([])
  /** Why the list could not be read; `null` while it is in step. */
  error = $state<string | null>(null)

  /** Site id → the reason its credential could not be read. */
  #refusals = $state<Record<string, string>>({})
  /** Site id → what its last connection test answered, this session. */
  #tests = $state<Record<string, BooruConnectionTest>>({})
  /**
   * The library path the list was read for. `undefined` before the first read,
   * because "no library open" is a path of its own — the same sentinel `Notes`
   * uses, for the same reason.
   */
  #loadedPath: string | null | undefined = undefined

  /**
   * Reads the list for the library at `path`, once per library. Called from
   * every screen that shows sites, so the second caller costs nothing; a
   * library switch is what makes it read again, and it drops what the previous
   * library's credentials did.
   */
  async load(path: string | null): Promise<void> {
    if (path === this.#loadedPath) return
    this.#loadedPath = path
    this.#refusals = {}
    this.#tests = {}
    await this.reload()
    // A read that failed loaded no library: leave the path unset so the next
    // caller asks again. Recording it would make one failure permanent — every
    // screen would go on saying this library has no sites.
    if (this.error !== null) this.#loadedPath = undefined
  }

  /** Re-reads the list after a save or a removal. */
  async reload(): Promise<void> {
    try {
      this.sites = await booruSiteList()
      this.error = null
    } catch (cause) {
      this.error = errorText(cause)
    }
  }

  site(id: string): BooruSite | undefined {
    return this.sites.find((site) => site.id === id)
  }

  /**
   * Why this site cannot be used, or `null` when nothing has found a problem.
   * `null` is "no refusal seen", not "the credential is there": nothing reads a
   * credential until it is needed.
   */
  refusal(id: string): string | null {
    return this.#refusals[id] ?? null
  }

  /** What the last connection test for this site answered, or `null`. */
  test(id: string): BooruConnectionTest | null {
    return this.#tests[id] ?? null
  }

  /**
   * The credential store refused to hand over this site's key. Remembered for
   * the session so the Settings list keeps saying so, rather than reverting to
   * "not checked" the next time the screen is opened (`booru-sites`: "the site
   * is marked unusable with the reason").
   */
  refused(id: string, reason: string): void {
    this.#refusals = { ...this.#refusals, [id]: reason }
    this.#tests = without(this.#tests, id)
  }

  /** The store answered for this site: whatever it refused before is over. */
  usable(id: string): void {
    if (!(id in this.#refusals)) return
    this.#refusals = without(this.#refusals, id)
  }

  /** A connection test answered, which is the store answering too. */
  tested(id: string, result: BooruConnectionTest): void {
    this.#tests = { ...this.#tests, [id]: result }
    this.usable(id)
  }

  /** The site is gone: what this session learnt about it goes with it. */
  forget(id: string): void {
    this.#refusals = without(this.#refusals, id)
    this.#tests = without(this.#tests, id)
  }
}

function without<T>(record: Record<string, T>, key: string): Record<string, T> {
  if (!(key in record)) return record
  const left = { ...record }
  delete left[key]
  return left
}

export const booruSites = new BooruSites()
