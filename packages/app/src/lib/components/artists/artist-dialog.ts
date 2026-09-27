// The pure half of `ArtistDialog.svelte` (`artist-workflow` design D4): what
// the confirming button says, what pressing it does, and when it may be
// pressed, each as a plain function over plain values rather than component
// state, so every branch of a save is tested here rather than only by hand.
import type { ArtistEntry, RenameArtistInput } from '@boorubox/shared'

/**
 * What the dialog opens for (`artist-workflow` design D4): an existing tag's own URLs and
 * carrier count (`adapter` is the image's record, `null` when the dialog was
 * opened with no image in scope — the sidebar row and the pinned chip over a
 * selection), or a new artist prefilled from an image's derived name and
 * profile URL. `tag` in create mode is the derived name, not yet a tag.
 */
export type ArtistDialogRequest
  = | { mode: 'edit', tag: string, adapter: import('@boorubox/shared').SiteAdapterRecord | null }
    | { mode: 'create', tag: string, url: string }

/**
 * One call `savePlan` orders; `ArtistDialog.svelte` runs them in sequence,
 * stopping on a refusal.
 */
export type ArtistDialogStep
  = | { kind: 'upsert', entry: ArtistEntry }
    | { kind: 'rename', input: RenameArtistInput }
    | { kind: 'apply', tag: string }

/**
 * Rust's whitespace, spelled out: `char::is_whitespace` is Unicode
 * `White_Space`, which includes U+0085 and excludes U+FEFF. JS `\s` and
 * `trim()` are the reverse on exactly those two, so neither may be used here.
 */
const RUST_WHITESPACE = /[\t-\r \u0085\u00A0\u1680\u2000-\u200A\u2028\u2029\u202F\u205F\u3000]+/

/**
 * A name as Rust stores it: `tags::underscored` — trimmed, lowercased, every
 * whitespace run one `_`. Must stay that function's mirror: Rust answers a
 * rename whose `to` spells the same as `from` with `{ retagged: 0 }` and
 * drops its URLs, so a comparison looser or stricter than Rust's reads a
 * case-only retype as a rename that silently does nothing. `__` is kept, as
 * Rust keeps it. Splitting and dropping the empty ends is the trim.
 */
export function underscored(name: string): string {
  return name.toLowerCase().split(RUST_WHITESPACE).filter((word) => word !== '').join('_')
}

/**
 * Exported for `ArtistDialog.svelte`'s own copy, which names the same
 * comparison in words ("and will be retagged") — one definition of "the same
 * name" rather than a second one drifting in the template.
 */
export function sameName(a: string, b: string): boolean {
  return underscored(a) === underscored(b)
}

function imagesCount(n: number): string {
  return `${n.toLocaleString()} ${n === 1 ? 'image' : 'images'}`
}

/** Order-independent: reordering the same URLs is not an edit worth enabling Save over. */
function sameUrls(a: string[], b: string[]): boolean {
  if (a.length !== b.length) return false
  const sortedA = [...a].sort()
  const sortedB = [...b].sort()
  return sortedA.every((url, index) => url === sortedB[index])
}

export type ConfirmLabelInput
  = | { mode: 'edit', from: string, name: string, carriers: number | null }
    | {
      mode: 'create'
      applyImages: number | null
      /** Whether the plan renames the derived tag ({@link renamesDerived}). */
      renaming: boolean
      /** The derived tag's carriers, `null` until the dialog's preview answers. */
      carriers: number | null
    }

/**
 * What the confirming button reads before a save (`artist-workflow` design
 * D4). Edit: "Save" while the name is unchanged (the URLs may still differ —
 * that is still a save, not a rename), else "Rename N images". Create that
 * renames the derived tag: "Create and rename N images" — the rename is the
 * step that changes existing tags, so it is the number the button names; the
 * apply line above it says how many come from the profile. Any other create:
 * "Create artist" until the apply-preview names how many images that is,
 * then "Create and tag N images". A count not yet read is never shown as 0:
 * the label is the plain verb until it is known.
 */
export function confirmLabel(input: ConfirmLabelInput): string {
  if (input.mode === 'edit') {
    if (sameName(input.name, input.from)) return 'Save'
    return input.carriers === null ? 'Rename' : `Rename ${imagesCount(input.carriers)}`
  }
  if (input.renaming) {
    return input.carriers === null ? 'Create artist' : `Create and rename ${imagesCount(input.carriers)}`
  }
  const n = input.applyImages
  if (n === null || n === 0) return 'Create artist'
  return `Create and tag ${imagesCount(n)}`
}

/**
 * Whether a create renames the derived tag onto the chosen name rather than
 * upserting beside it (`artist-workflow` design D4's create-mode rule):
 * only when the names differ and the derived tag already exists as an
 * artist tag. `savePlan` branches on it and the dialog's rename line and
 * label read it, so the copy never promises a different save than the one
 * that runs.
 */
export function renamesDerived(
  input: { derived: string, name: string, derivedExists: boolean },
): boolean {
  return input.derivedExists && !sameName(input.name, input.derived)
}

