import { describe, expect, it } from 'vitest'
import type { ArtistDialogStep } from './artist-dialog'
import {
  canConfirm,
  confirmLabel,
  refusalText,
  renamesDerived,
  retryLabel,
  sameName,
  savePlan,
  underscored,
} from './artist-dialog'

describe('underscored', () => {
  it('trims, lowercases and joins whitespace runs with one underscore, as tags::underscored does', () => {
    expect(underscored('  Some  Artist ')).toBe('some_artist')
    expect(underscored('山田\u3000太郎')).toBe('山田_太郎')
  })

  it('keeps a double underscore, as Rust does', () => {
    expect(underscored('a__b')).toBe('a__b')
  })

  it('of a blank text is empty', () => {
    expect(underscored('   ')).toBe('')
  })

  it('reads U+0085 as whitespace, as Rust does', () => {
    expect(underscored('\u0085alice\u0085art\u0085')).toBe('alice_art')
  })

  it('does not read U+FEFF as whitespace, as Rust does not', () => {
    expect(underscored('\uFEFFalice art')).toBe('\uFEFFalice_art')
    expect(underscored('alice\uFEFFart')).toBe('alice\uFEFFart')
  })
})

describe('sameName', () => {
  it('a case-only difference is the same name', () => {
    expect(sameName('Alice', 'alice')).toBe(true)
  })

  it('a space where the tag has an underscore is the same name', () => {
    expect(sameName('alice art', 'alice_art')).toBe(true)
  })

  it('a double underscore is not a single one', () => {
    expect(sameName('alice__art', 'alice_art')).toBe(false)
  })
})

describe('confirmLabel', () => {
  it('edit, same name: "Save"', () => {
    expect(confirmLabel({ mode: 'edit', from: 'alice', name: 'alice', carriers: 12 })).toBe('Save')
  })

  it('edit, a name differing only by spaces/underscores is the same name', () => {
    expect(
      confirmLabel({ mode: 'edit', from: 'alice_art', name: 'alice art', carriers: 12 }),
    ).toBe('Save')
  })

  it('edit, new name: "Rename N images"', () => {
    expect(
      confirmLabel({ mode: 'edit', from: 'metaljelly0811', name: 'metaljelly', carriers: 120 }),
    ).toBe('Rename 120 images')
  })

  it('edit, new name, one carrier: singular', () => {
    expect(confirmLabel({ mode: 'edit', from: 'a', name: 'b', carriers: 1 })).toBe('Rename 1 image')
  })

  it('edit, case-only change: "Save", not a rename', () => {
    expect(confirmLabel({ mode: 'edit', from: 'alice', name: 'Alice', carriers: 12 })).toBe('Save')
  })

  it('create, images known: "Create and tag N images"', () => {
    expect(confirmLabel({ mode: 'create', applyImages: 14, renaming: false, carriers: null })).toBe(
      'Create and tag 14 images',
    )
  })

  it('create, one image: singular', () => {
    expect(confirmLabel({ mode: 'create', applyImages: 1, renaming: false, carriers: null })).toBe(
      'Create and tag 1 image',
    )
  })

  it('create, no images yet: "Create artist"', () => {
    expect(confirmLabel({ mode: 'create', applyImages: 0, renaming: false, carriers: null })).toBe('Create artist')
  })

  it('create, images not yet answered: "Create artist"', () => {
    expect(confirmLabel({ mode: 'create', applyImages: null, renaming: false, carriers: null })).toBe(
      'Create artist',
    )
  })

  it('edit, new name, carriers not yet read: the plain "Rename"', () => {
    expect(confirmLabel({ mode: 'edit', from: 'a', name: 'b', carriers: null })).toBe('Rename')
  })

  it('create that renames, carriers not yet read: "Create artist", never "rename 0 images"', () => {
    expect(confirmLabel({ mode: 'create', applyImages: 14, renaming: true, carriers: null })).toBe(
      'Create artist',
    )
  })

  it('create that renames the derived tag names the rename count', () => {
    expect(confirmLabel({ mode: 'create', applyImages: 14, renaming: true, carriers: 9 })).toBe(
      'Create and rename 9 images',
    )
    expect(confirmLabel({ mode: 'create', applyImages: 14, renaming: true, carriers: 1 })).toBe(
      'Create and rename 1 image',
    )
  })
})

describe('renamesDerived', () => {
  it('only when the derived tag exists and the names differ', () => {
    expect(renamesDerived({ derived: 'alice_art', name: 'alice', derivedExists: true })).toBe(true)
    expect(renamesDerived({ derived: 'alice_art', name: 'alice', derivedExists: false })).toBe(false)
    expect(renamesDerived({ derived: 'alice_art', name: 'Alice Art', derivedExists: true })).toBe(false)
  })
})

