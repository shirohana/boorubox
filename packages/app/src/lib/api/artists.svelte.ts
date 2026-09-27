// Which artist entries are current, as a number (`artist-workflow` design
// D5): every writer of an entry — the artist dialog's save, Settings →
// Artists' form, delete and Apply — bumps it, and the inspector's Artist row
// re-reads on it. One shared counter rather than a counter per host: the row
// cannot know which of several dialogs, or the settings page, changed the
// entry that owns its image's profile URL.

export class ArtistRevision {
  current = $state(0)

  bump = (): void => {
    this.current++
  }
}

export const artistRevision = new ArtistRevision()