export type CanConfirmInput
  = | {
    mode: 'edit'
    saving: boolean
    previewKnown: boolean
    from: string
    name: string
    urls: string[]
    initialUrls: string[]
  }
  | { mode: 'create', saving: boolean, previewKnown: boolean }

/**
 * False while saving, false until the dialog's own preview has answered
 * (`artistPreview` in edit mode, `artistsList` in create mode — the data
 * `savePlan` needs to be correct, not the debounced apply-preview counts,
 * which are advisory only), and, in edit mode only, false while neither the
 * name nor the URL lines differ from what the dialog opened with: a no-op
 * `upsert` would rewrite `library.json` for nothing, and an empty entry is
 * refused anyway. Create mode has no such guard — any save there records a
 * new entry, never a no-op.
 */
export function canConfirm(input: CanConfirmInput): boolean {
  if (input.saving || !input.previewKnown) return false
  if (input.mode === 'create') return true
  return !sameName(input.name, input.from) || !sameUrls(input.urls, input.initialUrls)
}

export type SavePlanInput
  = | { mode: 'edit', from: string, name: string, urls: string[], apply: boolean }
    | {
      mode: 'create'
      derived: string
      name: string
      urls: string[]
      entries: ArtistEntry[]
      /**
     * Whether `derived` already exists as an artist tag — asked of the
     * vocabulary store (`vocabulary.categoryOf(derived) === 'artist'`), not
     * `artist_match` (`artist-workflow` design D8): a capture already tags
     * its author as an artist tag, so this is "has this profile been
     * captured before", not a second existence check duplicating what the
     * store already answers for every other tag-shaped question.
     */
      derivedExists: boolean
    }

/**
 * The ordered steps a save runs (`artist-workflow` design D4). Every name a
 * step carries is {@link underscored}, the spelling Rust stores. Edit, same
 * name: replace the entry's URLs; edit, new name: rename, which moves the old name's own URLs
 * and retags its carriers in one transaction. Create: when the chosen name
 * differs from the derived tag and that derived tag already exists as an
 * artist tag, renaming it onto the chosen name is what retags the handle's
 * own carriers and creates the entry — left otherwise, the handle tag sits
 * beside the chosen name and the owner corrects it by hand, which is exactly
 * what this branch avoids. Any other create unions the typed URLs into an
 * existing entry of that name, rather than replacing it, since `upsert`
 * itself only ever replaces. Apply always follows a create (there is no
 * checkbox for it — the dialog shows the count, not a choice); an edit
 * applies only when `apply` is checked.
 */
export function savePlan(input: SavePlanInput): ArtistDialogStep[] {
  if (input.mode === 'edit') {
    const { from, name, urls, apply } = input
    if (sameName(name, from)) {
      const steps: ArtistDialogStep[] = [{ kind: 'upsert', entry: { tag: from, urls } }]
      if (apply) steps.push({ kind: 'apply', tag: from })
      return steps
    }
    const to = underscored(name)
    const steps: ArtistDialogStep[] = [{ kind: 'rename', input: { from, to, urls } }]
    if (apply) steps.push({ kind: 'apply', tag: to })
    return steps
  }

  const { derived, urls, entries } = input
  const name = underscored(input.name)
  if (renamesDerived(input)) {
    return [
      { kind: 'rename', input: { from: derived, to: name, urls } },
      { kind: 'apply', tag: name },
    ]
  }
  const existing = entries.find((entry) => sameName(entry.tag, name))
  const unionUrls = existing ? Array.from(new Set([...existing.urls, ...urls])) : urls
  return [
    { kind: 'upsert', entry: { tag: name, urls: unionUrls } },
    { kind: 'apply', tag: name },
  ]
}

/** How a step reads in the dialog's own copy: its verb, as done and as under way. */
const STEP_WORDS: Record<ArtistDialogStep['kind'], { verb: string, done: string, doing: string }> = {
  upsert: { verb: 'save', done: 'saved', doing: 'saving' },
  rename: { verb: 'rename', done: 'renamed', doing: 'renaming' },
  apply: { verb: 'apply', done: 'applied', doing: 'applying' },
}

function capitalized(text: string): string {
  return text.charAt(0).toUpperCase() + text.slice(1)
}

/**
 * The confirming button while a refused save has steps left (`artist-workflow`
 * design D4): a retry runs only those, so the button names them — "Retry
 * apply" — rather than the fresh plan's "Rename N images", whose rename has
 * already landed.
 */
export function retryLabel(pending: ArtistDialogStep[]): string {
  return `Retry ${pending.map((step) => STEP_WORDS[step.kind].verb).join(' and ')}`
}

/**
 * The refusal line (`artist-workflow` design D4: the refusal and what
 * landed): "Renamed; applying failed: <reason>" once a step has landed, the
 * reason alone when nothing has — Rust's refusal already names the offender.
 */
export function refusalText(
  input: { landed: ArtistDialogStep[], failed: ArtistDialogStep, reason: string },
): string {
  if (input.landed.length === 0) return input.reason
  const done = capitalized(input.landed.map((step) => STEP_WORDS[step.kind].done).join(' and '))
  return `${done}; ${STEP_WORDS[input.failed.kind].doing} failed: ${input.reason}`
}