describe('canConfirm', () => {
  it('false for an untouched edit', () => {
    expect(
      canConfirm({
        mode: 'edit',
        saving: false,
        previewKnown: true,
        from: 'alice',
        name: 'alice',
        urls: ['https://x.com/alice'],
        initialUrls: ['https://x.com/alice'],
      }),
    ).toBe(false)
  })

  it('true once the name changes', () => {
    expect(
      canConfirm({
        mode: 'edit',
        saving: false,
        previewKnown: true,
        from: 'alice',
        name: 'alice_art',
        urls: ['https://x.com/alice'],
        initialUrls: ['https://x.com/alice'],
      }),
    ).toBe(true)
  })

  it('true once the URLs change, name unchanged', () => {
    expect(
      canConfirm({
        mode: 'edit',
        saving: false,
        previewKnown: true,
        from: 'alice',
        name: 'alice',
        urls: ['https://x.com/alice', 'https://x.com/alice_sub'],
        initialUrls: ['https://x.com/alice'],
      }),
    ).toBe(true)
  })

  it('true for an edit opened with the image\'s own profile appended to the stored URLs', () => {
    expect(
      canConfirm({
        mode: 'edit',
        saving: false,
        previewKnown: true,
        from: 'alice',
        name: 'alice',
        urls: ['https://x.com/alice', 'https://x.com/alice_sub'],
        initialUrls: ['https://x.com/alice'],
      }),
    ).toBe(true)
  })

  it('false for an edit whose name differs only by case, URLs untouched', () => {
    expect(
      canConfirm({
        mode: 'edit',
        saving: false,
        previewKnown: true,
        from: 'alice',
        name: 'Alice',
        urls: ['https://x.com/alice'],
        initialUrls: ['https://x.com/alice'],
      }),
    ).toBe(false)
  })

  it('false while the preview is pending', () => {
    expect(
      canConfirm({
        mode: 'edit',
        saving: false,
        previewKnown: false,
        from: 'alice',
        name: 'alice_art',
        urls: [],
        initialUrls: [],
      }),
    ).toBe(false)
    expect(canConfirm({ mode: 'create', saving: false, previewKnown: false })).toBe(false)
  })

  it('false while saving', () => {
    expect(
      canConfirm({
        mode: 'edit',
        saving: true,
        previewKnown: true,
        from: 'alice',
        name: 'alice_art',
        urls: [],
        initialUrls: [],
      }),
    ).toBe(false)
    expect(canConfirm({ mode: 'create', saving: true, previewKnown: true })).toBe(false)
  })

  it('create is always confirmable once its preview is known and it is not saving', () => {
    expect(canConfirm({ mode: 'create', saving: false, previewKnown: true })).toBe(true)
  })
})

describe('savePlan', () => {
  it('edit, same name, no apply: [upsert]', () => {
    expect(
      savePlan({ mode: 'edit', from: 'alice', name: 'alice', urls: ['https://x.com/alice'], apply: false }),
    ).toEqual([{ kind: 'upsert', entry: { tag: 'alice', urls: ['https://x.com/alice'] } }])
  })

  it('edit, same name, with apply: [upsert, apply(from)]', () => {
    expect(
      savePlan({ mode: 'edit', from: 'alice', name: 'alice', urls: ['https://x.com/alice'], apply: true }),
    ).toEqual([
      { kind: 'upsert', entry: { tag: 'alice', urls: ['https://x.com/alice'] } },
      { kind: 'apply', tag: 'alice' },
    ])
  })

  it('edit, new name, no apply: [rename]', () => {
    expect(
      savePlan({
        mode: 'edit',
        from: 'metaljelly0811',
        name: 'metaljelly',
        urls: ['https://x.com/metaljelly0811'],
        apply: false,
      }),
    ).toEqual([
      {
        kind: 'rename',
        input: { from: 'metaljelly0811', to: 'metaljelly', urls: ['https://x.com/metaljelly0811'] },
      },
    ])
  })

  it('edit, new name, with apply: [rename, apply(to)]', () => {
    expect(
      savePlan({
        mode: 'edit',
        from: 'metaljelly0811',
        name: 'metaljelly',
        urls: ['https://x.com/metaljelly0811'],
        apply: true,
      }),
    ).toEqual([
      {
        kind: 'rename',
        input: { from: 'metaljelly0811', to: 'metaljelly', urls: ['https://x.com/metaljelly0811'] },
      },
      { kind: 'apply', tag: 'metaljelly' },
    ])
  })

  it('create, name matches derived, no existing entry: [upsert, apply]', () => {
    expect(
      savePlan({
        mode: 'create',
        derived: 'alice_art',
        name: 'alice_art',
        urls: ['https://x.com/alice_art'],
        entries: [],
        derivedExists: false,
      }),
    ).toEqual([
      { kind: 'upsert', entry: { tag: 'alice_art', urls: ['https://x.com/alice_art'] } },
      { kind: 'apply', tag: 'alice_art' },
    ])
  })

  it('create under another name unions into that name\'s existing entry', () => {
    const plan = savePlan({
      mode: 'create',
      derived: 'alice_art',
      name: 'alice',
      urls: ['https://x.com/alice_art'],
      entries: [{ tag: 'alice', urls: ['https://www.pixiv.net/users/42'] }],
      derivedExists: false,
    })
    expect(plan[0]).toEqual({
      kind: 'upsert',
      entry: {
        tag: 'alice',
        urls: ['https://www.pixiv.net/users/42', 'https://x.com/alice_art'],
      },
    })
    expect(plan[1]).toEqual({ kind: 'apply', tag: 'alice' })
  })

  it(
    'create under a name that differs from the derived tag, when the derived tag already exists '
    + 'as an artist tag, renames the derived tag onto it instead of leaving it behind',
    () => {
      expect(
        savePlan({
          mode: 'create',
          derived: 'alice_art',
          name: 'alice',
          urls: ['https://x.com/alice_art'],
          entries: [],
          derivedExists: true,
        }),
      ).toEqual([
        { kind: 'rename', input: { from: 'alice_art', to: 'alice', urls: ['https://x.com/alice_art'] } },
        { kind: 'apply', tag: 'alice' },
      ])
    },
  )

  it('create under a name that only differs by spaces/underscores from derived is not a rename', () => {
    expect(
      savePlan({
        mode: 'create',
        derived: 'alice_art',
        name: 'alice art',
        urls: ['https://x.com/alice_art'],
        entries: [],
        derivedExists: true,
      }),
    ).toEqual([
      { kind: 'upsert', entry: { tag: 'alice_art', urls: ['https://x.com/alice_art'] } },
      { kind: 'apply', tag: 'alice_art' },
    ])
  })

  it('create under the derived name itself, derived existing, upserts rather than renames', () => {
    expect(
      savePlan({
        mode: 'create',
        derived: 'alice_art',
        name: 'alice_art',
        urls: ['https://x.com/alice_art'],
        entries: [],
        derivedExists: true,
      }),
    ).toEqual([
      { kind: 'upsert', entry: { tag: 'alice_art', urls: ['https://x.com/alice_art'] } },
      { kind: 'apply', tag: 'alice_art' },
    ])
  })

  it('create under a case-only variant of the derived name, derived existing, is not a rename', () => {
    const plan = savePlan({
      mode: 'create',
      derived: 'alice',
      name: 'Alice',
      urls: ['https://x.com/alice'],
      entries: [],
      derivedExists: true,
    })
    expect(plan).toEqual([
      { kind: 'upsert', entry: { tag: 'alice', urls: ['https://x.com/alice'] } },
      { kind: 'apply', tag: 'alice' },
    ])
  })

  it('edit, case-only change: [upsert] under the stored name, not a rename', () => {
    expect(
      savePlan({ mode: 'edit', from: 'alice', name: 'Alice', urls: ['https://x.com/alice'], apply: false }),
    ).toEqual([{ kind: 'upsert', entry: { tag: 'alice', urls: ['https://x.com/alice'] } }])
  })

  it('edit, new name spelled with spaces and capitals: rename to the underscored spelling', () => {
    expect(
      savePlan({ mode: 'edit', from: 'alice', name: 'Alice Art', urls: [], apply: true }),
    ).toEqual([
      { kind: 'rename', input: { from: 'alice', to: 'alice_art', urls: [] } },
      { kind: 'apply', tag: 'alice_art' },
    ])
  })
})

const rename: ArtistDialogStep = { kind: 'rename', input: { from: 'a', to: 'b', urls: [] } }
const upsert: ArtistDialogStep = { kind: 'upsert', entry: { tag: 'a', urls: [] } }
const apply: ArtistDialogStep = { kind: 'apply', tag: 'b' }

describe('retryLabel', () => {
  it('names only the steps left', () => {
    expect(retryLabel([apply])).toBe('Retry apply')
    expect(retryLabel([rename])).toBe('Retry rename')
    expect(retryLabel([upsert, apply])).toBe('Retry save and apply')
  })
})

describe('refusalText', () => {
  it('names what landed and the step that was refused', () => {
    expect(refusalText({ landed: [rename], failed: apply, reason: 'disk full' })).toBe(
      'Renamed; applying failed: disk full',
    )
    expect(refusalText({ landed: [upsert], failed: apply, reason: 'disk full' })).toBe(
      'Saved; applying failed: disk full',
    )
  })

  it('is the reason alone when nothing landed', () => {
    expect(refusalText({ landed: [], failed: rename, reason: 'bob is a general tag' })).toBe(
      'bob is a general tag',
    )
  })
})
